// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::excessive_precision)]
//! Instrument field-of-view definitions and visibility tests.
//!
//! Parses the IK keywords read by CSPICE `getfov`:
//!
//! ```text
//! INS<id>_FOV_CLASS_SPEC   'CORNERS' | 'ANGLES'
//! INS<id>_FOV_SHAPE        'CIRCLE' | 'ELLIPSE' | 'RECTANGLE' | 'POLYGON'
//! INS<id>_FOV_FRAME
//! INS<id>_BORESIGHT
//! INS<id>_FOV_BOUNDARY[_CORNERS]      (CORNERS class)
//! INS<id>_FOV_REF_VECTOR,
//! INS<id>_FOV_REF_ANGLE,
//! INS<id>_FOV_CROSS_ANGLE             (ANGLES class)
//! ```
//!
//! Point and ray visibility follows CSPICE `fovtrg`/`fovray` with a
//! point target: the target direction is projected into the FOV plane
//! at unit distance along the FOV axis, and the projected point is
//! tested against the cone, ellipse, or polygon. Ellipsoid targets use
//! [`ellipsoid_in_fov`], which matches `fovtrg` with `TSHAPE =
//! 'ELLIPSOID'`.

use std::collections::HashMap;

use crate::error::GeoError;
use crate::finder::{normalize_windows, solve_boolean, FinderOptions, Interval};
use crate::frame::{
    apply_assignments, extract_data_blocks, parse_f64, transpose, Frames, Mat3, Value,
};
use crate::pck::Pck;
use crate::spk::{Aberration, SpkKernel};

/// The geometric shape of an instrument FOV.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FovShape {
    /// Circular cone about the boresight.
    Circle,
    /// Elliptical cone about the boresight.
    Ellipse,
    /// Rectangular pyramid; treated as a polygon.
    Rectangle,
    /// General pyramid with a polygonal cross-section.
    Polygon,
}

impl FovShape {
    fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "CIRCLE" => Some(Self::Circle),
            "ELLIPSE" => Some(Self::Ellipse),
            "RECTANGLE" => Some(Self::Rectangle),
            "POLYGON" => Some(Self::Polygon),
            _ => None,
        }
    }
}

/// An instrument field of view.
#[derive(Clone, Debug)]
pub struct Fov {
    /// NAIF instrument id.
    pub instrument: i32,
    /// FOV shape.
    pub shape: FovShape,
    /// Frame in which the boresight and boundary vectors are expressed.
    pub frame: String,
    /// Boresight unit vector.
    pub boresight: [f64; 3],
    original_boresight: [f64; 3],
    /// Boundary vectors, in the FOV frame.
    pub boundary: Vec<[f64; 3]>,
    axis: [f64; 3],
    angular_radius: f64,
    pub(crate) semi_axes: [[f64; 3]; 2],
    pub(crate) semi_lengths: [f64; 2],
    polygon: Vec<[f64; 2]>,
    fov_frame_2d: ([f64; 3], [f64; 3], [f64; 3]),
}

impl Fov {
    /// ZZGFFVIN rejects visibility cones wider than its source margin.
    /// GETFOV parsing alone does not impose this restriction.
    pub(crate) fn validate_visibility(&self) -> Result<(), GeoError> {
        if self.angular_radius > std::f64::consts::FRAC_PI_2 - 1e-6 {
            return Err(GeoError::BadSearch("SPICE(FOVTOOWIDE)"));
        }
        Ok(())
    }

