//! Pointing direction from an observer to a target.
//!
//! Implements WebGeocalc POSITION, VELOCITY and VECTOR directions, corrected
//! frame epochs, FOV boundary rows, magnitude selection and all six coordinate
//! outputs. The compatibility wrappers return a unit POSITION direction.
//!
//! Semantics: JPL WebGeocalc official-api-info, DirectionSpecification and
//! POINTING_DIRECTION; numerical algorithms: CSPICE N0067 SPKEZP, SPKEZR,
//! PXFORM, STELAB, VHAT and the rectangular coordinate converters.

use crate::direction::{FrameAxis, RayDirection};
use crate::error::GeoError;
use crate::fov::Fov;
use crate::frame::Frames;
use crate::pck::Pck;
use crate::spk::{Aberration, SpkKernel};

/// Whether the direction is normalized or retains its physical magnitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointingMagnitude {
    /// Unit length (`UNIT`); the zero vector stays zero, as CSPICE `vhat`.
    Unit,
    /// Retain the original magnitude (`PRESERVE_ORIGINAL`).
    PreserveOriginal,
}

impl PointingMagnitude {
    /// Parse WebGeocalc's `vectorMagnitude` value.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "UNIT" => Some(Self::Unit),
            "PRESERVE_ORIGINAL" => Some(Self::PreserveOriginal),
            _ => None,
        }
    }
}

/// Pointing coordinate systems, with coordinates ordered as WebGeocalc prints them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointingRepresentation {
    /// X, Y, Z.
    Rectangular,
    /// Right ascension, declination, range.
    RightAscensionDeclination,
    /// Longitude, latitude, radius.
    Latitudinal,
    /// Longitude, Z, cylindrical radius.
    Cylindrical,
    /// Longitude, colatitude, radius.
    Spherical,
    /// Azimuth, elevation, range.
    AzimuthElevation {
        /// Counterclockwise azimuth about +Z.
        az_ccw: bool,
        /// Positive elevation towards +Z.
        el_plus_z: bool,
    },
}

impl PointingRepresentation {
    /// Parse WebGeocalc's `coordinateRepresentation` value. AZ_EL defaults to true/true.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "RECTANGULAR" => Some(Self::Rectangular),
            "RA_DEC" => Some(Self::RightAscensionDeclination),
            "LATITUDINAL" => Some(Self::Latitudinal),
            "CYLINDRICAL" => Some(Self::Cylindrical),
            "SPHERICAL" => Some(Self::Spherical),
            "AZ_EL" => Some(Self::AzimuthElevation {
                az_ccw: true,
                el_plus_z: true,
            }),
            _ => None,
        }
    }

    /// Output column identifiers, excluding the epoch and optional boundary number.
    pub fn output_ids(self) -> [&'static str; 3] {
        match self {
            Self::Rectangular => ["X", "Y", "Z"],
            Self::RightAscensionDeclination => ["RIGHT_ASCENSION", "DECLINATION", "RANGE"],
            Self::Latitudinal => ["LONGITUDE", "LATITUDE", "RADIUS"],
            Self::Cylindrical => ["LONGITUDE", "Z", "RADIUS"],
            Self::Spherical => ["LONGITUDE", "COLATITUDE", "RADIUS"],
            Self::AzimuthElevation { .. } => ["AZIMUTH", "ELEVATION", "RANGE"],
        }
    }
}

/// A rectangular pointing vector and its coordinate representation.
/// Angles in `coordinates` are radians; the order follows `output_ids`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointingColumns {
    /// Vector expressed in the requested output frame, after magnitude selection.
    pub vector: [f64; 3],
    /// Coordinates in WebGeocalc column order, angles in radians.
    pub coordinates: [f64; 3],
    /// Coordinate representation.
    pub representation: PointingRepresentation,
}

impl PointingColumns {
    /// WebGeocalc numeric output (angles in degrees).
    pub fn numeric_row(&self) -> [f64; 3] {
        let mut values = self.coordinates;
        if self.representation != PointingRepresentation::Rectangular {
            values[0] = values[0].to_degrees();
            if self.representation != PointingRepresentation::Cylindrical {
                values[1] = values[1].to_degrees();
            }
        }
        values
    }
}

