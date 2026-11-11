//! State-vector output representations, matching the WebGeocalc
//! `STATE_VECTOR` calculation and CSPICE `spkezr` + `recrad`/`reclat`/
//! `recsph`/`recyl`/`recpgr`/`recgeo`.
//!
//! WebGeocalc orders the state-vector columns per representation, which is
//! *not* the component order used by the Geometry Finder coordinate
//! systems in [`crate::coords`]. The orders below were read off the live
//! WGC API on 2026-11-11 and are reproduced in [`StateRepresentation::output_ids`]:
//!
//! | Representation | Columns after `DATE` |
//! |---|---|
//! | `RECTANGULAR` | `DISTANCE`, `SPEED`, `X`, `Y`, `Z`, `D_X_DT`, `D_Y_DT`, `D_Z_DT` |
//! | `RA_DEC` | `RIGHT_ASCENSION`, `DECLINATION`, `RANGE`, `D_RIGHT_ASCENSION_DT`, `D_DECLINATION_DT`, `D_RANGE_DT`, `SPEED` |
//! | `LATITUDINAL` | `LONGITUDE`, `LATITUDE`, `RADIUS`, ... |
//! | `CYLINDRICAL` | `LONGITUDE`, `Z`, `RADIUS`, ... |
//! | `SPHERICAL` | `LONGITUDE`, `COLATITUDE`, `RADIUS`, ... |
//! | `AZ_EL` | `AZIMUTH`, `ELEVATION`, `RANGE`, ... |
//! | `PLANETODETIC`, `PLANETOGRAPHIC` | `LONGITUDE`, `LATITUDE`, `ALTITUDE`, ... |
//!
//! Every representation then ends with `TIME_AT_TARGET` and `LIGHT_TIME`.
//!
//! Angles are radians in the Rust API and radians per second for the
//! rates; [`StateColumns::numeric_row`] converts to the degrees and
//! degrees per second that WebGeocalc prints.
//!
//! The rate columns are analytic derivatives of the coordinate map, not
//! finite differences. For the ellipsoidal representations they follow
//! from differentiating the planetodetic back-substitution
//!
//! ```text
//! p      = (N + h) cos(phi)
//! z      = (N (1 - e^2) + h) sin(phi)
//! N(phi) = a / sqrt(1 - e^2 sin^2(phi))
//! ```
//!
//! which gives a 2x2 linear system for `(dphi/dt, dh/dt)` once `dp/dt` and
//! `dz/dt` are known from the Cartesian velocity.

use crate::coords::rectangular_to_planetodetic;
use crate::error::GeoError;
use crate::pck::Pck;
use crate::spk::{Aberration, State};

/// A WebGeocalc state-vector representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateRepresentation {
    /// `X`, `Y`, `Z` in kilometres, and their rates as `km/s`.
    Rectangular,
    /// `RIGHT_ASCENSION` in `[0, 2*pi)`, `DECLINATION`, `RANGE`.
    RightAscensionDeclination,
    /// `LONGITUDE` in `(-pi, pi]`, `LATITUDE`, `RADIUS`.
    Latitudinal,
    /// `LONGITUDE` in `[0, 2*pi)`, `Z`, `RADIUS`.
    Cylindrical,
    /// `LONGITUDE` in `(-pi, pi]`, `COLATITUDE`, `RADIUS`.
    Spherical,
    /// `AZIMUTH`, `ELEVATION`, `RANGE`, with the WGC sign flags.
    AzimuthElevation {
        /// Azimuth increases counterclockwise from `+X` when true.
        az_ccw: bool,
        /// Elevation is positive toward `+Z` when true.
        el_plus_z: bool,
    },
    /// `LONGITUDE` in `(-pi, pi]` (east positive), `LATITUDE`, `ALTITUDE`.
    Planetodetic,
    /// `LONGITUDE` in `[0, 2*pi)`, `LATITUDE`, `ALTITUDE`; the longitude
    /// direction follows the body's spin sense.
    Planetographic,
}