    /// Parses an instrument FOV from the text of an IK/FK.
    pub fn from_text(text: &str, instrument: i32) -> Result<Self, GeoError> {
        let data = extract_data_blocks(text);
        let mut assignments = HashMap::new();
        apply_assignments(&data, &mut assignments)?;
        let prefix = format!("INS{instrument}_");

        let shape_text = find_scalar(&assignments, &format!("{prefix}FOV_SHAPE"))
            .ok_or(GeoError::BadSearch("instrument FOV_SHAPE is missing"))?;
        let shape =
            FovShape::parse(&shape_text).ok_or(GeoError::BadSearch("unsupported FOV_SHAPE"))?;
        let frame = find_scalar(&assignments, &format!("{prefix}FOV_FRAME"))
            .ok_or(GeoError::BadSearch("instrument FOV_FRAME is missing"))?;
        let boresight = find_vector(&assignments, &format!("{prefix}BORESIGHT"))
            .ok_or(GeoError::BadSearch("instrument BORESIGHT is missing"))?;
        let original_boresight = boresight;
        let boresight = unit(boresight);
        if boresight == [0.0; 3] {
            return Err(GeoError::BadSearch("instrument BORESIGHT is zero"));
        }

        let class = find_scalar(&assignments, &format!("{prefix}FOV_CLASS_SPEC"))
            .unwrap_or_else(|| "CORNERS".to_string())
            .trim()
            .to_ascii_uppercase();
        if class != "ANGLES" && class != "CORNERS" {
            return Err(GeoError::BadSearch("unknown FOV_CLASS_SPEC"));
        }

        let boundary = if class == "ANGLES" {
            angles_boundary(&assignments, &prefix, shape, original_boresight)?
        } else {
            let values = find_vector_list(
                &assignments,
                &[
                    format!("{prefix}FOV_BOUNDARY"),
                    format!("{prefix}FOV_BOUNDARY_CORNERS"),
                ],
            )
            .ok_or(GeoError::BadSearch("instrument FOV boundary is missing"))?;
            chunk_vectors(&values)?
        };

        let required = match shape {
            FovShape::Circle => 1,
            FovShape::Ellipse => 2,
            FovShape::Rectangle => 4,
            FovShape::Polygon => 3,
        };
        if boundary.len() < required || (shape != FovShape::Polygon && boundary.len() != required) {
            return Err(GeoError::BadSearch(
                "FOV boundary count is inconsistent with its shape",
            ));
        }

        let axis = if matches!(shape, FovShape::Polygon | FovShape::Rectangle) {
            polygon_axis(&boundary)?
        } else {
            boresight
        };

        let angular_radius = boundary
            .iter()
            .map(|vector| separation(*vector, axis))
            .fold(0.0f64, f64::max);

        let mut fov = Self {
            instrument,
            shape,
            frame,
            boresight,
            original_boresight,
            boundary,
            axis,
            angular_radius,
            semi_axes: [[0.0; 3]; 2],
            semi_lengths: [0.0; 2],
            polygon: Vec::new(),
            fov_frame_2d: frame_from_z(axis),
        };

        if shape == FovShape::Ellipse {
            fov.compute_semi_axes()?;
        }
        if matches!(shape, FovShape::Polygon | FovShape::Rectangle) {
            fov.compute_polygon()?;
        }

        Ok(fov)
    }

    /// Original IK boresight, including its magnitude, as returned by CSPICE getfov.
    /// Geometric visibility uses `boresight`, the normalized counterpart.
    pub fn original_boresight(&self) -> [f64; 3] {
        self.original_boresight
    }

    /// Opens an IK/FK file and parses an instrument FOV.
    pub fn open(path: impl AsRef<std::path::Path>, instrument: i32) -> Result<Self, GeoError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_text(&text, instrument)
    }

    fn compute_semi_axes(&mut self) -> Result<(), GeoError> {
        for index in 0..2 {
            let vector = self.boundary[index];
            let intercept = fov_plane_intercept(self.axis, vector).ok_or(GeoError::BadSearch(
                "FOV boundary does not intersect its plane",
            ))?;
            let semi = subtract(intercept, unit(self.axis));
            let length = norm(semi);
            if length == 0.0 {
                return Err(GeoError::BadSearch("FOV semi-axis has zero length"));
            }
            self.semi_axes[index] = semi;
            self.semi_lengths[index] = length;
        }
        Ok(())
    }

    fn compute_polygon(&mut self) -> Result<(), GeoError> {
        let (_, x_axis, y_axis) = self.fov_frame_2d;
        let mut polygon = Vec::with_capacity(self.boundary.len());
        for vector in &self.boundary {
            let intercept = fov_plane_intercept(self.axis, *vector).ok_or(GeoError::BadSearch(
                "FOV boundary does not intersect its plane",
            ))?;
            let offset = subtract(intercept, unit(self.axis));
            polygon.push([dot(x_axis, offset), dot(y_axis, offset)]);
        }
        self.polygon = polygon;
        Ok(())
    }

    /// Returns true when `direction` (expressed in the FOV frame) is
    /// inside the FOV.
    pub fn contains_direction(&self, direction: [f64; 3]) -> bool {
        let direction = unit(direction);
        self.contains_raw_direction(direction)
    }

    fn contains_raw_direction(&self, direction: [f64; 3]) -> bool {
        if direction == [0.0; 3] {
            return false;
        }
        let separation_from_axis = separation(direction, self.axis);

        if self.shape == FovShape::Circle {
            return separation_from_axis <= self.angular_radius;
        }
        if separation_from_axis > self.angular_radius {
            return false;
        }

        let Some(intercept) = fov_plane_intercept(self.axis, direction) else {
            return false;
        };
        let offset = subtract(intercept, unit(self.axis));

        match self.shape {
            FovShape::Ellipse => {
                let first =
                    (dot(offset, self.semi_axes[0]) / self.semi_lengths[0]) / self.semi_lengths[0];
                let second =
                    (dot(offset, self.semi_axes[1]) / self.semi_lengths[1]) / self.semi_lengths[1];
                first.powf(2.0) + second.powf(2.0) <= 1.0
            }
            FovShape::Polygon | FovShape::Rectangle => {
                let (_, x_axis, y_axis) = self.fov_frame_2d;
                winding_number(&self.polygon, [dot(x_axis, offset), dot(y_axis, offset)]) != 0
            }
            FovShape::Circle => unreachable!(),
        }
    }

    /// The maximum angular separation of the boundary from the FOV axis.
    pub fn angular_radius(&self) -> f64 {
        self.angular_radius
    }

    /// The FOV axis used by the containment test.
    pub fn axis(&self) -> [f64; 3] {
        self.axis
    }
}