/// Represent a vector with CSPICE's scaled coordinate algorithms.
///
/// Uses the operation order of `vhat`, `reclat`, `recrad`, `reccyl`,
/// `recsph` and `recazl` in N0067. Scaling avoids overflow/underflow.
/// Zero vectors follow those routines' zero-coordinate convention.
pub fn represent_pointing_vector(
    vector: [f64; 3],
    magnitude: PointingMagnitude,
    representation: PointingRepresentation,
) -> Result<PointingColumns, GeoError> {
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(GeoError::BadSearch("pointing vector must be finite"));
    }
    let vector = if magnitude == PointingMagnitude::Unit {
        unit(vector)
    } else {
        vector
    };
    let [x, y, z] = vector;
    let big = x.abs().max(y.abs()).max(z.abs());
    let (radius, longitude, latitude, colatitude) = if big > 0.0 {
        let a = x / big;
        let b = y / big;
        let c = z / big;
        let planar = (a * a + b * b).sqrt();
        (
            big * (a * a + b * b + c * c).sqrt(),
            if x == 0.0 && y == 0.0 {
                0.0
            } else {
                y.atan2(x)
            },
            c.atan2(planar),
            planar.atan2(c),
        )
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };
    let positive = |angle: f64| {
        if angle < 0.0 {
            angle + std::f64::consts::TAU
        } else {
            angle
        }
    };
    let coordinates = match representation {
        PointingRepresentation::Rectangular => vector,
        PointingRepresentation::Latitudinal => [longitude, latitude, radius],
        PointingRepresentation::RightAscensionDeclination => {
            [positive(longitude), latitude, radius]
        }
        PointingRepresentation::Spherical => [longitude, colatitude, radius],
        PointingRepresentation::Cylindrical => {
            let big = x.abs().max(y.abs());
            let (r, longitude) = if big == 0.0 {
                (0.0, 0.0)
            } else {
                let a = x / big;
                let b = y / big;
                (big * (a * a + b * b).sqrt(), positive(b.atan2(a)))
            };
            [longitude, z, r]
        }
        PointingRepresentation::AzimuthElevation { az_ccw, el_plus_z } => {
            let mut azimuth = positive(longitude);
            if !az_ccw && azimuth > 0.0 {
                azimuth = (std::f64::consts::TAU - azimuth).max(0.0);
            }
            let elevation = if !el_plus_z && latitude != 0.0 {
                -latitude
            } else {
                latitude
            };
            [azimuth, elevation, radius]
        }
    };
    if coordinates.iter().any(|value| !value.is_finite()) {
        return Err(GeoError::BadSearch("pointing coordinate overflow"));
    }
    Ok(PointingColumns {
        vector,
        coordinates,
        representation,
    })
}

/// Corrections supported by WebGeocalc's explicitly supplied vectors.
/// Light-time corrections affect the source-frame epoch; S/XS affect direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointingVectorCorrection {
    /// NONE.
    None,
    /// LT.
    LightTime,
    /// CN.
    ConvergedLightTime,
    /// XLT.
    TransmissionLightTime,
    /// XCN.
    TransmissionConvergedLightTime,
    /// Stellar aberration for reception, S.
    Stellar,
    /// Stellar aberration for transmission, XS.
    TransmissionStellar,
}

impl PointingVectorCorrection {
    /// Parse the seven vector correction strings accepted by WebGeocalc.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "NONE" => Some(Self::None),
            "LT" => Some(Self::LightTime),
            "CN" => Some(Self::ConvergedLightTime),
            "XLT" => Some(Self::TransmissionLightTime),
            "XCN" => Some(Self::TransmissionConvergedLightTime),
            "S" => Some(Self::Stellar),
            "XS" => Some(Self::TransmissionStellar),
            _ => None,
        }
    }

    fn frame_correction(self) -> Aberration {
        match self {
            Self::LightTime => Aberration::LightTime,
            Self::ConvergedLightTime => Aberration::ConvergedLightTime,
            Self::TransmissionLightTime => Aberration::TransmissionLightTime,
            Self::TransmissionConvergedLightTime => Aberration::TransmissionConvergedLightTime,
            _ => Aberration::None,
        }
    }
}

