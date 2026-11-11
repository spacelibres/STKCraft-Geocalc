// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::excessive_precision)]
//! Osculating orbital elements from a state vector.
//!
//! This follows CSPICE `oscelt`: the elements are defined with respect to
//! the J2000 equator (the `+z` axis) and the `+x` axis toward the
//! ascending node reference direction, and angles are returned in
//! radians. `mean_anomaly` and `true_anomaly` are also reported, matching
//! the WebGeocalc Osculating Elements table.

use std::f64::consts::{PI, TAU};

use crate::error::GeoError;
use crate::pck::Pck;
use crate::spk::{Aberration, SpkKernel, State};

/// Osculating elements for an elliptic, parabolic, or hyperbolic orbit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OsculatingElements {
    /// Perifocal distance (km).
    pub perifocal_distance: f64,
    /// Eccentricity.
    pub eccentricity: f64,
    /// Inclination (radians).
    pub inclination: f64,
    /// Longitude of the ascending node (radians).
    pub ascending_node: f64,
    /// Argument of periapsis (radians).
    pub argument_of_periapsis: f64,
    /// Mean anomaly (radians).
    pub mean_anomaly: f64,
    /// True anomaly (radians).
    pub true_anomaly: f64,
    /// Semi-major axis (km). Positive for ellipses; zero if not computable.
    pub semi_major_axis: f64,
    /// Orbital period in seconds; zero for non-elliptic or non-computable cases.
    pub period: f64,
    /// Gravitational parameter used (km^3/s^2).
    pub gm: f64,
    /// Range between the bodies (km).
    pub range: f64,
    /// Relative speed (km/s).
    pub speed: f64,
}

/// Source of the gravitational parameter for osculating elements.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GmSource {
    /// GM of the center body from the loaded constants kernel.
    Center,
    /// Sum of the center and orbiting bodies' GMs from the kernel.
    CenterOrbiter,
    /// Caller-supplied GM in km^3/s^2.
    Custom(f64),
}
impl GmSource {
    /// Parse the WebGeocalc GM specification mode.
    pub fn parse(mode: &str, custom: Option<f64>) -> Result<Self, GeoError> {
        match mode.trim().to_ascii_uppercase().as_str() {
            "CENTER" => Ok(Self::Center),
            "CENTER_ORBITER" => Ok(Self::CenterOrbiter),
            "CUSTOM" => custom
                .map(Self::Custom)
                .ok_or(GeoError::BadSearch("custom GM is required")),
            _ => Err(GeoError::BadSearch("unknown GM specification")),
        }
    }
    fn value(self, pck: &Pck, center: i32, orbiter: i32) -> Result<f64, GeoError> {
        let gm = match self {
            Self::Center => pck.gm(center).ok_or(GeoError::MissingPckData(center))?,
            Self::CenterOrbiter => {
                pck.gm(center).ok_or(GeoError::MissingPckData(center))?
                    + pck.gm(orbiter).ok_or(GeoError::MissingPckData(orbiter))?
            }
            Self::Custom(gm) => gm,
        };
        if !gm.is_finite() || gm <= 0.0 {
            return Err(GeoError::BadSearch(
                "gravitational parameter must be finite and positive",
            ));
        }
        Ok(gm)
    }
}

/// CSPICE OSCLTX elements in a built-in inertial frame, with explicit GM mode.
#[allow(clippy::too_many_arguments)]
pub fn osculating_elements_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &crate::frame::Frames,
    orbiting: i32,
    center: i32,
    et: f64,
    abcorr: Aberration,
    frame: &str,
    gm_source: GmSource,
) -> Result<OsculatingElements, GeoError> {
    if !frames.is_inertial(frame)? {
        return Err(GeoError::BadSearch(
            "orbital elements require a built-in inertial frame",
        ));
    }
    let gm = gm_source.value(pck, center, orbiting)?;
    let context = crate::pointing::PointingContext {
        kernel,
        frames,
        pck: Some(pck),
        fov: None,
    };
    let state = crate::pointing::state_in_inertial(&context, orbiting, center, et, abcorr, frame)?;
    elements_from_state(&state, gm)
}