impl StateRepresentation {
    /// Parses a WebGeocalc `stateRepresentation` value.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_uppercase().as_str() {
            "RECTANGULAR" => Some(Self::Rectangular),
            "RA_DEC" | "RA/DEC" | "RIGHT_ASCENSION_DECLINATION" => {
                Some(Self::RightAscensionDeclination)
            }
            "LATITUDINAL" => Some(Self::Latitudinal),
            "CYLINDRICAL" => Some(Self::Cylindrical),
            "SPHERICAL" => Some(Self::Spherical),
            "AZ_EL" | "AZ/EL" => Some(Self::AzimuthElevation {
                az_ccw: true,
                el_plus_z: true,
            }),
            "PLANETODETIC" => Some(Self::Planetodetic),
            "PLANETOGRAPHIC" => Some(Self::Planetographic),
            _ => None,
        }
    }

    /// The representation with explicit `AZ_EL` flags.
    pub fn azimuth_elevation(az_ccw: bool, el_plus_z: bool) -> Self {
        Self::AzimuthElevation { az_ccw, el_plus_z }
    }

    /// The WebGeocalc column identifiers, `DATE` first.
    pub fn output_ids(self) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = vec!["DATE"];
        match self {
            Self::Rectangular => ids.extend([
                "DISTANCE", "SPEED", "X", "Y", "Z", "D_X_DT", "D_Y_DT", "D_Z_DT",
            ]),
            Self::RightAscensionDeclination => ids.extend([
                "RIGHT_ASCENSION",
                "DECLINATION",
                "RANGE",
                "D_RIGHT_ASCENSION_DT",
                "D_DECLINATION_DT",
                "D_RANGE_DT",
                "SPEED",
            ]),
            Self::Latitudinal => ids.extend([
                "LONGITUDE",
                "LATITUDE",
                "RADIUS",
                "D_LONGITUDE_DT",
                "D_LATITUDE_DT",
                "D_RADIUS_DT",
                "SPEED",
            ]),
            Self::Cylindrical => ids.extend([
                "LONGITUDE",
                "Z",
                "RADIUS",
                "D_LONGITUDE_DT",
                "D_Z_DT",
                "D_RADIUS_DT",
                "SPEED",
            ]),
            Self::Spherical => ids.extend([
                "LONGITUDE",
                "COLATITUDE",
                "RADIUS",
                "D_LONGITUDE_DT",
                "D_COLATITUDE_DT",
                "D_RADIUS_DT",
                "SPEED",
            ]),
            Self::AzimuthElevation { .. } => ids.extend([
                "AZIMUTH",
                "ELEVATION",
                "RANGE",
                "D_AZIMUTH_DT",
                "D_ELEVATION_DT",
                "D_RANGE_DT",
                "SPEED",
            ]),
            Self::Planetodetic | Self::Planetographic => ids.extend([
                "LONGITUDE",
                "LATITUDE",
                "ALTITUDE",
                "D_LONGITUDE_DT",
                "D_LATITUDE_DT",
                "D_ALTITUDE_DT",
                "SPEED",
            ]),
        }
        ids.push("TIME_AT_TARGET");
        ids.push("LIGHT_TIME");
        ids
    }

    /// Whether component `index` of the coordinate triple is an angle.
    fn angle_mask(self) -> [bool; 3] {
        match self {
            Self::Rectangular => [false, false, false],
            Self::Cylindrical => [true, false, false],
            Self::RightAscensionDeclination
            | Self::Latitudinal
            | Self::Spherical
            | Self::AzimuthElevation { .. }
            | Self::Planetodetic
            | Self::Planetographic => [true, true, false],
        }
    }
}

/// One WebGeocalc state-vector row.
///
/// `coordinates` and `rates` are in the representation's own component
/// order (see [`StateRepresentation::output_ids`]), with angles in radians
/// and angular rates in radians per second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateColumns {
    /// Epoch of the row, TDB seconds past J2000.
    pub date: f64,
    /// The representation that produced this row.
    pub representation: StateRepresentation,
    /// The three coordinate components.
    pub coordinates: [f64; 3],
    /// The three coordinate rates, radians/s for angular components.
    pub rates: [f64; 3],
    /// `|r|` in kilometres; WGC's `DISTANCE`/`RANGE`/`RADIUS` column for
    /// the non-ellipsoidal representations.
    pub distance: f64,
    /// `|v|` in `km/s`.
    pub speed: f64,
    /// Observer-target light time in seconds.
    pub light_time: f64,
    /// The epoch at which the target state was evaluated.
    pub time_at_target: f64,
}