/// True when the apparent direction to `target` is inside the FOV of
/// `fov` at `et`. The target is modeled as a point, matching CSPICE
/// `fovtrg` with `TSHAPE = 'POINT'`.
pub fn target_in_fov(
    kernel: &SpkKernel,
    fov: &Fov,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: &Frames,
) -> Result<bool, GeoError> {
    target_in_fov_with_pck(kernel, None, fov, target, observer, et, abcorr, frames)
}

/// Point visibility with PCK-backed instrument frames, evaluated at ET.
#[allow(clippy::too_many_arguments)]
pub fn target_in_fov_with_pck(
    kernel: &SpkKernel,
    pck: Option<&Pck>,
    fov: &Fov,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: &Frames,
) -> Result<bool, GeoError> {
    fov.validate_visibility()?;
    if target == observer {
        return Err(GeoError::BadSearch(
            "FOV target and observer must be distinct",
        ));
    }
    let position = crate::surface::apparent_position(kernel, target, observer, et, abcorr)?;
    let rotation = frames
        .pxform_with_derivative("J2000", &fov.frame, et, pck)?
        .rotation;
    Ok(fov.contains_raw_direction(rotate(&rotation, position)))
}

/// Ellipsoid-target FOV visibility using the N0067 analytic predicates.
#[allow(clippy::too_many_arguments)]
pub fn ellipsoid_in_fov(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: &Fov,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: &Frames,
) -> Result<bool, GeoError> {
    crate::fov_ellipsoid::ellipsoid_in_fov(kernel, pck, fov, target, observer, et, abcorr, frames)
}

/// Ellipsoid visibility in an explicitly selected, target-centered frame.
#[allow(clippy::too_many_arguments)]
pub fn ellipsoid_in_fov_with_frame(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: &Fov,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: &Frames,
    target_frame: &str,
) -> Result<bool, GeoError> {
    crate::fov_ellipsoid::ellipsoid_in_frame(
        kernel,
        pck,
        fov,
        target,
        observer,
        et,
        abcorr,
        frames,
        target_frame,
    )
}

/// FOVTRG with an explicit target ellipsoid frame (ignored for POINT).
#[allow(clippy::too_many_arguments)]
pub fn target_in_fov_shaped_in(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: &Fov,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: &Frames,
    shape: FovTargetShape,
    target_frame: &str,
) -> Result<bool, GeoError> {
    match shape {
        FovTargetShape::Point => {
            target_in_fov_with_pck(kernel, Some(pck), fov, target, observer, et, abcorr, frames)
        }
        FovTargetShape::Ellipsoid => ellipsoid_in_fov_with_frame(
            kernel,
            pck,
            fov,
            target,
            observer,
            et,
            abcorr,
            frames,
            target_frame,
        ),
    }
}

/// GFTFOV with an explicit target ellipsoid frame.
#[allow(clippy::too_many_arguments)]
pub fn find_target_in_fov_shaped_in(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: &Fov,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    frames: &Frames,
    windows: &[Interval],
    options: FinderOptions,
    shape: FovTargetShape,
    target_frame: &str,
) -> Result<Vec<Interval>, GeoError> {
    fov.validate_visibility()?;
    boolean_windows(
        |et| {
            target_in_fov_shaped_in(
                kernel,
                pck,
                fov,
                target,
                observer,
                et,
                abcorr,
                frames,
                shape,
                target_frame,
            )
        },
        windows,
        options,
    )
}

