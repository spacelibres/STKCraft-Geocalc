// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::too_many_arguments, clippy::needless_late_init)]
//! Ray-ellipsoid tangent point, matching CSPICE `tangpt`.
//!
//! The tangent point is the point on a ray emanating from an observer
//! that is nearest to the target's surface. The point on the surface
//! nearest to that ray point is reported separately. This is the
//! geometry WebGeocalc exposes as *Tangent Point*.
//!
//! The result is expressed in the target's body-fixed frame. The
//! direction vector is supplied in the J2000 frame; any frame kernel
//! rotation is the caller's responsibility.

use crate::error::GeoError;
use crate::frame::{transpose, Frames, Mat3};
use crate::pck::Pck;
use crate::spk::{Aberration, SpkKernel};

/// Location at which light-time and stellar corrections are evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrectionLocus {
    /// Correct the point on the ray nearest to the surface.
    TangentPoint,
    /// Correct the nearest point on the ellipsoid surface.
    SurfacePoint,
}
impl CorrectionLocus {
    /// Parse the WebGeocalc/CSPICE correction locus.
    pub fn parse(value: &str) -> Option<Self> {
        match value
            .to_ascii_uppercase()
            .replace([' ', '_', '-'], "")
            .as_str()
        {
            "TANGENTPOINT" => Some(Self::TangentPoint),
            "SURFACEPOINT" => Some(Self::SurfacePoint),
            _ => None,
        }
    }
}

/// The tangent point and the surface point nearest to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TangentPoint {
    /// Tangent point relative to the target centre, body-fixed, km.
    pub point: [f64; 3],
    /// Distance from the tangent point to the surface, km.
    pub altitude: f64,
    /// Distance from the observer to the tangent point, km.
    pub range: f64,
    /// Surface point nearest the tangent point, body-fixed, km.
    pub surface_point: [f64; 3],
    /// Surface point minus the observer position, body-fixed, km.
    pub surface_vector: [f64; 3],
    /// Target epoch at which the body orientation was evaluated.
    pub epoch: f64,
    /// One-way light time used for the correction, seconds.
    pub light_time: f64,
}

/// Computes the tangent point of a ray to a triaxial ellipsoid.
///
/// `direction_j2000` is the ray direction in J2000. It need not be
/// unit length. `fixref` is always the IAU body-fixed frame of
/// `target`, built from the loaded text PCK.
pub fn tangent_point(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
) -> Result<TangentPoint, GeoError> {
    tangent_point_with_locus(
        kernel,
        pck,
        target,
        observer,
        et,
        abcorr,
        direction_j2000,
        CorrectionLocus::TangentPoint,
    )
}

/// CSPICE `tangpt` with an explicit aberration-correction locus.
pub fn tangent_point_with_locus(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
    locus: CorrectionLocus,
) -> Result<TangentPoint, GeoError> {
    tangent_point_in(
        kernel,
        pck,
        &Frames::new(),
        &format!("IAU_{target}"),
        target,
        observer,
        et,
        abcorr,
        direction_j2000,
        locus,
    )
}

