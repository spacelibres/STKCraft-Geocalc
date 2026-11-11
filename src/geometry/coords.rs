//! Cartesian and non-Cartesian coordinate systems used by the Geometry
//! Finder and by the sub-point family.
//!
//! The conversions follow CSPICE `reclat`, `recrad`, `recsph`, `recyl`,
//! and `recpgr`/`recgeo` for the ellipsoidal systems. Planetographic
//! latitude equals planetodetic latitude; only the longitude direction
//! differs, and it is east-positive for the Earth, Moon, and Sun exactly
//! as NAIF defines it.

use crate::error::GeoError;

/// A coordinate system supported by the Geometry Finder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateSystem {
    /// `X`, `Y`, `Z` in kilometres.
    Rectangular,
    /// `RADIUS`, `LONGITUDE` in `(-pi, pi]`, `LATITUDE` in
    /// `[-pi/2, pi/2]`.
    Latitudinal,
    /// `RANGE`, `RIGHT ASCENSION` in `[0, 2*pi)`, `DECLINATION`.
    RightAscensionDeclination,
    /// `RADIUS`, `COLATITUDE` in `[0, pi]`, `LONGITUDE` in `(-pi, pi]`.
    Spherical,
    /// `RADIUS`, `LONGITUDE` in `[0, 2*pi)`, `Z`.
    Cylindrical,
    /// `AZIMUTH`, `ELEVATION`, `RANGE`, with WGC flag conventions.
    AzimuthElevation {
        /// Azimuth increases counterclockwise from +X when true.
        az_ccw: bool,
        /// Elevation is positive toward +Z when true.
        el_plus_z: bool,
    },
    /// `LONGITUDE`, `LATITUDE`, `ALTITUDE`; latitude is the
    /// planetodetic latitude of an oblate or prolate spheroid.
    Geodetic,
    /// `LONGITUDE`, `LATITUDE`, `ALTITUDE`; longitude is east-positive
    /// for the Earth, Moon, and Sun.
    Planetographic,
}

impl CoordinateSystem {
    /// Parses a CSPICE coordinate-system name, case-insensitively.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_uppercase().as_str() {
            "RECTANGULAR" => Some(Self::Rectangular),
            "LATITUDINAL" => Some(Self::Latitudinal),
            "RA/DEC" | "RADEC" => Some(Self::RightAscensionDeclination),
            "SPHERICAL" => Some(Self::Spherical),
            "CYLINDRICAL" => Some(Self::Cylindrical),
            "AZ_EL" | "AZ/EL" | "AZIMUTH/ELEVATION" => Some(Self::AzimuthElevation {
                az_ccw: true,
                el_plus_z: true,
            }),
            "GEODETIC" => Some(Self::Geodetic),
            "PLANETOGRAPHIC" => Some(Self::Planetographic),
            _ => None,
        }
    }

    /// The canonical coordinate names and their index.
    pub fn coordinate_index(self, name: &str) -> Option<usize> {
        let name = name.trim().to_ascii_uppercase();
        let names: [&str; 3] = match self {
            Self::Rectangular => ["X", "Y", "Z"],
            Self::Latitudinal => ["RADIUS", "LONGITUDE", "LATITUDE"],
            Self::RightAscensionDeclination => ["RANGE", "RIGHT ASCENSION", "DECLINATION"],
            Self::Spherical => ["RADIUS", "COLATITUDE", "LONGITUDE"],
            Self::Cylindrical => ["RADIUS", "LONGITUDE", "Z"],
            Self::AzimuthElevation { .. } => ["AZIMUTH", "ELEVATION", "RANGE"],
            Self::Geodetic | Self::Planetographic => ["LONGITUDE", "LATITUDE", "ALTITUDE"],
        };
        names.iter().position(|candidate| *candidate == name)
    }
}

/// Converts a body-fixed or inertial Cartesian vector to `system`.
///
/// `radii` is required for the ellipsoidal systems and must satisfy
/// `a == b`; the triaxial planetodetic definition is not implemented.
/// `body` is used only to choose the planetographic longitude
/// direction; see [`planetographic_east_positive`].
pub fn to_coordinates(
    vector: [f64; 3],
    system: CoordinateSystem,
    radii: Option<[f64; 3]>,
    body: i32,
) -> Result<[f64; 3], GeoError> {
    to_coordinates_with_direction(
        vector,
        system,
        radii,
        planetographic_east_positive(body, None),
    )
}