/// True when a ray with direction `direction_j2000` is inside the FOV.
///
/// Light-time corrections are not meaningful for a ray; pass
/// [`Aberration::None`] unless stellar aberration is required.
pub fn ray_in_fov(
    kernel: &SpkKernel,
    fov: &Fov,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
    frames: &Frames,
) -> Result<bool, GeoError> {
    if abcorr != Aberration::None {
        return Err(GeoError::BadSearch(
            "light-time corrections are not defined for a ray target",
        ));
    }

    ray_in_fov_in(
        kernel,
        None,
        fov,
        observer,
        et,
        RayAberration::None,
        direction_j2000,
        "J2000",
        frames,
    )
}

/// Aberration corrections accepted by CSPICE FOVRAY/GFRFOV.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RayAberration {
    None,
    Stellar,
    TransmissionStellar,
}

impl RayAberration {
    pub fn parse(text: &str) -> Result<Self, GeoError> {
        if text.is_empty() {
            return Err(GeoError::BadSearch("SPICE(EMPTYSTRING)"));
        }
        match text.trim().to_ascii_uppercase().as_str() {
            "NONE" => Ok(Self::None),
            "S" => Ok(Self::Stellar),
            "XS" => Ok(Self::TransmissionStellar),
            _ => Err(GeoError::BadSearch("SPICE(INVALIDOPTION)")),
        }
    }
}

/// Source-order FOVRAY including a named ray frame and S/XS corrections.
#[allow(clippy::too_many_arguments)]
pub fn ray_in_fov_in(
    kernel: &SpkKernel,
    pck: Option<&Pck>,
    fov: &Fov,
    observer: i32,
    et: f64,
    correction: RayAberration,
    ray: [f64; 3],
    ray_frame: &str,
    frames: &Frames,
) -> Result<bool, GeoError> {
    fov.validate_visibility()?;
    if ray == [0.0; 3] || !ray.iter().all(|x| x.is_finite()) {
        return Err(GeoError::BadSearch(
            "ray direction must be finite and nonzero",
        ));
    }
    let rotation = frames
        .pxform_with_derivative(ray_frame, &fov.frame, et, pck)?
        .rotation;
    let mut direction = rotate(&rotation, unit(ray));
    if correction != RayAberration::None {
        let to_j2000 = frames
            .pxform_with_derivative(&fov.frame, "J2000", et, pck)?
            .rotation;
        let mut velocity = kernel.absolute_state(observer, et)?.velocity;
        if correction == RayAberration::TransmissionStellar {
            velocity = negate(velocity);
        }
        direction = rotate(
            &transpose(&to_j2000),
            crate::surface::stellar_aberration_position(rotate(&to_j2000, direction), velocity),
        );
    }
    Ok(fov.contains_raw_direction(direction))
}

fn boolean_windows<F: Fn(f64) -> Result<bool, GeoError>>(
    predicate: F,
    windows: &[Interval],
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    if !options.step.is_finite()
        || options.step <= 0.0
        || !options.tolerance.is_finite()
        || options.tolerance <= 0.0
    {
        return Err(GeoError::BadSearch(
            "step and tolerance must be positive and finite",
        ));
    }
    let mut found = Vec::new();
    for window in normalize_windows(windows)? {
        found.extend(solve_boolean(
            &predicate,
            window,
            options.step,
            options.tolerance,
        )?);
    }
    normalize_windows(&found)
}

/// Time intervals during which the point target is inside the FOV.
#[allow(clippy::too_many_arguments)]
pub fn find_target_in_fov(
    kernel: &SpkKernel,
    fov: &Fov,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    frames: &Frames,
    windows: &[Interval],
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    fov.validate_visibility()?;
    boolean_windows(
        |et| target_in_fov(kernel, fov, target, observer, et, abcorr, frames),
        windows,
        options,
    )
}

/// The target model used by the FOV visibility tests, matching the
/// WebGeocalc `shape` values for the FOV calculations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FovTargetShape {
    /// Treat the target as a point.
    Point,
    /// Treat the target as a triaxial ellipsoid from the text PCK.
    Ellipsoid,
}

impl FovTargetShape {
    /// Parses a WebGeocalc `shape` string.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "POINT" => Some(Self::Point),
            "ELLIPSOID" => Some(Self::Ellipsoid),
            _ => None,
        }
    }
}