/// Osculating elements of `orbiting` relative to `center` at `et`.
///
/// The gravitational parameter is taken from the loaded PCK for
/// `center`.
pub fn osculating_elements(
    kernel: &SpkKernel,
    pck: &Pck,
    orbiting: i32,
    center: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<OsculatingElements, GeoError> {
    let gm = pck.gm(center).ok_or(GeoError::MissingPckData(center))?;
    let corrected = kernel.state_corrected(orbiting, center, et, abcorr)?;
    elements_from_state(&corrected.state, gm)
}

/// Converts a state into osculating elements using CSPICE `oscelt`
/// conventions.
pub fn elements_from_state(state: &State, gm: f64) -> Result<OsculatingElements, GeoError> {
    if gm <= 0.0 || !gm.is_finite() {
        return Err(GeoError::BadSearch(
            "gravitational parameter must be positive",
        ));
    }

    if state
        .position
        .iter()
        .chain(state.velocity.iter())
        .any(|x| !x.is_finite())
    {
        return Err(GeoError::BadSearch("orbital state must be finite"));
    }
    let r = state.position;
    let v = state.velocity;
    let rmag = norm(r);
    let vmag = norm(v);
    if rmag == 0.0 || vmag == 0.0 {
        return Err(GeoError::BadSearch("zero position or velocity"));
    }

    let h = cross(r, v);
    let hmag = norm(h);
    if hmag == 0.0 {
        return Err(GeoError::BadSearch("position and velocity are parallel"));
    }

    let mut node = [-h[1], h[0], 0.0];
    let factor = vmag * vmag - gm / rmag;
    let evec = scale(add(scale(r, factor), scale(v, -dot(r, v))), 1.0 / gm);
    let raw_ecc = norm(evec);
    let eccentricity = if (raw_ecc - 1.0).abs() <= 1e-10 {
        1.0
    } else {
        raw_ecc
    };
    let semilatus = dot(h, h) / gm;
    let perifocal_distance = semilatus / (eccentricity + 1.0);
    let mut inclination = vsep(h, [0.0, 0.0, 1.0]);
    if inclination.abs() < 1e-10 {
        inclination = 0.0;
        node = [1.0, 0.0, 0.0];
    } else if (inclination - PI).abs() < 1e-10 {
        inclination = PI;
        node = [1.0, 0.0, 0.0];
    }
    let mut ascending_node = node[1].atan2(node[0]);
    if ascending_node < 0.0 {
        ascending_node += TAU;
    }
    let mut argument_of_periapsis = if eccentricity == 0.0 {
        0.0
    } else {
        vsep(node, evec)
    };
    if argument_of_periapsis != 0.0 {
        if inclination == 0.0 || inclination == PI {
            let x = crate::surface::unit_cross(h, node);
            if dot(evec, x) < 0.0 {
                argument_of_periapsis = TAU - argument_of_periapsis;
            }
        } else if evec[2] < 0.0 {
            argument_of_periapsis = TAU - argument_of_periapsis;
        }
    }
    let perix = unit(if eccentricity == 0.0 { node } else { evec });
    let periy = crate::surface::unit_cross(h, perix);
    let nu = dot(r, periy).atan2(dot(r, perix));
    let mean_anomaly = if eccentricity < 1.0 {
        let cose = (eccentricity + nu.cos()) / (eccentricity * nu.cos() + 1.0);
        let sine = rmag / perifocal_distance
            * ((1.0 - eccentricity) / (eccentricity + 1.0)).sqrt()
            * nu.sin();
        let e = sine.atan2(cose);
        let mut m = (e - eccentricity * e.sin()).abs().copysign(nu);
        if m < 0.0 {
            m += TAU;
        }
        m
    } else if eccentricity > 1.0 {
        let cosh = ((eccentricity + nu.cos()) / (eccentricity * nu.cos() + 1.0)).max(1.0);
        // CSPICE DACOSH, retaining the ordered divisions.
        let e = (cosh + cosh * (1.0 - 1.0 / cosh / cosh).sqrt()).ln();
        (eccentricity * e.sinh() - e).abs().copysign(nu)
    } else {
        let e = (nu / 2.0).tan();
        (e + e * (e * e) / 3.0).abs().copysign(nu)
    };
    // OSCLTX recomputes the eccentricity vector for true anomaly using
    // a distinct coefficient ordering; circular orbits use mean anomaly.
    let true_anomaly = if eccentricity.abs() <= 1e-10 {
        mean_anomaly
    } else {
        let c1 = 1.0 / gm * (rmag * (vmag * vmag) - gm);
        let c2 = -(1.0 / gm) * dot(r, v);
        let ev = add(scale(unit(r), c1), scale(v, c2));
        let pole = crate::surface::unit_cross(r, v);
        // TWOVEC(eccvec,1,pole,3): X is ev, Y=pole x ev, Z=X x Y.
        let x = unit(ev);
        let y = crate::surface::unit_cross(pole, ev);
        let mut nu = dot(y, r).atan2(dot(x, r));
        if nu < 0.0 {
            nu += TAU;
        }
        nu
    };
    let mut semi_major_axis = 0.0;
    let mut period = 0.0;
    let limit = f64::MAX / 200.0;
    if eccentricity != 1.0 && rmag > gm / limit {
        let energy = vmag * vmag * 0.5 - gm / rmag;
        if energy.abs() >= gm.abs() / limit {
            semi_major_axis = -gm / (energy * 2.0);
            if eccentricity < 1.0 {
                let bound = (limit / TAU).powf(0.66666666666666663);
                let cube_root = gm.powf(0.33333333333333331);
                let computable = if gm >= 1.0 {
                    semi_major_axis / cube_root < bound
                } else {
                    semi_major_axis < bound * cube_root
                };
                if computable {
                    period = TAU * (semi_major_axis / cube_root).powf(1.5);
                }
            }
        }
    }
    Ok(OsculatingElements {
        perifocal_distance,
        eccentricity,
        inclination,
        ascending_node,
        argument_of_periapsis,
        mean_anomaly,
        true_anomaly,
        semi_major_axis,
        period,
        gm,
        range: rmag,
        speed: vmag,
    })
}

fn vsep(a: [f64; 3], b: [f64; 3]) -> f64 {
    if norm(a) == 0.0 || norm(b) == 0.0 {
        return 0.0;
    }
    let a = unit(a);
    let b = unit(b);
    let d = dot(a, b);
    if d > 0.0 {
        2.0 * (norm(subtract(a, b)) * 0.5).asin()
    } else if d < 0.0 {
        PI - 2.0 * (norm(add(a, b)) * 0.5).asin()
    } else {
        PI / 2.0
    }
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: [f64; 3]) -> f64 {
    crate::surface::norm(a)
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

fn unit(a: [f64; 3]) -> [f64; 3] {
    let length = norm(a);
    if length == 0.0 {
        [0.0; 3]
    } else {
        [a[0] / length, a[1] / length, a[2] / length]
    }
}

#[cfg(test)]
mod tests {
    use super::elements_from_state;
    use crate::spk::State;

    #[test]
    fn circular_equatorial_orbit() {
        // Circular orbit of radius 7000 km about a 398600 km^3/s^2 body.
        let radius: f64 = 7000.0;
        let gm: f64 = 398_600.0;
        let speed = (gm / radius).sqrt();
        let state = State {
            position: [radius, 0.0, 0.0],
            velocity: [0.0, speed, 0.0],
        };
        let elements = elements_from_state(&state, gm).unwrap();
        assert!((elements.perifocal_distance - radius).abs() < 1e-6);
        assert!(elements.eccentricity < 1e-12);
        assert!(elements.inclination < 1e-12);
        assert!((elements.semi_major_axis - radius).abs() < 1e-6);
    }
}
#[cfg(test)]
mod input_state_precision {
    #[test]
    fn real_input_states_exact() {
        use crate::{Aberration, Frames, Pck, SpkKernel};
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/gm_de440.tpc").unwrap();
        let frames = Frames::new();
        let ctx = crate::pointing::PointingContext {
            kernel: &kernel,
            pck: Some(&pck),
            frames: &frames,
            fov: None,
        };
        let mut different = 0;
        let mut output = String::from("orbiter,center,et,corr,frame,component,actual,expected\n");
        for line in include_str!("../../tests/golden/orbit_observations_precision.csv").lines() {
            let f: Vec<_> = line.split(',').collect();
            if f[5] != "CENTER" {
                continue;
            }
            let a = crate::pointing::state_in_inertial(
                &ctx,
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
                Aberration::parse(f[3]).unwrap(),
                f[4],
            )
            .unwrap();
            for (i, (a, b)) in a
                .position
                .into_iter()
                .chain(a.velocity)
                .zip(&f[19..])
                .enumerate()
            {
                let b = b.parse::<f64>().unwrap();
                if a.to_bits() != b.to_bits() {
                    different += 1;
                    output.push_str(&format!(
                        "{},{},{},{},{},{i},{a:?},{b:?}\n",
                        f[0], f[1], f[2], f[3], f[4]
                    ));
                }
            }
        }
        std::fs::write("target/reference/orbit_input_mismatches.csv", output).unwrap();
        assert_eq!(
            different, 0,
            "actual input states saved to target/reference/orbit_input_mismatches.csv"
        );
    }
}
