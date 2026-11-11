//! WebGeocalc `directionVectorType` resolution.
//!
//! `SURFACE_INTERCEPT_POINT` and `TANGENT_POINT` both take a ray direction
//! rather than a plain direction-to-object. WebGeocalc accepts six forms
//! and this module turns any of them into a J2000 direction vector, which
//! is what CSPICE `sincpt`/`tangpt` consume as `DVEC`.
//!
//! | WebGeocalc `directionVectorType` | resolved from |
//! |---|---|
//! | `DIRECTION_TO_OBJECT` | apparent position of the object, optionally negated |
//! | `INSTRUMENT_BORESIGHT` | the instrument's `BORESIGHT`, rotated from its frame |
//! | `INSTRUMENT_FOV_BOUNDARY_VECTORS` | every FOV boundary vector, rotated |
//! | `REFERENCE_FRAME_AXIS` | one axis of a named frame, rotated |
//! | `VECTOR_IN_REFERENCE_FRAME` | an arbitrary vector in a named frame, rotated |
//! | `VECTOR_IN_INSTRUMENT_FOV` | an arbitrary vector in the instrument frame, rotated |
//!
//! Frames are resolved through [`Frames`]. The PCK-aware entry point supports
//! inertial, TKFRAME and body-fixed frames; unavailable models return errors.

use crate::error::GeoError;
use crate::fov::Fov;
use crate::frame::Frames;
use crate::spk::{Aberration, SpkKernel};
use crate::surface::{surface_intercept_geometry_in, SurfacePoint};
use crate::tangent::{tangent_point_in, CorrectionLocus, TangentPoint};

/// An axis of a reference frame, matching WebGeocalc
/// `directionFrameAxis`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameAxis {
    /// `+X`.
    X,
    /// `+Y`.
    Y,
    /// `+Z`.
    Z,
}

impl FrameAxis {
    /// Parses a WebGeocalc `directionFrameAxis` value.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "X" | "+X" | "X_AXIS" => Some(Self::X),
            "Y" | "+Y" | "Y_AXIS" => Some(Self::Y),
            "Z" | "+Z" | "Z_AXIS" => Some(Self::Z),
            _ => None,
        }
    }

    fn unit(self) -> [f64; 3] {
        match self {
            Self::X => [1.0, 0.0, 0.0],
            Self::Y => [0.0, 1.0, 0.0],
            Self::Z => [0.0, 0.0, 1.0],
        }
    }
}

/// A WebGeocalc `directionVectorType` specification.
#[derive(Clone, Debug, PartialEq)]
pub enum RayDirection {
    /// Direction to an ephemeris object, optionally reversed.
    ToObject {
        /// NAIF id of the object the ray points at.
        object: i32,
        /// Use the opposite direction (`antiVectorFlag`).
        anti: bool,
    },
    /// The instrument boresight. Requires a [`Fov`].
    InstrumentBoresight,
    /// Every instrument FOV boundary vector. Requires a [`Fov`].
    InstrumentFovBoundaryVectors,
    /// One axis of `frame`.
    FrameAxis {
        /// Frame the axis is expressed in.
        frame: String,
        /// Which axis.
        axis: FrameAxis,
        /// Use the opposite direction (`antiVectorFlag`).
        anti: bool,
    },
    /// An arbitrary vector in `frame`.
    VectorInFrame {
        /// Frame the vector is expressed in.
        frame: String,
        /// The vector, in `frame`.
        vector: [f64; 3],
        /// Use the opposite direction (`antiVectorFlag`).
        anti: bool,
        /// Apply stellar aberration to the vector's direction
        /// (`vectorAbCorr = STELLAR_ABERRATION_VECTOR`). WebGeocalc allows
        /// this only when `anti` is false.
        stellar_aberration: bool,
    },
    /// An arbitrary vector in the instrument frame. Requires a [`Fov`].
    VectorInFov {
        /// The vector, in the instrument's frame.
        vector: [f64; 3],
    },
}