/// Visibility of a shaped target, matching CSPICE `fovtrg` with the
/// corresponding `TSHAPE`.
#[allow(clippy::too_many_arguments)]
pub fn target_in_fov_shaped(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: &Fov,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: &Frames,
    shape: FovTargetShape,
) -> Result<bool, GeoError> {
    match shape {
        FovTargetShape::Point => {
            target_in_fov_with_pck(kernel, Some(pck), fov, target, observer, et, abcorr, frames)
        }
        FovTargetShape::Ellipsoid => {
            ellipsoid_in_fov(kernel, pck, fov, target, observer, et, abcorr, frames)
        }
    }
}

/// Time intervals during which the shaped target is inside the FOV.
#[allow(clippy::too_many_arguments)]
pub fn find_target_in_fov_shaped(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: &Fov,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    frames: &Frames,
    windows: &[Interval],
    options: FinderOptions,
    shape: FovTargetShape,
) -> Result<Vec<Interval>, GeoError> {
    fov.validate_visibility()?;
    boolean_windows(
        |et| {
            target_in_fov_shaped(
                kernel, pck, fov, target, observer, et, abcorr, frames, shape,
            )
        },
        windows,
        options,
    )
}

/// Time intervals during which the ray is inside the FOV.
#[allow(clippy::too_many_arguments)]
pub fn find_ray_in_fov(
    kernel: &SpkKernel,
    fov: &Fov,
    observer: i32,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
    frames: &Frames,
    windows: &[Interval],
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    fov.validate_visibility()?;
    boolean_windows(
        |et| ray_in_fov(kernel, fov, observer, et, abcorr, direction_j2000, frames),
        windows,
        options,
    )
}

/// GFRFOV in an arbitrary ray frame, with NONE/S/XS corrections.
#[allow(clippy::too_many_arguments)]
pub fn find_ray_in_fov_in(
    kernel: &SpkKernel,
    pck: Option<&Pck>,
    fov: &Fov,
    observer: i32,
    correction: RayAberration,
    ray: [f64; 3],
    ray_frame: &str,
    frames: &Frames,
    windows: &[Interval],
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    fov.validate_visibility()?;
    boolean_windows(
        |et| {
            ray_in_fov_in(
                kernel, pck, fov, observer, et, correction, ray, ray_frame, frames,
            )
        },
        windows,
        options,
    )
}

fn angles_boundary(
    assignments: &HashMap<String, Value>,
    prefix: &str,
    shape: FovShape,
    boresight: [f64; 3],
) -> Result<Vec<[f64; 3]>, GeoError> {
    let reference = find_vector(assignments, &format!("{prefix}FOV_REF_VECTOR"))
        .ok_or(GeoError::BadSearch("instrument FOV_REF_VECTOR is missing"))?;
    let units = find_scalar(assignments, &format!("{prefix}FOV_ANGLE_UNITS"))
        .ok_or(GeoError::BadSearch("instrument FOV_ANGLE_UNITS is missing"))?
        .trim()
        .to_ascii_uppercase();
    let degrees_per_radian = 180.0 / std::f64::consts::PI;
    let factor = match units.as_str() {
        "RADIANS" => degrees_per_radian,
        "DEGREES" => 1.0,
        "ARCMINUTES" => 0.016666666666666666,
        "ARCSECONDS" => 2.7777777777777778e-4,
        "HOURANGLE" => 15.0,
        "MINUTEANGLE" => 0.25,
        "SECONDANGLE" => 0.0041666666666666666,
        _ => return Err(GeoError::BadSearch("unknown FOV angular units")),
    };
    let convert = |value: f64| value * factor / degrees_per_radian;
    let ref_angle = convert(
        find_number(assignments, &format!("{prefix}FOV_REF_ANGLE"))
            .ok_or(GeoError::BadSearch("instrument FOV_REF_ANGLE is missing"))
            .and_then(|text| parse_f64(&text))?,
    );
    let cross_angle = find_number(assignments, &format!("{prefix}FOV_CROSS_ANGLE"))
        .map(|text| parse_f64(&text))
        .transpose()?
        .map(convert);

    let cross_axis = cross(boresight, reference);
    if cross_axis == [0.0; 3] {
        return Err(GeoError::BadSearch(
            "FOV_REF_VECTOR is parallel to the boresight",
        ));
    }
    let reference_axis = cross(boresight, cross_axis);

    Ok(match shape {
        FovShape::Circle => vec![rotate_about_axis(boresight, cross_axis, ref_angle)],
        FovShape::Ellipse => {
            let cross_angle =
                cross_angle.ok_or(GeoError::BadSearch("instrument FOV_CROSS_ANGLE is missing"))?;
            vec![
                rotate_about_axis(boresight, cross_axis, ref_angle),
                rotate_about_axis(boresight, reference_axis, cross_angle),
            ]
        }
        FovShape::Rectangle => {
            let cross_angle =
                cross_angle.ok_or(GeoError::BadSearch("instrument FOV_CROSS_ANGLE is missing"))?;
            let magnitude = norm(boresight);
            let b = unit(boresight);
            let b1 = unit(perpendicular(reference, boresight));
            let b2 = cross(b, b1);
            let (cr, cc) = (ref_angle.cos(), cross_angle.cos());
            if cr < 1e-15 || cc < 1e-15 {
                return Err(GeoError::BadSearch("improper rectangular FOV angles"));
            }
            let (sr, sc) = (ref_angle.sin(), cross_angle.sin());
            let normals = [
                add(scale(b1, -cr), scale(b, sr)),
                add(scale(b2, -cc), scale(b, sc)),
                add(scale(b1, cr), scale(b, sr)),
                add(scale(b2, cc), scale(b, sc)),
            ];
            let mut corners = Vec::with_capacity(4);
            for i in 0..4 {
                corners.push(scale(
                    unit(cross(normals[i], normals[(i + 1) % 4])),
                    magnitude,
                ));
            }
            corners
        }
        FovShape::Polygon => {
            return Err(GeoError::BadSearch(
                "the ANGLES class does not support POLYGON FOVs",
            ))
        }
    })
}