/// CSPICE TANGPT with an explicit target-centered frame and correction locus.
pub fn tangent_point_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
    locus: CorrectionLocus,
) -> Result<TangentPoint, GeoError> {
    crate::surface::validate_target_frame(frames, target_frame, target)?;
    if target == observer {
        return Err(GeoError::BadSearch("observer and target coincide"));
    }
    if norm(direction_j2000) == 0.0 {
        return Err(GeoError::BadSearch("ray direction is the zero vector"));
    }
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let (uselt, cn, stellar, transmission) = abcorr.flags();
    let frame = crate::spk::FrameSource::Named {
        frames,
        pck,
        frame_name: target_frame,
    };
    let (initial_target, initial_lt) =
        crate::surface::apparent_position_in_frame(kernel, target, observer, et, abcorr, &frame)?;
    let mut epoch = crate::state_rep::time_at_target(et, initial_lt, abcorr);
    let rotation = frame.rotation_at(epoch)?;
    let mut fixed_observer = negate(initial_target);
    let mut fixed_direction = rotate(&rotation, direction_j2000);
    let mut result = solve_geometry(fixed_observer, fixed_direction, radii, !uselt)?;
    if !uselt {
        result.epoch = et;
        result.light_time = if locus == CorrectionLocus::TangentPoint {
            result.range
        } else {
            norm(result.surface_vector)
        } / crate::spk::CLIGHT;
        return Ok(result);
    }
    let obs = kernel.state_in_frame(observer, 0, et, frames, Some(pck), "J2000")?;
    let distance = if locus == CorrectionLocus::TangentPoint {
        norm(subtract(fixed_observer, result.point))
    } else {
        norm(result.surface_vector)
    };
    let mut lt = distance / crate::spk::CLIGHT;
    epoch = crate::state_rep::time_at_target(et, lt, abcorr);
    let mut previous_lt = 0.0;
    let mut stellar_observer = fixed_observer;
    for _ in 0..if cn { 10 } else { 3 } {
        let previous_tangent = result.point;
        let previous_surface = result.surface_point;
        let center = kernel.state_in_frame(target, 0, epoch, frames, Some(pck), "J2000")?;
        let inertial_observer = subtract(obs.position, center.position);
        let rotation = frame.rotation_at(epoch)?;
        let offset = if stellar {
            let point = if locus == CorrectionLocus::TangentPoint {
                result.point
            } else {
                result.surface_point
            };
            let inertial_point = rotate(&transpose(&rotation), point);
            let relative = add(negate(inertial_observer), inertial_point);
            let velocity = if transmission {
                negate(obs.velocity)
            } else {
                obs.velocity
            };
            let corrected = crate::surface::stellar_aberration_position(relative, velocity);
            rotate(&rotation, subtract(corrected, relative))
        } else {
            [0.0; 3]
        };
        fixed_observer = rotate(&rotation, inertial_observer);
        fixed_direction = rotate(&rotation, direction_j2000);
        stellar_observer = subtract(fixed_observer, offset);
        result = solve_geometry(stellar_observer, fixed_direction, radii, true)?;
        let next_epoch;
        if result.range == 0.0 {
            lt = if locus == CorrectionLocus::TangentPoint {
                0.0
            } else {
                result.altitude / crate::spk::CLIGHT
            };
            next_epoch = crate::state_rep::time_at_target(et, lt, abcorr);
        } else {
            let point = if locus == CorrectionLocus::TangentPoint {
                result.point
            } else {
                result.surface_point
            };
            lt = norm(subtract(fixed_observer, point)) / crate::spk::CLIGHT;
            next_epoch = crate::state_rep::time_at_target(et, lt, abcorr);
        }
        let lt_converged = (lt - previous_lt).abs() < lt.abs() * 1e-16 || next_epoch == epoch;
        let stellar_converged = !stellar
            || (relative_difference(result.point, previous_tangent) < 1e-16
                && relative_difference(result.surface_point, previous_surface) < 1e-16);
        epoch = next_epoch;
        previous_lt = lt;
        if lt_converged && stellar_converged {
            break;
        }
    }
    result.range = norm(subtract(stellar_observer, result.point));
    result.epoch = epoch;
    result.light_time = lt;
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn solve_geometry(
    observer_in_body: [f64; 3],
    direction: [f64; 3],
    radii: [f64; 3],
    apply_ray_limit: bool,
) -> Result<TangentPoint, GeoError> {
    let level = (observer_in_body[0] / radii[0]).powi(2)
        + (observer_in_body[1] / radii[1]).powi(2)
        + (observer_in_body[2] / radii[2]).powi(2);
    if level < 1.0 {
        return Err(GeoError::BadSearch(
            "observer is inside the target ellipsoid",
        ));
    }

    let (surface_point, altitude) =
        nearest_point_on_ellipsoid_to_line(radii, observer_in_body, direction)?;
    let offset = subtract(surface_point, observer_in_body);
    let mut point = add(crate::surface::project(offset, direction), observer_in_body);
    let range;
    let mut surface_point = surface_point;
    let mut altitude = altitude;

    if apply_ray_limit && dot(subtract(point, observer_in_body), direction) <= 0.0 {
        point = observer_in_body;
        range = 0.0;
        let near = crate::surface::nearest_point_on_ellipsoid(observer_in_body, radii)?;
        surface_point = near.point;
        altitude = near.altitude;
    } else if apply_ray_limit && altitude == 0.0 {
        point = surface_point;
        range = norm(subtract(observer_in_body, surface_point));
    } else {
        range = norm(subtract(observer_in_body, point));
    }

    Ok(TangentPoint {
        point,
        altitude,
        range,
        surface_point,
        surface_vector: subtract(surface_point, observer_in_body),
        epoch: 0.0,
        light_time: 0.0,
    })
}

/// Nearest point on a triaxial ellipsoid to a line, and the distance
/// between them. Matches the geometric result of CSPICE `npedln`.
pub fn nearest_point_on_ellipsoid_to_line(
    radii: [f64; 3],
    line_point: [f64; 3],
    direction: [f64; 3],
) -> Result<([f64; 3], f64), GeoError> {
    if radii.iter().any(|&r| r <= 0.0) {
        return Err(GeoError::BadSearch("ellipsoid axes must be positive"));
    }
    if norm(direction) == 0.0 {
        return Err(GeoError::BadSearch("line direction is the zero vector"));
    }
    let direction = unit(direction);
    let scaling = radii[0].max(radii[1]).max(radii[2]);
    let radii = [radii[0] / scaling, radii[1] / scaling, radii[2] / scaling];
    let [a, b, c] = radii;
    let line_point = [
        line_point[0] / scaling,
        line_point[1] / scaling,
        line_point[2] / scaling,
    ];
    if let Some(point) = crate::surface::ray_surface_intersection(radii, line_point, direction)? {
        return Ok((scale(point, scaling), 0.0));
    }
    if let Some(point) =
        crate::surface::ray_surface_intersection(radii, line_point, negate(direction))?
    {
        return Ok((scale(point, scaling), 0.0));
    }

    // NPEDLN: intersect the ellipsoid with the plane containing all
    // candidate near points, project that ellipse perpendicular to the
    // ray, solve NPELPT, then invert the projection (VPRJPI).
    let normal = unit([
        direction[0] / (a * a),
        direction[1] / (b * b),
        direction[2] / (c * c),
    ]);
    let (_, span1, span2) = orthonormal_frame(normal);
    let distorted1 = [
        (1.0 / a) * span1[0],
        (1.0 / b) * span1[1],
        (1.0 / c) * span1[2],
    ];
    let distorted2 = [
        (1.0 / a) * span2[0],
        (1.0 / b) * span2[1],
        (1.0 / c) * span2[2],
    ];
    let distorted_normal = crate::surface::unit_cross(distorted1, distorted2);
    let (_, vec1, vec2) = orthonormal_frame(distorted_normal);
    let vec1 = [a * vec1[0], b * vec1[1], c * vec1[2]];
    let vec2 = [a * vec2[0], b * vec2[1], c * vec2[2]];
    let (major, minor) = ellipse_axes(vec1, vec2);
    let projection_normal = unit(direction);
    let pmajor = crate::surface::perpendicular(major, projection_normal);
    let pminor = crate::surface::perpendicular(minor, projection_normal);
    let (major, minor) = ellipse_axes(pmajor, pminor);
    let projected_point = add(
        line_point,
        scale(projection_normal, -dot(line_point, projection_normal)),
    );
    let (near, distance) = nearest_on_ellipse(projected_point, major, minor)?;
    let multiplier = (-dot(near, normal)) / dot(projection_normal, normal);
    let point = add(near, scale(projection_normal, multiplier));
    Ok((scale(point, scaling), scaling * distance))
}

// Source ports of FRAME, SAELGV/DIAGS2/RQUAD, and NPELPT. Keeping
// their operation order is necessary for CSPICE floating-point parity.
pub(crate) fn orthonormal_frame(axis: [f64; 3]) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let x = unit(axis);
    let [a, b, c] = [x[0] * x[0], x[1] * x[1], x[2] * x[2]];
    if a + b + c == 0.0 {
        return ([1., 0., 0.], [0., 1., 0.], [0., 0., 1.]);
    }
    let (i, j, k, f) = if a <= b && a <= c {
        (0, 1, 2, (b + c).sqrt())
    } else if b <= a && b <= c {
        (1, 2, 0, (a + c).sqrt())
    } else {
        (2, 0, 1, (a + b).sqrt())
    };
    let mut y = [0.0; 3];
    let mut z = [0.0; 3];
    y[i] = 0.0;
    y[j] = -x[k] / f;
    y[k] = x[j] / f;
    z[i] = f;
    z[j] = -x[i] * y[k];
    z[k] = x[i] * y[j];
    (x, y, z)
}
pub(crate) fn ellipse_axes(first: [f64; 3], second: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let scaling = norm(first).max(norm(second));
    if scaling == 0.0 {
        return ([0.0; 3], [0.0; 3]);
    }
    let a = [first[0] / scaling, first[1] / scaling, first[2] / scaling];
    let b = [
        second[0] / scaling,
        second[1] / scaling,
        second[2] / scaling,
    ];
    let (d, c) = diagonalize(dot(a, a), dot(a, b), dot(b, b));
    let (major, minor) = if d[0].abs() >= d[1].abs() {
        (0, 1)
    } else {
        (1, 0)
    };
    let major = add(scale(a, c[major][0]), scale(b, c[major][1]));
    let minor = add(scale(a, c[minor][0]), scale(b, c[minor][1]));
    (scale(major, scaling), scale(minor, scaling))
}
fn quadratic_roots(a: f64, b: f64, c: f64) -> [f64; 2] {
    let scaling = a.abs().max(b.abs()).max(c.abs());
    let (a, b, c) = if (a != 0.0 && a / scaling == 0.0)
        || (b != 0.0 && b / scaling == 0.0)
        || (c != 0.0 && c / scaling == 0.0)
    {
        (a, b, c)
    } else {
        (a / scaling, b / scaling, c / scaling)
    };
    let discriminant = b * b - a * 4.0 * c;
    if b < 0.0 {
        let first = (-b + discriminant.sqrt()) / (a * 2.0);
        [first, c / a / first]
    } else if b > 0.0 {
        let second = (-b - discriminant.sqrt()) / (a * 2.0);
        [c / a / second, second]
    } else {
        let first = discriminant.sqrt() / (a * 2.0);
        [first, -first]
    }
}
fn unit2(v: [f64; 2]) -> [f64; 2] {
    let maximum = v[0].abs().max(v[1].abs());
    if maximum == 0.0 {
        return [0.0; 2];
    }
    let x = v[0] / maximum;
    let y = v[1] / maximum;
    let length = maximum * (x * x + y * y).sqrt();
    [v[0] / length, v[1] / length]
}
fn diagonalize(s0: f64, s1: f64, s3: f64) -> ([f64; 2], [[f64; 2]; 2]) {
    if s1 == 0.0 {
        return ([s0, s3], [[1.0, 0.0], [0.0, 1.0]]);
    }
    let scaling = s0.abs().max(s1.abs()).max(s3.abs());
    let a = s0 / scaling;
    let b = s1 / scaling;
    let c = s3 / scaling;
    let mut d = quadratic_roots(1.0, -(a + c), a * c - b * b);
    let rotate = if (d[0] - a).abs() >= (d[0] - c).abs() {
        let v = unit2([b, (d[0] - a).max(b.abs())]);
        d.swap(0, 1);
        [[v[1], -v[0]], [v[0], v[1]]]
    } else {
        let v = unit2([(d[0] - c).max(b.abs()), b]);
        [[v[0], v[1]], [-v[1], v[0]]]
    };
    ([d[0] * scaling, d[1] * scaling], rotate)
}
fn nearest_on_ellipse(
    point: [f64; 3],
    major: [f64; 3],
    minor: [f64; 3],
) -> Result<([f64; 3], f64), GeoError> {
    let minlen = norm(minor);
    let majlen = norm(major);
    if minlen.min(majlen) == 0.0 {
        return Err(GeoError::BadSearch("degenerate projected ellipse"));
    }
    let factor = 1.0 / majlen;
    let major = scale(major, factor);
    let minor = scale(minor, factor);
    let translated = scale(point, factor);
    // TWOVEC(major,1,minor,2): rows X, Y, Z.
    let x = unit(major);
    let z = crate::surface::unit_cross(major, minor);
    let y = crate::surface::unit_cross(z, major);
    let rotation = [x, y, z];
    let local = rotate(&rotation, translated);
    let near = crate::surface::nearest_point_on_ellipsoid(
        [local[0], local[1], 0.0],
        [1.0, minlen / majlen, 2.0],
    )?;
    let near = scale(near.point, majlen);
    let near = rotate(&transpose(&rotation), near);
    Ok((near, norm(subtract(near, point))))
}

