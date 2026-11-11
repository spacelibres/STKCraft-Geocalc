// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::needless_range_loop)]
//! Point locations and the local topocentric frame.
//!
//! WebGeocalc can observe from, or target, a location that is not an
//! ephemeris object. `FIXED_POINT` is a position that is constant in a
//! reference frame relative to a centre body; `CONSTANT_VELOCITY_POINT`
//! adds a constant Cartesian velocity, and positions at other epochs are
//! extrapolated linearly from a reference epoch.
//!
//! Two consequences are easy to miss and are implemented here rather than
//! left to the caller:
//!
//! * The coordinates are given in a frame that may rotate, so the point's
//!   J2000 velocity is `R v_frame + R_dot r_frame`, exactly as a station on
//!   a rotating body moves with it.
//! * CSPICE evaluates the output frame at a locus the caller chooses
//!   (`frameLocus`). [`FrameLocus::Center`] is the traditional
//!   `spkezr` rule, [`FrameLocus::Observer`] samples the frame at the
//!   observation epoch, and [`FrameLocus::Target`] uses the target epoch
//!   with the light-time-rate factor. All three agree for an inertial
//!   frame, which is why the distinction only shows up with a point.
//!
//! Coordinate conversions follow the CSPICE inverses (`latrec`, `sphrec`,
//! `cylrec`, `radrec`, `georec`, `pgrrec`, `azlrec`). In particular,
//! `pgrrec` is `georec` with the body's longitude sense applied, so a
//! planetographic latitude is the geodetic latitude, which is what
//! `recpgr` returns here as well.

use crate::error::GeoError;
use crate::frame::{mat_mul, Frames, Mat3};
use crate::pck::Pck;
use crate::spk::{Aberration, CorrectedState, SpkKernel, State};

/// Coordinate system used to specify a location's position, matching the
/// WebGeocalc `coordinateRepresentation` values.
///
/// Angles are in radians, distances in kilometres, matching SPICE.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocationCoordinates {
    /// Cartesian coordinates.
    Rectangular {
        /// X coordinate, km.
        x: f64,
        /// Y coordinate, km.
        y: f64,
        /// Z coordinate, km.
        z: f64,
    },
    /// Range, right ascension, and declination.
    RightAscensionDeclination {
        /// Distance from the centre, km.
        range: f64,
        /// Right ascension, radians.
        right_ascension: f64,
        /// Declination, radians.
        declination: f64,
    },
    /// Planetocentric radius, longitude, and latitude (`latrec`).
    Latitudinal {
        /// Distance from the centre, km.
        radius: f64,
        /// Longitude, radians.
        longitude: f64,
        /// Planetocentric latitude, radians.
        latitude: f64,
    },
    /// Geodetic longitude, latitude, and altitude above the spheroid.
    Planetodetic {
        /// Longitude, radians.
        longitude: f64,
        /// Geodetic latitude, radians.
        latitude: f64,
        /// Altitude above the reference spheroid, km.
        altitude: f64,
    },
    /// Planetographic longitude and latitude with an altitude.
    ///
    /// CSPICE defines planetographic coordinates as geodetic ones with
    /// the body's longitude sense, so `latitude` is the geodetic value.
    Planetographic {
        /// Planetographic longitude, radians, in the body's own sense.
        longitude: f64,
        /// Planetographic latitude, radians.
        latitude: f64,
        /// Altitude above the reference spheroid, km.
        altitude: f64,
    },
    /// Cylindrical radius, longitude, and Z.
    Cylindrical {
        /// Distance from the Z axis, km.
        radius: f64,
        /// Longitude, radians.
        longitude: f64,
        /// Z coordinate, km.
        z: f64,
    },
    /// Radius, colatitude, and longitude (`sphrec`).
    Spherical {
        /// Distance from the centre, km.
        radius: f64,
        /// Colatitude, radians.
        colatitude: f64,
        /// Longitude, radians.
        longitude: f64,
    },
    /// Range, azimuth, and elevation, with the two WGC direction flags.
    AzimuthElevation {
        /// Distance from the origin, km.
        range: f64,
        /// Azimuth, radians.
        azimuth: f64,
        /// Elevation, radians.
        elevation: f64,
        /// Whether azimuth increases counterclockwise from +X.
        azimuth_counterclockwise: bool,
        /// Whether elevation increases towards +Z.
        elevation_positive_up: bool,
    },
}