/// The three typed WebGeocalc direction specifications.
#[derive(Clone, Debug, PartialEq)]
pub enum PointingDirection {
    /// Apparent position of a target relative to an observer (SPKEZP).
    Position {
        /// Target NAIF id.
        target: i32,
        /// Observer NAIF id.
        observer: i32,
        /// State aberration correction.
        aberration: Aberration,
        /// Negate the corrected direction.
        anti: bool,
    },
    /// Velocity measured in `input_frame`, subsequently rotated as a vector.
    Velocity {
        /// Target NAIF id.
        target: i32,
        /// Observer NAIF id.
        observer: i32,
        /// Frame in which the source velocity is measured, including its rotation rate.
        input_frame: String,
        /// State aberration correction.
        aberration: Aberration,
        /// Negate the corrected direction.
        anti: bool,
    },
    /// Frame axis, arbitrary vector, boresight, or every FOV boundary vector.
    Vector {
        /// Vector specification. ToObject belongs to Position and is rejected here.
        direction: RayDirection,
        /// Observer NAIF id; used for frame light time and stellar aberration.
        observer: i32,
        /// Vector corrections, including standalone S/XS.
        aberration: PointingVectorCorrection,
        /// Negate the corrected direction. Applied after stellar aberration.
        anti: bool,
    },
}

/// Kernels and optional instrument data used by a pointing calculation.
pub struct PointingContext<'a> {
    /// Ephemeris kernel.
    pub kernel: &'a SpkKernel,
    /// Frame graph, including fixed FK frames.
    pub frames: &'a Frames,
    /// Body orientation models for body-fixed frames.
    pub pck: Option<&'a Pck>,
    /// Instrument FOV for boresight and boundary specifications.
    pub fov: Option<&'a Fov>,
}

/// One fully specified pointing calculation at one TDB epoch.
#[derive(Clone, Debug, PartialEq)]
pub struct PointingRequest {
    /// Observer epoch, TDB seconds past J2000.
    pub epoch: f64,
    /// Input direction specification.
    pub direction: PointingDirection,
    /// Output frame.
    pub output_frame: String,
    /// Unit or original magnitude.
    pub magnitude: PointingMagnitude,
    /// Output representation and optional AZ_EL flags.
    pub representation: PointingRepresentation,
}

/// One pointing result; FOV boundary requests return one per corner in IK order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointingResult {
    /// Original observer epoch.
    pub epoch: f64,
    /// Common epoch used for the frame orientations.
    pub frame_epoch: f64,
    /// One-based FOV corner index, or None for other direction types.
    pub boundary_point_number: Option<usize>,
    /// Direction and its coordinate representation.
    pub columns: PointingColumns,
}