impl StateColumns {
    /// The numeric columns in WebGeocalc order, excluding `DATE`, with
    /// angles in degrees and angular rates in degrees per second.
    ///
    /// `TIME_AT_TARGET` is a date column in WebGeocalc; here it is emitted
    /// as TDB seconds past J2000 so the row stays numeric.
    pub fn numeric_row(&self) -> Vec<f64> {
        let angles = self.representation.angle_mask();
        let mut coordinates = self.coordinates;
        let mut rates = self.rates;
        for index in 0..3 {
            if angles[index] {
                coordinates[index] = coordinates[index].to_degrees();
                rates[index] = rates[index].to_degrees();
            }
        }

        let mut row: Vec<f64> = Vec::with_capacity(10);
        match self.representation {
            StateRepresentation::Rectangular => {
                row.push(self.distance);
                row.push(self.speed);
                row.extend_from_slice(&coordinates);
                row.extend_from_slice(&rates);
            }
            _ => {
                row.extend_from_slice(&coordinates);
                row.extend_from_slice(&rates);
                row.push(self.speed);
            }
        }
        row.push(self.time_at_target);
        row.push(self.light_time);
        row
    }
}

/// The WebGeocalc `TIME_AT_TARGET` column: the epoch at which the target
/// state was evaluated.
///
/// Reception corrections (`LT`, `LT+S`, `CN`, `CN+S`) evaluate the target
/// one light time in the past, transmission corrections (`X...`) one light
/// time in the future, and `NONE` evaluates it at the requested epoch even
/// though the geometric light time is still reported.
pub fn time_at_target(date: f64, light_time: f64, abcorr: Aberration) -> f64 {
    let (use_light_time, _, _, transmission) = abcorr.flags();
    if !use_light_time {
        return date;
    }
    if transmission {
        date + light_time
    } else {
        date - light_time
    }
}

/// Converts a state into the WebGeocalc column set.
///
/// `state` must already be expressed in the requested reference frame.
/// `body` selects the planetographic longitude direction: positive east
/// for the Earth, Moon, and Sun, and positive west otherwise. Use
/// [`state_columns_with_pck`] to derive the direction from the body's
/// prime-meridian rate instead.
pub fn state_columns(
    date: f64,
    state: State,
    representation: StateRepresentation,
    light_time: f64,
    abcorr: Aberration,
    radii: Option<[f64; 3]>,
    body: i32,
) -> Result<StateColumns, GeoError> {
    let east = crate::coords::planetographic_east_positive(body, None);
    build(date, state, representation, light_time, abcorr, radii, east)
}

/// Converts a state into the WebGeocalc column set, taking the body radii
/// and the planetographic longitude direction from a text PCK.
pub fn state_columns_with_pck(
    date: f64,
    state: State,
    representation: StateRepresentation,
    light_time: f64,
    abcorr: Aberration,
    pck: &Pck,
    body: i32,
) -> Result<StateColumns, GeoError> {
    // Only the ellipsoidal representations need radii, so the lookup is
    // lazy: a rectangular or spherical request works without a PCK entry
    // for `body`.
    let radii = match representation {
        StateRepresentation::Planetodetic | StateRepresentation::Planetographic => {
            Some(pck.radii(body).ok_or(GeoError::BadSearch(
                "the PCK has no radii for the requested body",
            ))?)
        }
        _ => pck.radii(body),
    };
    let east = pck
        .planetographic_east_positive(body)
        .unwrap_or_else(|| crate::coords::planetographic_east_positive(body, None));
    build(date, state, representation, light_time, abcorr, radii, east)
}