impl LocationCoordinates {
    /// Converts to rectangular coordinates, kilometres.
    ///
    /// The planetodetic and planetographic forms need the centre body's
    /// radii (and, for planetographic, its longitude sense) from `pck`.
    pub fn to_rectangular(&self, pck: &Pck, center: i32) -> Result<[f64; 3], GeoError> {
        match *self {
            LocationCoordinates::Rectangular { x, y, z } => Ok([x, y, z]),
            LocationCoordinates::RightAscensionDeclination {
                range,
                right_ascension,
                declination,
            } => Ok(latrec(range, right_ascension, declination)),
            LocationCoordinates::Latitudinal {
                radius,
                longitude,
                latitude,
            } => Ok(latrec(radius, longitude, latitude)),
            LocationCoordinates::Cylindrical {
                radius,
                longitude,
                z,
            } => Ok([radius * longitude.cos(), radius * longitude.sin(), z]),
            LocationCoordinates::Spherical {
                radius,
                colatitude,
                longitude,
            } => Ok([
                radius * longitude.cos() * colatitude.sin(),
                radius * longitude.sin() * colatitude.sin(),
                radius * colatitude.cos(),
            ]),
            LocationCoordinates::AzimuthElevation {
                range,
                azimuth,
                elevation,
                azimuth_counterclockwise,
                elevation_positive_up,
            } => {
                let azimuth = if azimuth_counterclockwise {
                    azimuth
                } else {
                    -azimuth
                };
                let elevation = if elevation_positive_up {
                    elevation
                } else {
                    -elevation
                };
                Ok(latrec(range, azimuth, elevation))
            }
            LocationCoordinates::Planetodetic {
                longitude,
                latitude,
                altitude,
            } => planetodetic(longitude, latitude, altitude, pck, center),
            LocationCoordinates::Planetographic {
                longitude,
                latitude,
                altitude,
            } => {
                let east_positive = pck
                    .planetographic_east_positive(center)
                    .ok_or(GeoError::MissingPckData(center))?;
                let geodetic_longitude = if east_positive { longitude } else { -longitude };
                planetodetic(geodetic_longitude, latitude, altitude, pck, center)
            }
        }
    }
}

/// A location given as a point rather than as an ephemeris object.
///
/// `position` and `velocity` are expressed in `frame`, relative to the
/// centre body `center`, in kilometres and kilometres per second.
#[derive(Clone, Debug, PartialEq)]
pub struct PointLocation {
    /// NAIF id of the centre of motion, which must be in the SPK.
    pub center: i32,
    /// Frame the coordinates are expressed in: `J2000`, `ECLIPJ2000`, a
    /// frame-kernel frame, or `IAU_<naif-id>`.
    pub frame: String,
    /// Position at `epoch`, km.
    pub position: [f64; 3],
    /// Constant velocity in `frame`, km/s. Zero for a fixed point.
    pub velocity: [f64; 3],
    /// ET at which `position` is valid, seconds past J2000.
    pub epoch: f64,
}

impl PointLocation {
    /// A point that does not move relative to its centre.
    pub fn fixed(center: i32, frame: impl Into<String>, position: [f64; 3]) -> Self {
        Self {
            center,
            frame: frame.into(),
            position,
            velocity: [0.0; 3],
            epoch: 0.0,
        }
    }

    /// A point with a constant velocity, valid at `epoch`.
    pub fn constant_velocity(
        center: i32,
        frame: impl Into<String>,
        position: [f64; 3],
        velocity: [f64; 3],
        epoch: f64,
    ) -> Self {
        Self {
            center,
            frame: frame.into(),
            position,
            velocity,
            epoch,
        }
    }

    /// A point specified in one of the WebGeocalc coordinate systems.
    pub fn from_coordinates(
        center: i32,
        frame: impl Into<String>,
        coordinates: LocationCoordinates,
        pck: &Pck,
    ) -> Result<Self, GeoError> {
        Ok(Self::fixed(
            center,
            frame,
            coordinates.to_rectangular(pck, center)?,
        ))
    }