/// Turns `direction` into J2000 ray directions at `et`.
///
/// Most specifications produce one direction;
/// [`RayDirection::InstrumentFovBoundaryVectors`] produces one per boundary
/// vector, in the order the instrument defines them.
#[allow(clippy::too_many_arguments)]
pub fn resolve_ray_directions(
    kernel: &SpkKernel,
    fov: Option<&Fov>,
    frames: &Frames,
    direction: &RayDirection,
    observer: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<Vec<[f64; 3]>, GeoError> {
    resolve_ray_directions_impl(kernel, fov, frames, None, direction, observer, et, abcorr)
}

/// Resolve surface-ray definitions including time-dependent IAU frames.
/// The direction frame uses the SINCPT/TANGPT DREF epoch: ET for inertial
/// or observer-centered frames; otherwise ET plus or minus the SPKEZP
/// light time between the frame center and observer.
#[allow(clippy::too_many_arguments)]
pub fn resolve_ray_directions_with_pck(
    kernel: &SpkKernel,
    fov: Option<&Fov>,
    frames: &Frames,
    pck: &crate::Pck,
    direction: &RayDirection,
    observer: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<Vec<[f64; 3]>, GeoError> {
    resolve_ray_directions_impl(
        kernel,
        fov,
        frames,
        Some(pck),
        direction,
        observer,
        et,
        abcorr,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_ray_directions_impl(
    kernel: &SpkKernel,
    fov: Option<&Fov>,
    frames: &Frames,
    pck: Option<&crate::Pck>,
    direction: &RayDirection,
    observer: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<Vec<[f64; 3]>, GeoError> {
    let instrument = || {
        fov.ok_or(GeoError::BadSearch(
            "this direction type requires an instrument FOV",
        ))
    };
    let to_j2000 = |frame: &str, vector: [f64; 3]| -> Result<[f64; 3], GeoError> {
        let (light_time, _, _, transmission) = abcorr.flags();
        let mut epoch = et;
        if light_time && !frames.is_inertial(frame)? {
            let center = frames.center(frame)?;
            if center != observer {
                let (_, lt) = kernel.position_corrected_in_frame(
                    center, observer, et, abcorr, frames, pck, "J2000",
                )?;
                epoch = et + if transmission { lt } else { -lt };
            }
        }
        Ok(rotate(
            &frames.pxform_in(frame, "J2000", epoch, pck)?,
            vector,
        ))
    };

    match direction {
        RayDirection::ToObject { object, anti } => {
            if *anti {
                // WebGeocalc: an anti-vector to an object cannot also carry
                // stellar aberration, because the correction has no defined
                // light-time direction.
                let (_, _, stellar, _) = abcorr.flags();
                if stellar {
                    return Err(GeoError::BadAberration(
                        "an anti-vector direction to an object cannot use stellar aberration"
                            .to_string(),
                    ));
                }
            }
            let position = kernel
                .position_corrected_in_frame(*object, observer, et, abcorr, frames, pck, "J2000")?
                .0;
            Ok(vec![if *anti { negate(position) } else { position }])
        }
        RayDirection::InstrumentBoresight => {
            let fov = instrument()?;
            Ok(vec![to_j2000(&fov.frame, fov.original_boresight())?])
        }
        RayDirection::InstrumentFovBoundaryVectors => {
            let fov = instrument()?;
            fov.boundary
                .iter()
                .map(|vector| to_j2000(&fov.frame, *vector))
                .collect()
        }
        RayDirection::VectorInFov { vector } => {
            let fov = instrument()?;
            Ok(vec![to_j2000(&fov.frame, *vector)?])
        }
        RayDirection::FrameAxis { frame, axis, anti } => {
            let vector = to_j2000(frame, axis.unit())?;
            Ok(vec![if *anti { negate(vector) } else { vector }])
        }
        RayDirection::VectorInFrame {
            frame,
            vector,
            anti,
            stellar_aberration,
        } => {
            if *stellar_aberration && *anti {
                return Err(GeoError::BadSearch(
                    "stellar aberration of the direction vector requires antiVectorFlag = false",
                ));
            }
            let mut resolved = to_j2000(frame, *vector)?;
            if *stellar_aberration {
                let (_, _, _, transmission) = abcorr.flags();
                let observer_state =
                    kernel.state_in_frame(observer, 0, et, frames, pck, "J2000")?;
                let velocity = if transmission {
                    negate(observer_state.velocity)
                } else {
                    observer_state.velocity
                };
                resolved = crate::surface::stellar_aberration_position(resolved, velocity);
            }
            Ok(vec![if *anti { negate(resolved) } else { resolved }])
        }
    }
}

/// Surface intercept points for a WebGeocalc direction specification.
///
/// Returns one [`SurfacePoint`] per resolved direction. An FOV boundary
/// specification therefore returns one point per boundary vector; a
/// direction that misses the body is reported as an error rather than
/// dropped, so the caller can tell the difference between "no intercept"
/// and "no directions".
#[allow(clippy::too_many_arguments)]
pub fn surface_intercept_points(
    kernel: &SpkKernel,
    pck: &crate::pck::Pck,
    fov: Option<&Fov>,
    frames: &Frames,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction: &RayDirection,
) -> Result<Vec<SurfacePoint>, GeoError> {
    surface_intercept_points_in(
        kernel,
        pck,
        fov,
        frames,
        &format!("IAU_{target}"),
        target,
        observer,
        et,
        abcorr,
        direction,
    )
}

/// Surface intercepts with an explicit target-centered shape frame.
#[allow(clippy::too_many_arguments)]
pub fn surface_intercept_points_in(
    kernel: &SpkKernel,
    pck: &crate::pck::Pck,
    fov: Option<&Fov>,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction: &RayDirection,
) -> Result<Vec<SurfacePoint>, GeoError> {
    let directions =
        resolve_ray_directions_with_pck(kernel, fov, frames, pck, direction, observer, et, abcorr)?;
    directions
        .into_iter()
        .map(|vector| {
            surface_intercept_geometry_in(
                kernel,
                pck,
                frames,
                target_frame,
                target,
                observer,
                et,
                abcorr,
                vector,
            )
            .map(|geometry| SurfacePoint {
                point: geometry.point,
                altitude: geometry.altitude,
            })
        })
        .collect()
}

/// Tangent points for a WebGeocalc direction specification.
#[allow(clippy::too_many_arguments)]
pub fn tangent_points(
    kernel: &SpkKernel,
    pck: &crate::pck::Pck,
    fov: Option<&Fov>,
    frames: &Frames,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction: &RayDirection,
) -> Result<Vec<TangentPoint>, GeoError> {
    tangent_points_in(
        kernel,
        pck,
        fov,
        frames,
        &format!("IAU_{target}"),
        target,
        observer,
        et,
        abcorr,
        direction,
        CorrectionLocus::TangentPoint,
    )
}

/// Tangent points in an explicit shape frame and correction locus.
#[allow(clippy::too_many_arguments)]
pub fn tangent_points_in(
    kernel: &SpkKernel,
    pck: &crate::pck::Pck,
    fov: Option<&Fov>,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction: &RayDirection,
    locus: CorrectionLocus,
) -> Result<Vec<TangentPoint>, GeoError> {
    let directions =
        resolve_ray_directions_with_pck(kernel, fov, frames, pck, direction, observer, et, abcorr)?;
    directions
        .into_iter()
        .map(|vector| {
            tangent_point_in(
                kernel,
                pck,
                frames,
                target_frame,
                target,
                observer,
                et,
                abcorr,
                vector,
                locus,
            )
        })
        .collect()
}

fn rotate(matrix: &crate::frame::Mat3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn negate(vector: [f64; 3]) -> [f64; 3] {
    [-vector[0], -vector[1], -vector[2]]
}