fn build(
    date: f64,
    state: State,
    representation: StateRepresentation,
    light_time: f64,
    abcorr: Aberration,
    radii: Option<[f64; 3]>,
    planetographic_east: bool,
) -> Result<StateColumns, GeoError> {
    let position = state.position;
    let velocity = state.velocity;
    let radius = norm(position);
    if radius == 0.0 {
        return Err(GeoError::BadSearch("zero-length position vector"));
    }
    let (coordinates, rates) = exact_coordinates_and_rates(
        position,
        velocity,
        representation,
        radii,
        planetographic_east,
    )?;

    Ok(StateColumns {
        date,
        representation,
        coordinates,
        rates,
        distance: radius,
        speed: norm(velocity),
        light_time,
        time_at_target: time_at_target(date, light_time, abcorr),
    })
}

// CSPICE XFMSTA obtains rates by inverting the orthogonal columns of
// DRDLAT/DRDCYL/DRDSPH/DRDGEO, then multiplying by velocity with MXV.
// Closed-form quotient simplifications are not bit-equivalent.
fn exact_coordinates_and_rates(
    position: [f64; 3],
    velocity: [f64; 3],
    representation: StateRepresentation,
    radii: Option<[f64; 3]>,
    east: bool,
) -> Result<([f64; 3], [f64; 3]), GeoError> {
    use crate::pointing::{
        represent_pointing_vector, PointingMagnitude, PointingRepresentation as P,
    };
    if representation == StateRepresentation::Rectangular {
        return Ok((position, velocity));
    }
    if position[0] == 0.0 && position[1] == 0.0 {
        if velocity[0] != 0.0
            || velocity[1] != 0.0
            || matches!(representation, StateRepresentation::AzimuthElevation { .. })
        {
            return Err(GeoError::BadSearch(
                "coordinate rate is undefined on the Z axis",
            ));
        }
        // XFMSTA defines a special axial-state branch, bypassing the
        // singular Jacobian when both position and velocity lie on Z.
        let rep = match representation {
            StateRepresentation::RightAscensionDeclination => Some(P::RightAscensionDeclination),
            StateRepresentation::Latitudinal => Some(P::Latitudinal),
            StateRepresentation::Cylindrical => Some(P::Cylindrical),
            StateRepresentation::Spherical => Some(P::Spherical),
            _ => None,
        };
        let coordinates = if let Some(rep) = rep {
            represent_pointing_vector(position, PointingMagnitude::PreserveOriginal, rep)?
                .coordinates
        } else {
            let mut values = rectangular_to_planetodetic(
                position,
                radii.ok_or(GeoError::BadSearch("body radii required"))?,
            )?;
            if representation == StateRepresentation::Planetographic && !east {
                values[0] = -values[0];
                if values[0] < 0.0 {
                    values[0] += std::f64::consts::TAU;
                }
            }
            values
        };
        let rates = if representation == StateRepresentation::Cylindrical {
            [0.0, velocity[2], 0.0]
        } else {
            [0.0, 0.0, velocity[2]]
        };
        return Ok((coordinates, rates));
    }
    let mut signs = [1.0; 3];
    let (coordinates, columns, order) = match representation {
        StateRepresentation::RightAscensionDeclination
        | StateRepresentation::Latitudinal
        | StateRepresentation::AzimuthElevation { .. } => {
            let lat = represent_pointing_vector(
                position,
                PointingMagnitude::PreserveOriginal,
                P::Latitudinal,
            )?
            .coordinates;
            let [lon, phi, r] = lat;
            let columns = [
                [lon.cos() * phi.cos(), lon.sin() * phi.cos(), phi.sin()],
                [-r * lon.sin() * phi.cos(), r * lon.cos() * phi.cos(), 0.0],
                [
                    -r * lon.cos() * phi.sin(),
                    -r * lon.sin() * phi.sin(),
                    r * phi.cos(),
                ],
            ];
            let rep = match representation {
                StateRepresentation::RightAscensionDeclination => P::RightAscensionDeclination,
                StateRepresentation::AzimuthElevation { az_ccw, el_plus_z } => {
                    if !az_ccw {
                        signs[1] = -1.0;
                    }
                    if !el_plus_z {
                        signs[2] = -1.0;
                    }
                    P::AzimuthElevation { az_ccw, el_plus_z }
                }
                _ => P::Latitudinal,
            };
            (
                represent_pointing_vector(position, PointingMagnitude::PreserveOriginal, rep)?
                    .coordinates,
                columns,
                [1, 2, 0],
            )
        }
        StateRepresentation::Cylindrical => {
            let coords = represent_pointing_vector(
                position,
                PointingMagnitude::PreserveOriginal,
                P::Cylindrical,
            )?
            .coordinates;
            let [lon, _, r] = coords;
            (
                coords,
                [
                    [lon.cos(), lon.sin(), 0.0],
                    [-lon.sin() * r, lon.cos() * r, 0.0],
                    [0.0, 0.0, 1.0],
                ],
                [1, 2, 0],
            )
        }
        StateRepresentation::Spherical => {
            let coords = represent_pointing_vector(
                position,
                PointingMagnitude::PreserveOriginal,
                P::Spherical,
            )?
            .coordinates;
            let [lon, theta, r] = coords;
            let ct = theta.cos();
            let st = theta.sin();
            let cl = lon.cos();
            let sl = lon.sin();
            (
                coords,
                [
                    [cl * st, sl * st, ct],
                    [r * cl * ct, r * sl * ct, -r * st],
                    [-r * sl * st, r * cl * st, 0.0],
                ],
                [2, 1, 0],
            )
        }
        StateRepresentation::Planetodetic | StateRepresentation::Planetographic => {
            let radii = radii.ok_or(GeoError::BadSearch(
                "ellipsoidal coordinates require body radii",
            ))?;
            let [lon, lat, alt] = rectangular_to_planetodetic(position, radii)?;
            let re = radii[0];
            let f = (radii[0] - radii[2]) / radii[0];
            let clat = lat.cos();
            let clon = lon.cos();
            let slat = lat.sin();
            let slon = lon.sin();
            let flat = 1.0 - f;
            let flat2 = flat * flat;
            let g = (clat * clat + flat2 * slat * slat).sqrt();
            let g2 = g * g;
            let dg = (flat2 - 1.0) * slat * clat / g;
            let columns = [
                [
                    -(alt + re / g) * slon * clat,
                    (alt + re / g) * clon * clat,
                    0.0,
                ],
                [
                    -re * dg / g2 * clon * clat - (alt + re / g) * clon * slat,
                    -re * dg / g2 * slon * clat - (alt + re / g) * slon * slat,
                    -flat2 * re * dg / g2 * slat + (alt + flat2 * re / g) * clat,
                ],
                [clon * clat, slon * clat, slat],
            ];
            let longitude = if representation == StateRepresentation::Planetographic {
                if !east {
                    signs[0] = -1.0;
                }
                let signed = if east { lon } else { -lon };
                if signed < 0.0 {
                    signed + std::f64::consts::TAU
                } else {
                    signed
                }
            } else {
                lon
            };
            ([longitude, lat, alt], columns, [0, 1, 2])
        }
        StateRepresentation::Rectangular => unreachable!(),
    };
    let mut derivatives = [0.0; 3];
    for i in 0..3 {
        let column = columns[i];
        let length = norm(column);
        if length == 0.0 {
            return Err(GeoError::BadSearch("singular coordinate Jacobian"));
        }
        let inverse = 1.0 / length;
        let row = [
            (column[0] / length) * inverse * signs[i],
            (column[1] / length) * inverse * signs[i],
            (column[2] / length) * inverse * signs[i],
        ];
        derivatives[i] = row[0] * velocity[0] + row[1] * velocity[1] + row[2] * velocity[2];
    }
    Ok((
        coordinates,
        [
            derivatives[order[0]],
            derivatives[order[1]],
            derivatives[order[2]],
        ],
    ))
}