    /// Position relative to the centre at `et`, kilometres in `frame`.
    pub fn offset(&self, et: f64) -> [f64; 3] {
        let elapsed = et - self.epoch;
        add(self.position, scale(self.velocity, elapsed))
    }

    /// Position and velocity relative to the centre at `et`, in `frame`.
    pub fn offset_state(&self, et: f64) -> State {
        State {
            position: self.offset(et),
            velocity: self.velocity,
        }
    }
}

/// State of a point relative to the solar system barycenter in J2000.
///
/// The rotation is sampled at `et`, and the frame's angular rate enters
/// the velocity, so a station fixed on a rotating body carries the body's
/// motion.
pub fn point_state(
    kernel: &SpkKernel,
    frames: &Frames,
    pck: &Pck,
    location: &PointLocation,
    et: f64,
) -> Result<State, GeoError> {
    let center = kernel.state_in_frame(location.center, 0, et, frames, Some(pck), "J2000")?;
    let offset = point_offset_state(frames, pck, location, et)?;
    Ok(State {
        position: add(center.position, offset.position),
        velocity: add(center.velocity, offset.velocity),
    })
}

fn point_offset_state(
    frames: &Frames,
    pck: &Pck,
    location: &PointLocation,
    et: f64,
) -> Result<State, GeoError> {
    let rotation = frames.pxform_with_derivative(&location.frame, "J2000", et, Some(pck))?;
    Ok(transform_state(
        location.offset_state(et),
        &rotation.rotation,
        &rotation.rotation_dot,
        1.0,
    ))
}

// CSPICE MXVG adds all six products in column order, after scaling each
// derivative-block entry. Grouping the angular and linear sums changes bits.
fn transform_state(state: State, rotation: &Mat3, derivative: &Mat3, factor: f64) -> State {
    let position = rotate(rotation, state.position);
    let mut velocity = [0.0; 3];
    for (row, value) in velocity.iter_mut().enumerate() {
        for column in 0..3 {
            *value += (factor * derivative[row][column]) * state.position[column];
        }
        for column in 0..3 {
            *value += rotation[row][column] * state.velocity[column];
        }
    }
    State { position, velocity }
}

/// Which side of an observation is a point location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Endpoint<'a> {
    /// An ephemeris object, by NAIF id.
    Object(i32),
    /// A point location.
    Point(&'a PointLocation),
}

/// Where WebGeocalc evaluates the output frame's orientation.
///
/// All three agree when the output frame is inertial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameLocus {
    /// The traditional `spkezr` rule: the frame is sampled at the epoch at
    /// which the observer sees the frame's centre.
    Center,
    /// The frame is sampled at the observation epoch.
    Observer,
    /// The frame is sampled at the target epoch, with the light-time-rate
    /// factor applied to the rotating part of the velocity.
    Target,
}

impl FrameLocus {
    /// Parses a WebGeocalc `frameLocus` value.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "CENTER" | "CENTRE" => Some(Self::Center),
            "OBSERVER" => Some(Self::Observer),
            "TARGET" => Some(Self::Target),
            _ => None,
        }
    }
}