fn rotate(matrix: &Mat3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    crate::surface::unit(a)
}

fn norm(a: [f64; 3]) -> f64 {
    crate::surface::norm(a)
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

#[cfg(test)]
mod tests {
    use super::nearest_point_on_ellipsoid_to_line;

    #[test]
    fn nearest_point_to_axis_line() {
        let (point, distance) = nearest_point_on_ellipsoid_to_line(
            [1000.0, 2000.0, 3000.0],
            [5000.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        )
        .unwrap();
        assert!((point[0] - 1000.0).abs() < 1e-12);
        assert!(point[1].abs() < 1e-12);
        assert!(point[2].abs() < 1e-12);
        assert!((distance - 4000.0).abs() < 1e-9);
    }

    #[test]
    fn intercepting_line_returns_surface_point() {
        let (point, distance) = nearest_point_on_ellipsoid_to_line(
            [1000.0, 1000.0, 1000.0],
            [5000.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
        )
        .unwrap();
        assert!((point[0] - 1000.0).abs() < 1e-12);
        assert_eq!(distance, 0.0);
    }
}

fn relative_difference(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = norm(subtract(a, b));
    if d == 0.0 {
        0.0
    } else {
        d / norm(a).max(norm(b))
    }
}
#[cfg(test)]
mod ellipse_precision_tests {
    #[test]
    fn axes_exact() {
        let mut differences = 0;
        for line in include_str!("../../tests/golden/ellipse_axes_precision.csv").lines() {
            let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
            let (a, b) = super::ellipse_axes([v[0], v[1], v[2]], [v[3], v[4], v[5]]);
            for (a, b) in a.into_iter().chain(b).zip(&v[6..]) {
                if a.to_bits() != b.to_bits() {
                    differences += 1;
                }
            }
        }
        assert_eq!(differences, 0);
    }
}