/// Calculate all three WebGeocalc direction types, including corrected frame epochs.
///
/// POSITION uses the output frame's center light time. VELOCITY first computes
/// the velocity in its input frame, including the frame rate and light-time-rate
/// factor, and then rotates that velocity as a direction: no output-frame rate is
/// added. VELOCITY and VECTOR evaluate both frame orientations at the corrected
/// input-frame epoch. The latter distinction was verified with live WGC results
/// where the input and output frames have different centers.
pub fn pointing_calculation(
    context: &PointingContext<'_>,
    request: &PointingRequest,
) -> Result<Vec<PointingResult>, GeoError> {
    if !request.epoch.is_finite() {
        return Err(GeoError::BadSearch("pointing epoch must be finite"));
    }
    let et = request.epoch;
    let mut boundary = false;
    let (mut vectors, frame_epoch, anti) = match &request.direction {
        PointingDirection::Position {
            target,
            observer,
            aberration,
            anti,
        } => {
            let epoch = corrected_position_frame_epoch(
                context,
                &request.output_frame,
                *observer,
                et,
                *aberration,
            )?;
            let position = context
                .kernel
                .position_corrected_in_frame(
                    *target,
                    *observer,
                    et,
                    *aberration,
                    context.frames,
                    context.pck,
                    &request.output_frame,
                )?
                .0;
            (vec![position], epoch, *anti)
        }
        PointingDirection::Velocity {
            target,
            observer,
            input_frame,
            aberration,
            anti,
        } => {
            let (epoch, _) =
                corrected_frame_epoch(context, input_frame, *observer, et, *aberration)?;
            let velocity =
                state_in_inertial(context, *target, *observer, et, *aberration, input_frame)?
                    .velocity;
            let vector = if input_frame.eq_ignore_ascii_case(&request.output_frame) {
                velocity
            } else {
                let rotation = context.frames.pxform_in(
                    input_frame,
                    &request.output_frame,
                    epoch,
                    context.pck,
                )?;
                rotate(&rotation, velocity)
            };
            (vec![vector], epoch, *anti)
        }
        PointingDirection::Vector {
            direction,
            observer,
            aberration,
            anti,
        } => {
            let instrument = || {
                context.fov.ok_or(GeoError::BadSearch(
                    "pointing direction requires an instrument FOV",
                ))
            };
            let (frame, vectors, inner_anti, inner_stellar) = match direction {
                RayDirection::ToObject { .. } => {
                    return Err(GeoError::BadSearch(
                        "ToObject requires PointingDirection::Position",
                    ))
                }
                RayDirection::InstrumentBoresight => {
                    let fov = instrument()?;
                    (
                        fov.frame.as_str(),
                        vec![fov.original_boresight()],
                        false,
                        false,
                    )
                }
                RayDirection::InstrumentFovBoundaryVectors => {
                    let fov = instrument()?;
                    if fov.boundary.is_empty() {
                        return Err(GeoError::BadSearch("instrument FOV has no boundaries"));
                    }
                    boundary = true;
                    (fov.frame.as_str(), fov.boundary.clone(), false, false)
                }
                RayDirection::VectorInFov { vector } => {
                    (instrument()?.frame.as_str(), vec![*vector], false, false)
                }
                RayDirection::FrameAxis { frame, axis, anti } => {
                    let vector = match axis {
                        FrameAxis::X => [1.0, 0.0, 0.0],
                        FrameAxis::Y => [0.0, 1.0, 0.0],
                        FrameAxis::Z => [0.0, 0.0, 1.0],
                    };
                    (frame.as_str(), vec![vector], *anti, false)
                }
                RayDirection::VectorInFrame {
                    frame,
                    vector,
                    anti,
                    stellar_aberration,
                } => (frame.as_str(), vec![*vector], *anti, *stellar_aberration),
            };
            let epoch = corrected_position_frame_epoch(
                context,
                frame,
                *observer,
                et,
                aberration.frame_correction(),
            )?;
            let stellar = inner_stellar
                || matches!(
                    aberration,
                    PointingVectorCorrection::Stellar
                        | PointingVectorCorrection::TransmissionStellar
                );
            let vectors = if stellar {
                let to_j2000 = context
                    .frames
                    .pxform_in(frame, "J2000", epoch, context.pck)?;
                let to_output =
                    context
                        .frames
                        .pxform_in("J2000", &request.output_frame, epoch, context.pck)?;
                let mut velocity = context
                    .kernel
                    .state_in_frame(*observer, 0, et, context.frames, context.pck, "J2000")?
                    .velocity;
                if matches!(aberration, PointingVectorCorrection::TransmissionStellar) {
                    velocity = velocity.map(|value| -value);
                }
                vectors
                    .into_iter()
                    .map(|vector| {
                        rotate(
                            &to_output,
                            crate::surface::stellar_aberration_position(
                                rotate(&to_j2000, vector),
                                velocity,
                            ),
                        )
                    })
                    .collect()
            } else {
                let rotation =
                    context
                        .frames
                        .pxform_in(frame, &request.output_frame, epoch, context.pck)?;
                vectors
                    .into_iter()
                    .map(|vector| rotate(&rotation, vector))
                    .collect()
            };
            (vectors, epoch, *anti ^ inner_anti)
        }
    };
    if anti {
        for vector in &mut vectors {
            *vector = vector.map(|value| -value);
        }
    }
    vectors
        .into_iter()
        .enumerate()
        .map(|(index, vector)| {
            Ok(PointingResult {
                epoch: et,
                frame_epoch,
                boundary_point_number: boundary.then_some(index + 1),
                columns: represent_pointing_vector(
                    vector,
                    request.magnitude,
                    request.representation,
                )?,
            })
        })
        .collect()
}