/// An output reference frame for an observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputFrame<'a> {
    /// A frame resolved through [`Frames`], including `IAU_<naif-id>`.
    Spice(&'a str),
    /// The local topocentric frame at a fixed-point observer.
    Topocentric(&'a PointLocation),
}

/// Apparent state of one endpoint as seen from another, in J2000.
///
/// Mirrors CSPICE `spkcpo`/`spkcpt`: exactly one endpoint may be a point,
/// which WebGeocalc enforces as well.
pub fn state_between(
    kernel: &SpkKernel,
    frames: &Frames,
    pck: &Pck,
    target: Endpoint<'_>,
    observer: Endpoint<'_>,
    et: f64,
    abcorr: Aberration,
) -> Result<CorrectedState, GeoError> {
    if matches!((target, observer), (Endpoint::Point(_), Endpoint::Point(_))) {
        return Err(GeoError::BadSearch(
            "at most one endpoint of an observation may be a point location",
        ));
    }

    let (use_light_time, converged, stellar, transmission) = abcorr.flags();
    if !use_light_time {
        let state = match (target, observer) {
            (Endpoint::Object(target), Endpoint::Object(observer)) => {
                kernel.state_in_frame(target, observer, et, frames, Some(pck), "J2000")?
            }
            (Endpoint::Object(target), Endpoint::Point(point)) => {
                let relative =
                    kernel.state_in_frame(target, point.center, et, frames, Some(pck), "J2000")?;
                let offset = point_offset_state(frames, pck, point, et)?;
                State {
                    position: subtract(relative.position, offset.position),
                    velocity: subtract(relative.velocity, offset.velocity),
                }
            }
            (Endpoint::Point(point), Endpoint::Object(observer)) => {
                let relative = kernel.state_in_frame(
                    point.center,
                    observer,
                    et,
                    frames,
                    Some(pck),
                    "J2000",
                )?;
                let offset = point_offset_state(frames, pck, point, et)?;
                State {
                    position: add(relative.position, offset.position),
                    velocity: add(relative.velocity, offset.velocity),
                }
            }
            _ => unreachable!(),
        };
        let distance = norm(state.position);
        let light_time = distance / crate::spk::CLIGHT;
        let light_time_rate = if distance == 0.0 {
            0.0
        } else {
            dot(state.position, state.velocity) / (crate::spk::CLIGHT * distance)
        };
        return Ok(CorrectedState {
            state,
            light_time,
            light_time_rate,
        });
    }
    let observer_state = endpoint_state(kernel, frames, pck, observer, et)?;
    let (mut relative, light_time, light_time_rate) = kernel.light_time_state_with(
        et,
        &observer_state,
        |epoch| endpoint_state(kernel, frames, pck, target, epoch),
        use_light_time,
        converged,
        transmission,
    )?;

    if stellar {
        let acceleration = endpoint_acceleration(kernel, frames, pck, observer, et)?;
        relative = SpkKernel::aberrate(
            transmission,
            acceleration,
            observer_state.velocity,
            &relative,
        );
    }

    Ok(CorrectedState {
        state: relative,
        light_time,
        light_time_rate,
    })
}

/// Computes and expresses a point observation with CSPICE's complete operation order.
///
/// Prefer this integrated operation to `state_between` followed by `express` when
/// comparing exact SPKCPT/SPKCPO/SPKCVT/SPKCVO values. For geometric observations,
/// CSPICE transforms the center-relative ephemeris and point offset independently
/// into the output frame before adding/subtracting them; rotating an already
/// subtracted J2000 state cannot reproduce that floating-point operation order.
#[allow(clippy::too_many_arguments)]
pub fn state_between_in(
    kernel: &SpkKernel,
    frames: &Frames,
    pck: &Pck,
    target: Endpoint<'_>,
    observer: Endpoint<'_>,
    frame: OutputFrame<'_>,
    locus: FrameLocus,
    et: f64,
    abcorr: Aberration,
) -> Result<CorrectedState, GeoError> {
    if matches!((target, observer), (Endpoint::Point(_), Endpoint::Point(_))) {
        return Err(GeoError::BadSearch(
            "at most one endpoint may be a point location",
        ));
    }
    if let OutputFrame::Topocentric(location) = frame {
        if !matches!(observer,Endpoint::Point(point) if point==location && point.velocity==[0.0;3])
        {
            return Err(GeoError::BadSearch(
                "topocentric output requires its fixed-point observer",
            ));
        }
        if frames.center(&location.frame)? != location.center {
            return Err(GeoError::BadSearch(
                "topocentric observer frame must be centered on the observer's center body",
            ));
        }
        let body = state_between_in(
            kernel,
            frames,
            pck,
            target,
            observer,
            OutputFrame::Spice(&location.frame),
            FrameLocus::Observer,
            et,
            abcorr,
        )?;
        let local = local_axes(location.offset(et), pck, location.center)?;
        return Ok(CorrectedState {
            state: State {
                position: rotate(&local, body.state.position),
                velocity: rotate(&local, body.state.velocity),
            },
            light_time: body.light_time,
            light_time_rate: body.light_time_rate,
        });
    }
    let OutputFrame::Spice(name) = frame else {
        unreachable!()
    };
    let geometric =
        |target, observer| kernel.state_in_frame(target, observer, et, frames, Some(pck), name);
    let offset = |point: &PointLocation| -> Result<State, GeoError> {
        let rotation = frames.pxform_with_derivative(&point.frame, name, et, Some(pck))?;
        Ok(transform_state(
            point.offset_state(et),
            &rotation.rotation,
            &rotation.rotation_dot,
            1.0,
        ))
    };
    let (light_time, converged, stellar, transmission) = abcorr.flags();
    if !light_time {
        let state = match (target, observer) {
            (Endpoint::Object(t), Endpoint::Object(o)) => geometric(t, o)?,
            (Endpoint::Object(t), Endpoint::Point(o)) => {
                let base = geometric(t, o.center)?;
                let point = offset(o)?;
                State {
                    position: subtract(base.position, point.position),
                    velocity: subtract(base.velocity, point.velocity),
                }
            }
            (Endpoint::Point(t), Endpoint::Object(o)) => {
                let base = geometric(t.center, o)?;
                let point = offset(t)?;
                State {
                    position: add(base.position, point.position),
                    velocity: add(base.velocity, point.velocity),
                }
            }
            _ => unreachable!(),
        };
        let distance = norm(state.position);
        return Ok(CorrectedState {
            state,
            light_time: distance / crate::spk::CLIGHT,
            light_time_rate: if distance == 0.0 {
                0.0
            } else {
                dot(state.position, state.velocity) / (distance * crate::spk::CLIGHT)
            },
        });
    }
    if locus == FrameLocus::Center && frames.is_inertial(name)? {
        let endpoint_at = |endpoint: Endpoint<'_>, epoch: f64| -> Result<State, GeoError> {
            match endpoint {
                Endpoint::Object(body) => {
                    kernel.state_in_frame(body, 0, epoch, frames, Some(pck), name)
                }
                Endpoint::Point(point) => {
                    let center =
                        kernel.state_in_frame(point.center, 0, epoch, frames, Some(pck), name)?;
                    let rotation =
                        frames.pxform_with_derivative(&point.frame, name, epoch, Some(pck))?;
                    let offset = transform_state(
                        point.offset_state(epoch),
                        &rotation.rotation,
                        &rotation.rotation_dot,
                        1.0,
                    );
                    Ok(State {
                        position: add(center.position, offset.position),
                        velocity: add(center.velocity, offset.velocity),
                    })
                }
            }
        };
        let observer_state = endpoint_at(observer, et)?;
        let (mut state, lt, rate) = kernel.light_time_state_with(
            et,
            &observer_state,
            |epoch| endpoint_at(target, epoch),
            true,
            converged,
            transmission,
        )?;
        if stellar {
            let before = endpoint_at(observer, et - 1.0)?.velocity;
            let after = endpoint_at(observer, et + 1.0)?.velocity;
            let acceleration = scale(subtract(after, before), 0.5);
            state =
                SpkKernel::aberrate(transmission, acceleration, observer_state.velocity, &state);
        }
        return Ok(CorrectedState {
            state,
            light_time: lt,
            light_time_rate: rate,
        });
    }
    let corrected = state_between(kernel, frames, pck, target, observer, et, abcorr)?;
    express(
        kernel,
        frames,
        pck,
        corrected,
        observer,
        frame,
        Some(frames.center(name)?),
        locus,
        et,
        abcorr,
    )
}