fn polygon_axis(boundary: &[[f64; 3]]) -> Result<[f64; 3], GeoError> {
    if boundary.len() < 3 {
        return Err(GeoError::BadSearch(
            "polygon FOV requires at least three boundaries",
        ));
    }
    let mut sum = [0.0; 3];
    for (index, vector) in boundary.iter().enumerate() {
        if cross(*vector, boundary[(index + 1) % boundary.len()]) == [0.; 3] {
            return Err(GeoError::BadSearch(
                "consecutive polygon boundaries are linearly dependent",
            ));
        }
        sum = add(sum, unit(*vector));
    }
    let average = scale(sum, 1.0 / boundary.len() as f64);
    let axis = unit(average);
    if boundary
        .iter()
        .any(|vector| separation(*vector, average) > std::f64::consts::FRAC_PI_2 - 1e-12)
    {
        // ZZHULLAX unitizes its candidate; ZZFOVAXI applies VHATIP once
        // more after either construction path.
        return hull_axis(boundary).map(unit);
    }
    Ok(axis)
}

// Literal N0067 ZZHULLAX fallback: select an exterior hull face, then
// rotate its inward normal into the available longitude sector.
fn hull_axis(bounds: &[[f64; 3]]) -> Result<[f64; 3], GeoError> {
    let half = std::f64::consts::FRAC_PI_2;
    let pi = std::f64::consts::PI;
    let face = |i: usize, j: usize| -> Option<[f64; 3]> {
        let mut normal = cross(bounds[i], bounds[j]);
        if normal == [0.; 3] {
            return None;
        }
        let mut first = true;
        for (m, &vector) in bounds.iter().enumerate() {
            if m == i || m == j {
                continue;
            }
            let mut angle = separation(normal, vector);
            if first {
                if angle > half {
                    normal = scale(normal, -1.);
                    angle = pi - angle;
                }
                first = false;
            }
            if angle >= half - 1e-12 {
                return None;
            }
        }
        Some(normal)
    };
    let mut chosen = None;
    for i in 0..bounds.len() {
        let j = (i + 1) % bounds.len();
        if let Some(normal) = face(i, j) {
            chosen = Some((i, j, normal));
            break;
        }
    }
    if chosen.is_none() {
        'outer: for i in 0..bounds.len() {
            for j in i + 1..bounds.len() {
                if let Some(normal) = face(i, j) {
                    chosen = Some((i, j, normal));
                    break 'outer;
                }
            }
        }
    }
    let (first, second, normal) = chosen.ok_or(GeoError::BadSearch("no exterior FOV hull face"))?;
    let x = unit(add(
        scale(unit(bounds[first]), 0.5),
        scale(unit(bounds[second]), 0.5),
    ));
    let y = unit(normal);
    let z = crate::surface::unit_cross(x, y);
    let transform = [x, y, z];
    let mut minimum = pi;
    let mut maximum = 0.0_f64;
    for (index, &vector) in bounds.iter().enumerate() {
        if index == first || index == second {
            continue;
        }
        let v = rotate(&transform, vector);
        let longitude = if v[0] == 0. && v[1] == 0. {
            0.
        } else {
            v[1].atan2(v[0])
        };
        minimum = minimum.min(longitude);
        maximum = maximum.max(longitude);
    }
    if minimum < 2e-12 || maximum > pi - 2e-12 {
        return Err(GeoError::BadSearch(
            "FOV hull face has invalid longitude span",
        ));
    }
    let delta = pi - maximum;
    let axis = unit(rotate_about_axis(y, z, -delta / 2.0));
    if bounds.iter().any(|&v| separation(v, axis) > half - 1e-12) {
        return Err(GeoError::BadSearch("FOV hull exceeds angular limit"));
    }
    Ok(axis)
}