/// Converts a Cartesian vector to `system` with an explicit planetographic
/// longitude direction.
///
/// `planetographic_east` is ignored for every system except
/// [`CoordinateSystem::Planetographic`].
pub fn to_coordinates_with_direction(
    vector: [f64; 3],
    system: CoordinateSystem,
    radii: Option<[f64; 3]>,
    planetographic_east: bool,
) -> Result<[f64; 3], GeoError> {
    let [x, y, z] = vector;
    let [radius, longitude, latitude] = rectangular_latitudinal(vector);
    match system {
        CoordinateSystem::Rectangular => Ok(vector),
        CoordinateSystem::Latitudinal => Ok([radius, longitude, latitude]),
        CoordinateSystem::RightAscensionDeclination => Ok([
            radius,
            if longitude < 0.0 {
                longitude + std::f64::consts::TAU
            } else {
                longitude
            },
            latitude,
        ]),
        CoordinateSystem::Spherical => {
            let big = x.abs().max(y.abs()).max(z.abs());
            let colatitude = if big > 0.0 {
                let (a, b, c) = (x / big, y / big, z / big);
                (a * a + b * b).sqrt().atan2(c)
            } else {
                0.0
            };
            Ok([radius, colatitude, longitude])
        }
        CoordinateSystem::Cylindrical => {
            let big = x.abs().max(y.abs());
            let (planar, lon) = if big == 0.0 {
                (0.0, 0.0)
            } else {
                let (a, b) = (x / big, y / big);
                (big * (a * a + b * b).sqrt(), b.atan2(a))
            };
            Ok([
                planar,
                if lon < 0.0 {
                    lon + std::f64::consts::TAU
                } else {
                    lon
                },
                z,
            ])
        }
        CoordinateSystem::AzimuthElevation { az_ccw, el_plus_z } => {
            // RECAZL first calls RECRAD, then complements positive RA.
            let mut azimuth = if longitude < 0.0 {
                longitude + std::f64::consts::TAU
            } else {
                longitude
            };
            if !az_ccw && azimuth > 0.0 {
                azimuth = (std::f64::consts::TAU - azimuth).max(0.0);
            }
            let elevation = if !el_plus_z && latitude != 0.0 {
                -latitude
            } else {
                latitude
            };
            Ok([azimuth, elevation, radius])
        }
        CoordinateSystem::Geodetic | CoordinateSystem::Planetographic => {
            let radii = radii.ok_or(GeoError::BadSearch(
                "ellipsoidal coordinates require body radii",
            ))?;
            let [a, b, _c] = radii;
            if a != b {
                return Err(GeoError::BadSearch(
                    "ellipsoidal coordinates require a == b",
                ));
            }
            let [longitude, latitude, altitude] = rectangular_to_planetodetic(vector, radii)?;
            if system == CoordinateSystem::Geodetic {
                Ok([longitude, latitude, altitude])
            } else {
                // Planetographic longitude is positive east for the Earth,
                // Moon, and Sun, and follows the body's spin sense
                // otherwise (NAIF `recpgr`).
                let signed = if planetographic_east {
                    longitude
                } else {
                    -longitude
                };
                Ok([
                    (if signed < 0.0 {
                        signed + std::f64::consts::TAU
                    } else {
                        signed
                    })
                    .clamp(0.0, std::f64::consts::TAU),
                    latitude,
                    altitude,
                ])
            }
        }
    }
}

/// The `index`th coordinate of `vector` in `system`.
pub fn coordinate_value(
    vector: [f64; 3],
    system: CoordinateSystem,
    index: usize,
    radii: Option<[f64; 3]>,
    body: i32,
) -> Result<f64, GeoError> {
    let coordinates = to_coordinates(vector, system, radii, body)?;
    coordinates
        .get(index)
        .copied()
        .ok_or(GeoError::BadSearch("coordinate index out of range"))
}