/// Expresses an apparent J2000 state in an output frame at a chosen locus.
///
/// `frame_center` is the NAIF id the frame is centred on when the locus is
/// [`FrameLocus::Center`]; it is ignored for the other loci and for a
/// topocentric frame, which is centred on the observer.
#[allow(clippy::too_many_arguments)]
pub fn express(
    kernel: &SpkKernel,
    frames: &Frames,
    pck: &Pck,
    corrected: CorrectedState,
    observer: Endpoint<'_>,
    frame: OutputFrame<'_>,
    frame_center: Option<i32>,
    locus: FrameLocus,
    et: f64,
    abcorr: Aberration,
) -> Result<CorrectedState, GeoError> {
    let (use_light_time, converged, _, transmission) = abcorr.flags();
    let sign = if transmission { 1.0 } else { -1.0 };

    let (epoch, factor) = if !use_light_time {
        // CSPICE evaluates the frame at ET whenever light time is off,
        // regardless of the requested locus.
        (et, 1.0)
    } else {
        match locus {
            FrameLocus::Observer => (et, 1.0),
            FrameLocus::Center => match frame_center {
                Some(center) => {
                    let observer_state = endpoint_state(kernel, frames, pck, observer, et)?;
                    let (relative, light_time, rate) = kernel.light_time_state_with(
                        et,
                        &observer_state,
                        |epoch| kernel.state_in_frame(center, 0, epoch, frames, Some(pck), "J2000"),
                        true,
                        converged,
                        transmission,
                    )?;
                    let _ = relative;
                    (et + sign * light_time, 1.0 + sign * rate)
                }
                None => (et, 1.0),
            },
            FrameLocus::Target => (
                et + sign * corrected.light_time,
                1.0 + sign * corrected.light_time_rate,
            ),
        }
    };

    if let OutputFrame::Topocentric(location) = frame {
        // AZLCPO evaluates SPKCPO in the observer's body frame at ET and
        // then performs two independent MXV operations into the local frame.
        // Multiplying the matrices first changes floating-point cancellation.
        let body = frames.pxform_with_derivative("J2000", &location.frame, et, Some(pck))?;
        let body_state = transform_state(corrected.state, &body.rotation, &body.rotation_dot, 1.0);
        let local = local_axes(location.offset(et), pck, location.center)?;
        return Ok(CorrectedState {
            state: State {
                position: rotate(&local, body_state.position),
                velocity: rotate(&local, body_state.velocity),
            },
            light_time: corrected.light_time,
            light_time_rate: corrected.light_time_rate,
        });
    }
    let (rotation, rotation_dot) = match frame {
        OutputFrame::Spice(name) => {
            // `pxform_with_derivative` maps its first frame into its
            // second, so the output frame is the target here.
            let rotation = frames.pxform_with_derivative("J2000", name, epoch, Some(pck))?;
            (rotation.rotation, rotation.rotation_dot)
        }
        OutputFrame::Topocentric(location) => topocentric_rotation(frames, pck, location, epoch)?,
    };

    Ok(CorrectedState {
        state: transform_state(corrected.state, &rotation, &rotation_dot, factor),
        light_time: corrected.light_time,
        light_time_rate: corrected.light_time_rate,
    })
}