fn find_scalar(assignments: &HashMap<String, Value>, key: &str) -> Option<String> {
    assignments.get(key).and_then(Value::as_string)
}

fn find_number(assignments: &HashMap<String, Value>, key: &str) -> Option<String> {
    assignments
        .get(key)
        .and_then(Value::as_number)
        .map(str::to_string)
}

fn find_vector(assignments: &HashMap<String, Value>, key: &str) -> Option<[f64; 3]> {
    let values = find_vector_list(assignments, &[key.to_string()])?;
    if values.len() != 3 {
        return None;
    }
    Some([values[0], values[1], values[2]])
}

fn find_vector_list(assignments: &HashMap<String, Value>, keys: &[String]) -> Option<Vec<f64>> {
    for key in keys {
        if let Some(value) = assignments.get(key) {
            if let Some(items) = value.as_list() {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(parse_f64(item).ok()?);
                }
                return Some(values);
            }
        }
    }
    None
}

fn chunk_vectors(values: &[f64]) -> Result<Vec<[f64; 3]>, GeoError> {
    if values.len() % 3 != 0 {
        return Err(GeoError::BadSearch(
            "FOV boundary vector list is not a multiple of 3",
        ));
    }
    Ok(values
        .chunks_exact(3)
        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
        .collect())
}

/// SPICE `FRAME`: right-handed orthonormal frame with `z` as its third
/// axis and the first axis constructed from the input vector.
fn frame_from_z(z: [f64; 3]) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let x = unit(z);
    let (a, b, c) = (x[0] * x[0], x[1] * x[1], x[2] * x[2]);
    if a + b + c == 0.0 {
        return ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    }
    let (f, s1, s2, s3) = if a <= b && a <= c {
        ((b + c).sqrt(), 0, 1, 2)
    } else if b <= a && b <= c {
        ((a + c).sqrt(), 1, 2, 0)
    } else {
        ((a + b).sqrt(), 2, 0, 1)
    };
    let mut y = [0.0; 3];
    let mut z_out = [0.0; 3];
    y[s1] = 0.0;
    y[s2] = -x[s3] / f;
    y[s3] = x[s2] / f;
    z_out[s1] = f;
    z_out[s2] = -x[s1] * y[s3];
    z_out[s3] = x[s1] * y[s2];
    (x, y, z_out)
}

fn winding_number(polygon: &[[f64; 2]], point: [f64; 2]) -> i32 {
    let mut total = 0.0;
    let mut previous = [polygon[0][0] - point[0], polygon[0][1] - point[1]];
    for index in 1..=polygon.len() {
        let vertex = polygon[index % polygon.len()];
        let next = [vertex[0] - point[0], vertex[1] - point[1]];
        let sep = separation_2d(next, previous);
        if next[0] * -previous[1] + next[1] * previous[0] >= 0.0 {
            total += sep;
        } else {
            total -= sep;
        }
        previous = next;
    }
    (total / std::f64::consts::TAU).round() as i32
}

fn separation_2d(a: [f64; 2], b: [f64; 2]) -> f64 {
    let norm = |v: [f64; 2]| {
        let m = v[0].abs().max(v[1].abs());
        if m == 0.0 {
            0.0
        } else {
            m * ((v[0] / m).powi(2) + (v[1] / m).powi(2)).sqrt()
        }
    };
    let (na, nb) = (norm(a), norm(b));
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    let d = a[0] * b[0] + a[1] * b[1];
    if d == 0.0 {
        return std::f64::consts::FRAC_PI_2;
    }
    let (ra, rb) = (1.0 / na, 1.0 / nb);
    let sign = if d > 0.0 { -1.0 } else { 1.0 };
    let x = a[0] * ra + sign * b[0] * rb;
    let y = a[1] * ra + sign * b[1] * rb;
    let angle = ((x * x + y * y).sqrt() * 0.5).asin() * 2.0;
    if d > 0.0 {
        angle
    } else {
        std::f64::consts::PI - angle
    }
}