/// Whether planetographic longitude increases to the east for `body`,
/// following CSPICE `recpgr`.
///
/// The Earth, Moon, and Sun are exceptions: their planetographic
/// longitude is positive east. For every other body the direction follows
/// the spin sense, which CSPICE takes from the sign of the first-degree
/// term of the body's prime-meridian polynomial: retrograde (negative)
/// spin is positive east, prograde is positive west.
///
/// Pass `prime_meridian_rate` to resolve the spin sense exactly. Without
/// it, only the Earth/Moon/Sun exception is applied and every other body
/// is reported positive west, so a retrograde body such as Venus (299) or
/// Uranus (799) needs the rate. [`crate::pck::Pck::planetographic_east_positive`]
/// reads it from a text PCK.
pub fn planetographic_east_positive(body: i32, prime_meridian_rate: Option<f64>) -> bool {
    if matches!(body, 10 | 301 | 399) {
        return true;
    }
    prime_meridian_rate.map(|rate| rate < 0.0).unwrap_or(false)
}

/// Body-fixed rectangular to planetodetic longitude, latitude, and
/// altitude for an oblate or prolate spheroid.
pub fn rectangular_to_planetodetic(point: [f64; 3], radii: [f64; 3]) -> Result<[f64; 3], GeoError> {
    let [a, b, c] = radii;
    if !a.is_finite() || !b.is_finite() || !c.is_finite() || a <= 0.0 || c <= 0.0 {
        return Err(GeoError::BadSearch(
            "ellipsoid axes must be finite and positive",
        ));
    }
    if a != b {
        return Err(GeoError::BadSearch(
            "planetodetic conversion requires equatorial axes a == b",
        ));
    }
    // RECGEO derives the polar semi-axis from RE and F, then uses NEARPT.
    let flattening = (a - c) / a;
    let polar = a - flattening * a;
    let radii = [a, a, polar];
    let base = crate::surface::nearest_point_on_ellipsoid(point, radii)?;
    let minimum = a.min(polar);
    let normal: [f64; 3] = std::array::from_fn(|i| {
        let r = minimum / radii[i];
        base.point[i] * (r * r)
    });
    let length = crate::surface::norm(normal);
    let normal = normal.map(|x| x / length);
    let [_, _, latitude] = rectangular_latitudinal(normal);
    let longitude = if point[0] == 0.0 && point[1] == 0.0 {
        0.0
    } else {
        point[1].atan2(point[0])
    };
    Ok([longitude, latitude, base.altitude])
}