fn endpoint_state(
    kernel: &SpkKernel,
    frames: &Frames,
    pck: &Pck,
    endpoint: Endpoint<'_>,
    et: f64,
) -> Result<State, GeoError> {
    match endpoint {
        Endpoint::Object(body) => kernel.state_in_frame(body, 0, et, frames, Some(pck), "J2000"),
        Endpoint::Point(location) => point_state(kernel, frames, pck, location, et),
    }
}

/// Numerically differentiated SSB acceleration of an endpoint.
///
/// CSPICE `zzspkfao` differentiates the observer's velocity with states
/// one second either side, and this reproduces that recipe for a point as
/// well as an object. The acceleration is only used by the transmission
/// stellar-aberration terms.
fn endpoint_acceleration(
    kernel: &SpkKernel,
    frames: &Frames,
    pck: &Pck,
    endpoint: Endpoint<'_>,
    et: f64,
) -> Result<[f64; 3], GeoError> {
    let before = endpoint_state(kernel, frames, pck, endpoint, et - 1.0)?;
    let after = endpoint_state(kernel, frames, pck, endpoint, et + 1.0)?;
    Ok(scale(subtract(after.velocity, before.velocity), 0.5))
}

/// Rotation from J2000 into the local topocentric frame at `location`.
///
/// The topocentric frame is fixed in the body-fixed frame of the centre
/// body, so its J2000 orientation rate is the body's.
pub fn topocentric_rotation(
    frames: &Frames,
    pck: &Pck,
    location: &PointLocation,
    et: f64,
) -> Result<(Mat3, Mat3), GeoError> {
    let (to_body, to_body_dot) = {
        let rotation = frames.pxform_with_derivative("J2000", &location.frame, et, Some(pck))?;
        (rotation.rotation, rotation.rotation_dot)
    };
    let local = local_axes(location.offset(et), pck, location.center)?;

    Ok((mat_mul(&local, &to_body), mat_mul(&local, &to_body_dot)))
}