// NVC2PL followed by INRYPL, specialized to a zero ray vertex.
fn fov_plane_intercept(axis: [f64; 3], direction: [f64; 3]) -> Option<[f64; 3]> {
    let constant = 1.0 / norm(axis);
    let normal = unit(axis);
    let ray = unit(direction);
    let projection = dot(ray, normal);
    if projection <= 0.0 {
        return None;
    }
    if 1.0 >= projection * (f64::MAX / 3.0) / constant.max(1.0) {
        return None;
    }
    Some(scale(scale(ray, 1.0 / projection), constant))
}

fn rotate_about_axis(vector: [f64; 3], axis: [f64; 3], angle: f64) -> [f64; 3] {
    if norm(axis) == 0.0 {
        return vector;
    }
    let axis = unit(axis);
    let projection = crate::surface::project(vector, axis);
    let first = subtract(vector, projection);
    let second = cross(axis, first);
    add(
        add(scale(first, angle.cos()), scale(second, angle.sin())),
        projection,
    )
}

fn rotate(matrix: &Mat3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

/// CSPICE `VSEP`: stable angle between two vectors.
pub fn separation(first: [f64; 3], second: [f64; 3]) -> f64 {
    let first_length = norm(first);
    let second_length = norm(second);
    if first_length == 0.0 || second_length == 0.0 {
        return 0.0;
    }
    let u1 = [
        first[0] / first_length,
        first[1] / first_length,
        first[2] / first_length,
    ];
    let u2 = [
        second[0] / second_length,
        second[1] / second_length,
        second[2] / second_length,
    ];
    let cosine = dot(u1, u2);
    if cosine > 0.0 {
        2.0 * (norm(subtract(u1, u2)) * 0.5).asin()
    } else if cosine < 0.0 {
        std::f64::consts::PI - 2.0 * (norm(add(u1, u2)) * 0.5).asin()
    } else {
        std::f64::consts::FRAC_PI_2
    }
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let length = norm(a);
    if length == 0.0 {
        [0.0; 3]
    } else {
        [a[0] / length, a[1] / length, a[2] / length]
    }
}

fn norm(a: [f64; 3]) -> f64 {
    let m = a[0].abs().max(a[1].abs()).max(a[2].abs());
    if m == 0.0 {
        0.0
    } else {
        let x = a[0] / m;
        let y = a[1] / m;
        let z = a[2] / m;
        m * (x * x + y * y + z * z).sqrt()
    }
}

fn perpendicular(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let ma = a[0].abs().max(a[1].abs()).max(a[2].abs());
    let mb = b[0].abs().max(b[1].abs()).max(b[2].abs());
    if ma == 0.0 {
        return [0.; 3];
    }
    if mb == 0.0 {
        return a;
    }
    let a = [a[0] / ma, a[1] / ma, a[2] / ma];
    let b = [b[0] / mb, b[1] / mb, b[2] / mb];
    scale(subtract(a, crate::surface::project(a, b)), ma)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn negate(a: [f64; 3]) -> [f64; 3] {
    [-a[0], -a[1], -a[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod axis_precision {
    #[test]
    fn fov_hull_axis_exact_cspice() {
        let mut failures = Vec::new();
        let mut successes = 0;
        for line in include_str!("../../tests/golden/fov_axis_exact.csv").lines() {
            let f: Vec<_> = line.split(',').collect();
            let n: usize = f[1].parse().unwrap();
            let values: Vec<f64> = f[3..].iter().map(|x| x.parse().unwrap()).collect();
            let bounds: Vec<_> = values[3..]
                .chunks_exact(3)
                .map(|x| [x[0], x[1], x[2]])
                .collect();
            assert_eq!(bounds.len(), n);
            match (super::polygon_axis(&bounds), f[2] == "1") {
                (Ok(axis), true) => {
                    successes += 1;
                    if axis != values[..3] {
                        failures.push(format!("{} {axis:?} != {:?}", f[0], &values[..3]));
                    }
                }
                (Err(_), false) => {}
                (actual, expected) => {
                    failures.push(format!("{} {actual:?} expected{expected}", f[0]))
                }
            }
        }
        assert!(successes > 100);
        assert!(
            failures.is_empty(),
            "{} hull axis differences: {:?}",
            failures.len(),
            failures.iter().take(4).collect::<Vec<_>>()
        );
    }
}