// N0067 RECLAT, including zero vectors and overflow/underflow scaling.
fn rectangular_latitudinal(vector: [f64; 3]) -> [f64; 3] {
    let big = vector[0].abs().max(vector[1].abs()).max(vector[2].abs());
    if big <= 0.0 {
        return [0.0; 3];
    }
    let [x, y, z] = vector.map(|x| x / big);
    let radius = big * (x * x + y * y + z * z).sqrt();
    let latitude = z.atan2((x * x + y * y).sqrt());
    let longitude = if vector[0] == 0.0 && vector[1] == 0.0 {
        0.0
    } else {
        vector[1].atan2(vector[0])
    };
    [radius, longitude, latitude]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn close(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn latitudinal_axes() {
        let origin = [0.0, 0.0, 0.0];
        assert_eq!(
            to_coordinates(origin, CoordinateSystem::Latitudinal, None, 399).unwrap(),
            [0.0; 3]
        );

        let x = to_coordinates([1.0, 0.0, 0.0], CoordinateSystem::Latitudinal, None, 399).unwrap();
        assert!(close(x[0], 1.0, 1e-15));
        assert!(close(x[1], 0.0, 1e-15));
        assert!(close(x[2], 0.0, 1e-15));

        let y = to_coordinates([0.0, 1.0, 0.0], CoordinateSystem::Latitudinal, None, 399).unwrap();
        assert!(close(y[1], PI / 2.0, 1e-15));

        let z = to_coordinates([0.0, 0.0, 1.0], CoordinateSystem::Latitudinal, None, 399).unwrap();
        assert!(close(z[2], PI / 2.0, 1e-15));
    }

    #[test]
    fn planetodetic_round_trip() {
        let radii = [6378.1366, 6378.1366, 6356.7519];
        let point = [4000.0, -3000.0, 2500.0];
        let [lon, lat, alt] = rectangular_to_planetodetic(point, radii).unwrap();
        let rebuilt = crate::surface::planetodetic_to_rectangular(lat, lon, alt, radii).unwrap();
        for axis in 0..3 {
            assert!(close(point[axis], rebuilt[axis], 1e-8));
        }
    }
}

/// Extract a coordinate using the loaded PCK's planetographic longitude convention.
/// Missing longitude metadata is an error for planetographic coordinates.
pub fn coordinate_value_with_pck(
    vector: [f64; 3],
    system: CoordinateSystem,
    index: usize,
    radii: Option<[f64; 3]>,
    body: i32,
    pck: &crate::pck::Pck,
) -> Result<f64, GeoError> {
    let east = if system == CoordinateSystem::Planetographic {
        pck.planetographic_east_positive(body)
            .ok_or(GeoError::MissingPckData(body))?
    } else {
        false
    };
    let coordinates = to_coordinates_with_direction(vector, system, radii, east)?;
    coordinates
        .get(index)
        .copied()
        .ok_or(GeoError::BadSearch("coordinate index out of range"))
}

#[cfg(test)]
mod exact_coordinate_golden {
    use super::*;
    #[test]
    fn exact_cspice_rectangular_coordinate_conversions() {
        let mut mismatches = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_coordinates.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let number = |i: usize| f[i].parse::<f64>().unwrap();
            let actual = to_coordinates_with_direction(
                [number(2), number(3), number(4)],
                if f[0].starts_with("AZEL_") {
                    CoordinateSystem::AzimuthElevation {
                        az_ccw: f[0].as_bytes()[5] == b'1',
                        el_plus_z: f[0].as_bytes()[7] == b'1',
                    }
                } else {
                    CoordinateSystem::parse(f[0]).unwrap()
                },
                Some([number(5), number(5), number(6)]),
                f[1] == "1",
            );
            let expected = [number(7), number(8), number(9)];
            match actual {
                Ok(a) if a == expected => {}
                other => mismatches.push(format!("{line}: {other:?}")),
            };
            count += 1;
        }
        assert!(
            mismatches.is_empty(),
            "{} mismatches/{count} cases:\n{}",
            mismatches.len(),
            mismatches
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[cfg(test)]
mod planetographic_metadata {
    use super::*;
    #[test]
    fn native_recpgr_missing_pm_and_overrides() {
        let mut count = 0;
        for line in include_str!("../../tests/golden/planetographic_metadata.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let body = f[0].parse().unwrap();
            let mut text = String::from("\\begindata\n");
            if !f[1].is_empty() {
                text += &format!("BODY{body}_PM = ({})\n", f[1].replace(';', " "));
            }
            if f[2] != "NONE" {
                text += &format!("BODY{body}_PGR_POSITIVE_LON = '{}'\n", f[2]);
            }
            let pck = crate::pck::Pck::from_text(&text).unwrap();
            for component in 0..3 {
                let result = coordinate_value_with_pck(
                    [3., 4., 5.],
                    CoordinateSystem::Planetographic,
                    component,
                    Some([1.; 3]),
                    body,
                    &pck,
                );
                if f[3].starts_with("SPICE(") {
                    assert!(
                        matches!(result, Err(GeoError::MissingPckData(_))),
                        "{line}:actual{result:?}"
                    );
                } else {
                    let expected: Vec<f64> = f[3].split(';').map(|x| x.parse().unwrap()).collect();
                    assert_eq!(result.unwrap(), expected[component], "{line}");
                }
            }
            count += 1;
        }
        assert_eq!(count, 72);
    }
    #[test]
    fn gf_planetographic_does_not_guess_missing_moon_pm() {
        use crate::{Condition, FinderOptions, Frames, Interval, Pck, SpkKernel};
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::from_text("\\begindata\nBODY301_RADII=(1737.4 1737.4 1737.4)\n").unwrap();
        let frames=Frames::from_text("\\begindata\nFRAME_PM_TEST=1900301\nFRAME_1900301_NAME='PM_TEST'\nFRAME_1900301_CLASS=4\nFRAME_1900301_CLASS_ID=1900301\nFRAME_1900301_CENTER=301\nTKFRAME_1900301_RELATIVE='J2000'\nTKFRAME_1900301_SPEC='ANGLES'\nTKFRAME_1900301_ANGLES=(0 0 0)\nTKFRAME_1900301_AXES=(1 2 3)\n").unwrap();
        for component in 0..3 {
            let result = crate::finder::find_position_coordinate_with_radii(
                &kernel,
                &pck,
                301,
                399,
                crate::Aberration::None,
                &frames,
                "PM_TEST",
                CoordinateSystem::Planetographic,
                component,
                &[Interval { start: 0., end: 0. }],
                Condition::LessThan,
                1.,
                FinderOptions::default(),
            );
            assert!(
                matches!(result, Err(GeoError::MissingPckData(301))),
                "{component}:{result:?}"
            );
        }
    }
}