/// Rotation from the body-fixed frame into the local topocentric frame.
///
/// `+Z` is the outward ellipsoid normal, `+X` is the body-fixed `+Z` axis
/// projected onto the local horizontal plane, so it points at the body's
/// north pole, and `+Y` completes the right-handed set. A point on the
/// body-fixed Z axis has no such projection, and there the body-fixed
/// `+X` axis is used instead, which is the special case WebGeocalc
/// documents.
fn local_axes(offset: [f64; 3], pck: &Pck, center: i32) -> Result<Mat3, GeoError> {
    let radii = pck.radii(center).ok_or(GeoError::MissingPckData(center))?;
    let [a, b, c] = radii;
    if a <= 0.0 || b <= 0.0 || c <= 0.0 {
        return Err(GeoError::BadSearch("ellipsoid axes must be positive"));
    }

    if offset[0] == 0.0 && offset[1] == 0.0 {
        if offset[2] < 0.0 {
            return Ok(crate::frame::rot1(std::f64::consts::PI));
        }
        return Ok(crate::frame::IDENTITY);
    }
    // AZLCPO always calls NEARPT, even for points nominally on the ellipsoid.
    let nearest = crate::surface::nearest_point_on_ellipsoid(offset, radii)?.point;
    // SURFNM scales by the smallest semiaxis before normalizing.
    let minimum = a.min(b).min(c);
    let ratios = [minimum / a, minimum / b, minimum / c];
    let normal = unit([
        nearest[0] * (ratios[0] * ratios[0]),
        nearest[1] * (ratios[1] * ratios[1]),
        nearest[2] * (ratios[2] * ratios[2]),
    ]);
    // TWOVEC(normal,3,+Z,1): two scaled unit cross products, then a
    // second normalization of the primary axis, matching the source.
    let y = unit_cross(normal, [0.0, 0.0, 1.0]);
    let x = unit_cross(y, normal);
    let z = unit(normal);
    Ok([x, y, z])
}

/// CSPICE `LATREC`: radius, longitude, latitude to rectangular.
fn latrec(radius: f64, longitude: f64, latitude: f64) -> [f64; 3] {
    [
        radius * longitude.cos() * latitude.cos(),
        radius * longitude.sin() * latitude.cos(),
        radius * latitude.sin(),
    ]
}

/// CSPICE `GEOREC`: geodetic longitude and latitude with an altitude.
fn planetodetic(
    longitude: f64,
    latitude: f64,
    altitude: f64,
    pck: &Pck,
    center: i32,
) -> Result<[f64; 3], GeoError> {
    let radii = pck.radii(center).ok_or(GeoError::MissingPckData(center))?;
    crate::surface::planetodetic_to_rectangular(latitude, longitude, altitude, radii)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: [f64; 3]) -> f64 {
    let big = a[0].abs().max(a[1].abs()).max(a[2].abs());
    if big == 0.0 {
        return 0.0;
    }
    let x = a[0] / big;
    let y = a[1] / big;
    let z = a[2] / big;
    big * (x * x + y * y + z * z).sqrt()
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let length = norm(a);
    if length == 0.0 {
        a
    } else {
        [a[0] / length, a[1] / length, a[2] / length]
    }
}

fn unit_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let ma = a[0].abs().max(a[1].abs()).max(a[2].abs());
    let mb = b[0].abs().max(b[1].abs()).max(b[2].abs());
    if ma == 0.0 || mb == 0.0 {
        return [0.0; 3];
    }
    unit(cross(
        [a[0] / ma, a[1] / ma, a[2] / ma],
        [b[0] / mb, b[1] / mb, b[2] / mb],
    ))
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

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn rotate(matrix: &Mat3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}