// SPKACS evaluates observer acceleration and the light-time/stellar state
// in the requested inertial frame. Rotating a corrected J2000 state afterward
// changes both cancellation and the acceleration differencing in the final bits.
pub(crate) fn state_in_inertial(
    context: &PointingContext<'_>,
    target: i32,
    observer: i32,
    et: f64,
    correction: Aberration,
    frame: &str,
) -> Result<crate::spk::State, GeoError> {
    let empty = crate::pck::Pck::default();
    let pck = context.pck.unwrap_or(&empty);
    Ok(context
        .kernel
        .apparent_state_in(
            target,
            observer,
            et,
            correction,
            &crate::spk::FrameSource::Named {
                frames: context.frames,
                pck,
                frame_name: frame,
            },
        )?
        .state)
}

fn corrected_position_frame_epoch(
    context: &PointingContext<'_>,
    frame: &str,
    observer: i32,
    et: f64,
    correction: Aberration,
) -> Result<f64, GeoError> {
    let (light_time, _, _, transmission) = correction.flags();
    if !light_time || context.frames.is_inertial(frame)? {
        return Ok(et);
    }
    let center = context.frames.center(frame)?;
    if center == observer {
        return Ok(et);
    }
    let (_, lt) = context.kernel.position_corrected_in_frame(
        center,
        observer,
        et,
        correction,
        context.frames,
        context.pck,
        "J2000",
    )?;
    Ok(et + if transmission { lt } else { -lt })
}

fn corrected_frame_epoch(
    context: &PointingContext<'_>,
    frame: &str,
    observer: i32,
    et: f64,
    correction: Aberration,
) -> Result<(f64, f64), GeoError> {
    let (light_time, _, _, transmission) = correction.flags();
    if !light_time || context.frames.is_inertial(frame)? {
        return Ok((et, 1.0));
    }
    let center = context.frames.center(frame)?;
    if center == observer {
        return Ok((et, 1.0));
    }
    let empty = crate::pck::Pck::default();
    let corrected = context.kernel.apparent_state_in(
        center,
        observer,
        et,
        correction,
        &crate::spk::FrameSource::Named {
            frames: context.frames,
            pck: context.pck.unwrap_or(&empty),
            frame_name: "J2000",
        },
    )?;
    let sign = if transmission { 1.0 } else { -1.0 };
    Ok((
        et + sign * corrected.light_time,
        1.0 + sign * corrected.light_time_rate,
    ))
}

/// Unit pointing vector from `observer` to `target` at `et`, expressed
/// in `frame`.
///
/// `frame` may be `"J2000"` or any frame defined by the supplied
/// [`Frames`]. The target and observer positions include the requested
/// aberration correction.
pub fn pointing_direction(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: Option<&Frames>,
    frame: &str,
) -> Result<[f64; 3], GeoError> {
    if target == observer {
        return Err(GeoError::BadSearch("observer and target coincide"));
    }
    let defaults = Frames::new();
    let frames = frames.unwrap_or(&defaults);
    let context = PointingContext {
        kernel,
        frames,
        pck: None,
        fov: None,
    };
    let request = PointingRequest {
        epoch: et,
        direction: PointingDirection::Position {
            target,
            observer,
            aberration: abcorr,
            anti: false,
        },
        output_frame: if frame.trim().is_empty() {
            "J2000".into()
        } else {
            frame.trim().into()
        },
        magnitude: PointingMagnitude::Unit,
        representation: PointingRepresentation::Rectangular,
    };
    Ok(pointing_calculation(&context, &request)?[0].columns.vector)
}

/// Pointing direction as right ascension and declination in radians,
/// in the frame supplied to [`pointing_direction`].
pub fn pointing_ra_dec(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: Option<&Frames>,
    frame: &str,
) -> Result<[f64; 2], GeoError> {
    let direction = pointing_direction(kernel, target, observer, et, abcorr, frames, frame)?;
    let columns = represent_pointing_vector(
        direction,
        PointingMagnitude::PreserveOriginal,
        PointingRepresentation::RightAscensionDeclination,
    )?;
    Ok([columns.coordinates[0], columns.coordinates[1]])
}

fn rotate(matrix: &crate::frame::Mat3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn unit(vector: [f64; 3]) -> [f64; 3] {
    let big = vector[0].abs().max(vector[1].abs()).max(vector[2].abs());
    if big == 0.0 {
        return [0.0; 3];
    }
    let a = vector[0] / big;
    let b = vector[1] / big;
    let c = vector[2] / big;
    let length = big * (a * a + b * b + c * c).sqrt();
    [vector[0] / length, vector[1] / length, vector[2] / length]
}