fn norm(vector: [f64; 3]) -> f64 {
    let big = vector[0].abs().max(vector[1].abs()).max(vector[2].abs());
    if big == 0.0 {
        return 0.0;
    }
    let x = vector[0] / big;
    let y = vector[1] / big;
    let z = vector[2] / big;
    big * (x * x + y * y + z * z).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn spherical_rates_match_finite_differences() {
        let position = [1234.5, -2345.6, 3456.7];
        let velocity = [0.7, -1.1, 0.3];
        let dt = 1e-3;
        let advanced = [
            position[0] + velocity[0] * dt,
            position[1] + velocity[1] * dt,
            position[2] + velocity[2] * dt,
        ];
        let state = State { position, velocity };

        for representation in [
            StateRepresentation::RightAscensionDeclination,
            StateRepresentation::Latitudinal,
            StateRepresentation::Cylindrical,
            StateRepresentation::Spherical,
            StateRepresentation::azimuth_elevation(false, false),
        ] {
            let columns =
                state_columns(0.0, state, representation, 0.0, Aberration::None, None, 0).unwrap();
            let advanced_columns = state_columns(
                0.0,
                State {
                    position: advanced,
                    velocity,
                },
                representation,
                0.0,
                Aberration::None,
                None,
                0,
            )
            .unwrap();

            for index in 0..3 {
                let difference =
                    (advanced_columns.coordinates[index] - columns.coordinates[index]) / dt;
                let tolerance = 1e-4 * columns.rates[index].abs().max(1e-9);
                assert!(
                    close(difference, columns.rates[index], tolerance),
                    "{representation:?} component {index}: finite difference {difference:e} analytic {:e}",
                    columns.rates[index]
                );
            }
        }
    }

    #[test]
    fn planetodetic_rates_match_finite_differences() {
        let radii = [6378.1366, 6378.1366, 6356.7519];
        let position = [4000.0, -3000.0, 2500.0];
        let velocity = [0.4, -0.9, 0.6];
        let dt = 1e-3;
        let advanced = [
            position[0] + velocity[0] * dt,
            position[1] + velocity[1] * dt,
            position[2] + velocity[2] * dt,
        ];

        let columns = state_columns(
            0.0,
            State { position, velocity },
            StateRepresentation::Planetodetic,
            0.0,
            Aberration::None,
            Some(radii),
            399,
        )
        .unwrap();
        let advanced_columns = state_columns(
            0.0,
            State {
                position: advanced,
                velocity,
            },
            StateRepresentation::Planetodetic,
            0.0,
            Aberration::None,
            Some(radii),
            399,
        )
        .unwrap();

        for index in 0..3 {
            let difference =
                (advanced_columns.coordinates[index] - columns.coordinates[index]) / dt;
            let tolerance = 1e-4 * columns.rates[index].abs().max(1e-9);
            assert!(
                close(difference, columns.rates[index], tolerance),
                "component {index}: finite difference {difference:e} analytic {:e}",
                columns.rates[index]
            );
        }
    }

    #[test]
    fn rectangular_reports_velocity_components() {
        let state = State {
            position: [1.0, 2.0, 3.0],
            velocity: [0.1, 0.2, 0.3],
        };
        let columns = state_columns(
            0.0,
            state,
            StateRepresentation::Rectangular,
            0.0,
            Aberration::None,
            None,
            0,
        )
        .unwrap();
        assert_eq!(columns.coordinates, [1.0, 2.0, 3.0]);
        assert_eq!(columns.rates, [0.1, 0.2, 0.3]);
        assert!(close(columns.distance, 14.0_f64.sqrt(), 1e-14));
    }

    #[test]
    fn output_ids_match_webgeocalc() {
        // Column identifiers read off the live WebGeocalc `STATE_VECTOR`
        // API on 2026-11-11; the order is part of the contract.
        let expected: [(StateRepresentation, &[&str]); 8] = [
            (
                StateRepresentation::Rectangular,
                &[
                    "DISTANCE",
                    "SPEED",
                    "X",
                    "Y",
                    "Z",
                    "D_X_DT",
                    "D_Y_DT",
                    "D_Z_DT",
                    "TIME_AT_TARGET",
                    "LIGHT_TIME",
                ],
            ),
            (
                StateRepresentation::RightAscensionDeclination,
                &[
                    "RIGHT_ASCENSION",
                    "DECLINATION",
                    "RANGE",
                    "D_RIGHT_ASCENSION_DT",
                    "D_DECLINATION_DT",
                    "D_RANGE_DT",
                    "SPEED",
                    "TIME_AT_TARGET",
                    "LIGHT_TIME",
                ],
            ),
            (
                StateRepresentation::Latitudinal,
                &[
                    "LONGITUDE",
                    "LATITUDE",
                    "RADIUS",
                    "D_LONGITUDE_DT",
                    "D_LATITUDE_DT",
                    "D_RADIUS_DT",
                    "SPEED",
                    "TIME_AT_TARGET",
                    "LIGHT_TIME",
                ],
            ),
            (
                StateRepresentation::Cylindrical,
                &[
                    "LONGITUDE",
                    "Z",
                    "RADIUS",
                    "D_LONGITUDE_DT",
                    "D_Z_DT",
                    "D_RADIUS_DT",
                    "SPEED",
                    "TIME_AT_TARGET",
                    "LIGHT_TIME",
                ],
            ),
            (
                StateRepresentation::Spherical,
                &[
                    "LONGITUDE",
                    "COLATITUDE",
                    "RADIUS",
                    "D_LONGITUDE_DT",
                    "D_COLATITUDE_DT",
                    "D_RADIUS_DT",
                    "SPEED",
                    "TIME_AT_TARGET",
                    "LIGHT_TIME",
                ],
            ),
            (
                StateRepresentation::azimuth_elevation(true, true),
                &[
                    "AZIMUTH",
                    "ELEVATION",
                    "RANGE",
                    "D_AZIMUTH_DT",
                    "D_ELEVATION_DT",
                    "D_RANGE_DT",
                    "SPEED",
                    "TIME_AT_TARGET",
                    "LIGHT_TIME",
                ],
            ),
            (
                StateRepresentation::Planetodetic,
                &[
                    "LONGITUDE",
                    "LATITUDE",
                    "ALTITUDE",
                    "D_LONGITUDE_DT",
                    "D_LATITUDE_DT",
                    "D_ALTITUDE_DT",
                    "SPEED",
                    "TIME_AT_TARGET",
                    "LIGHT_TIME",
                ],
            ),
            (
                StateRepresentation::Planetographic,
                &[
                    "LONGITUDE",
                    "LATITUDE",
                    "ALTITUDE",
                    "D_LONGITUDE_DT",
                    "D_LATITUDE_DT",
                    "D_ALTITUDE_DT",
                    "SPEED",
                    "TIME_AT_TARGET",
                    "LIGHT_TIME",
                ],
            ),
        ];

        for (representation, ids) in expected {
            let mut wanted = vec!["DATE"];
            wanted.extend(ids);
            assert_eq!(representation.output_ids(), wanted, "{representation:?}");
        }
    }

    #[test]
    fn time_at_target_follows_aberration() {
        assert!(close(
            time_at_target(100.0, 2.0, Aberration::None),
            100.0,
            0.0
        ));
        assert!(close(
            time_at_target(100.0, 2.0, Aberration::LightTime),
            98.0,
            0.0
        ));
        assert!(close(
            time_at_target(100.0, 2.0, Aberration::TransmissionLightTime),
            102.0,
            0.0
        ));
    }

    #[test]
    fn numeric_row_matches_wgc_column_count() {
        let state = State {
            position: [1.0, 2.0, 3.0],
            velocity: [0.1, 0.2, 0.3],
        };
        for representation in [
            StateRepresentation::Rectangular,
            StateRepresentation::RightAscensionDeclination,
            StateRepresentation::Latitudinal,
            StateRepresentation::Cylindrical,
            StateRepresentation::Spherical,
            StateRepresentation::azimuth_elevation(true, true),
            StateRepresentation::Planetodetic,
            StateRepresentation::Planetographic,
        ] {
            let columns = state_columns(
                0.0,
                state,
                representation,
                0.0,
                Aberration::None,
                Some([1.0, 1.0, 1.0]),
                399,
            )
            .unwrap();
            assert_eq!(
                columns.numeric_row().len() + 1,
                representation.output_ids().len()
            );
        }
    }
}
