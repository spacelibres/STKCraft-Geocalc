// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::too_many_arguments, clippy::excessive_precision)]
//! Occultation of one body by another as seen from an observer.
//!
//! The apparent limb of a triaxial ellipsoid is an ellipse on the sky.
//! This module constructs that ellipse exactly from the ellipsoid's
//! shape matrix and the observer position, projects it through the
//! observer, and classifies two silhouettes as disjoint, partially
//! overlapping, or containing one another. That classification is the
//! same one CSPICE `occult` reports as no occultation, partial, annular,
//! or total.
//!
//! A point may be occulted by an ellipsoid or transit across one;
//! point transits are classified as annular, following CSPICE.
//!
//! The result is a geometric state. Event searches bisect the state
//! transition and therefore locate tangency to the search tolerance.

use crate::error::GeoError;
use crate::finder::{normalize_windows, solve_boolean, FinderOptions, Interval};
use crate::frame::Mat3;
use crate::pck::Pck;
use crate::spk::{Aberration, SpkKernel};

/// Shape model for an occultation target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccultShape {
    /// A point target.
    Point,
    /// A triaxial ellipsoid from the loaded text PCK.
    Ellipsoid,
}

/// Geometric occultation state for `front` (the nearer body, which
/// occults) and `back` (the body being occulted).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccultationState {
    /// The silhouettes do not overlap.
    None,
    /// The back body is completely hidden by the front body.
    Full,
    /// The silhouettes overlap but neither contains the other.
    Partial,
    /// The back body is visible as a ring around the front body.
    Annular,
}

/// Occultation condition selected by a search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccultationType {
    /// Any overlap of the silhouettes.
    Any,
    /// Complete occultation of the back body.
    Full,
    /// Partial overlap only.
    Partial,
    /// The back body surrounds the front body.
    Annular,
}

impl OccultationType {
    /// Parses a CSPICE `OCCULTATION TYPE` string.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "ANY" => Some(Self::Any),
            "FULL" => Some(Self::Full),
            "PARTIAL" => Some(Self::Partial),
            "ANNULAR" => Some(Self::Annular),
            _ => None,
        }
    }

    fn matches(self, state: OccultationState) -> bool {
        match self {
            Self::Any => state != OccultationState::None,
            Self::Full => state == OccultationState::Full,
            Self::Partial => state == OccultationState::Partial,
            Self::Annular => state == OccultationState::Annular,
        }
    }
}

/// Computes the occultation state of `back` by `front` at `et`.
#[allow(clippy::too_many_arguments)]
pub fn occultation_state(
    kernel: &SpkKernel,
    pck: &Pck,
    front: i32,
    front_shape: OccultShape,
    back: i32,
    back_shape: OccultShape,
    observer: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<OccultationState, GeoError> {
    if front == observer || back == observer {
        return Err(GeoError::BadSearch(
            "observer coincides with an occultation target",
        ));
    }
    if front == back {
        return Err(GeoError::BadSearch("front and back bodies are the same"));
    }

    if front_shape == OccultShape::Point && back_shape == OccultShape::Point {
        return Err(GeoError::BadSearch(
            "at least one occultation body must be extended",
        ));
    }
    // ZZGFOCIN deliberately strips stellar aberration for occultations.
    let abcorr = without_stellar(abcorr);
    let (front_center, front_light_time) =
        crate::spkapo::corrected_position(kernel, front, observer, et, abcorr)?;
    let (back_center, back_light_time) =
        crate::spkapo::corrected_position(kernel, back, observer, et, abcorr)?;

    if front_shape == OccultShape::Point || back_shape == OccultShape::Point {
        return point_occultation_state(
            kernel,
            pck,
            front,
            front_shape,
            back,
            observer,
            et,
            abcorr,
            front_center,
            back_center,
        );
    }
    let front_radii = pck.radii(front).ok_or(GeoError::MissingPckData(front))?;
    let back_radii = pck.radii(back).ok_or(GeoError::MissingPckData(back))?;
    let front_epoch = crate::state_rep::time_at_target(et, front_light_time, abcorr);
    let back_epoch = crate::state_rep::time_at_target(et, back_light_time, abcorr);
    let front_rotation = pck
        .body_rotation(front, front_epoch)
        .ok_or(GeoError::MissingPckData(front))?;
    let back_rotation = pck
        .body_rotation(back, back_epoch)
        .ok_or(GeoError::MissingPckData(back))?;
    crate::occult_ellipsoid::classify_ellipsoids(
        front_center,
        front_radii,
        &front_rotation,
        back_center,
        back_radii,
        &back_rotation,
    )
}

/// Time intervals during which the selected occultation condition holds.
#[allow(clippy::too_many_arguments)]
pub fn find_occultation(
    kernel: &SpkKernel,
    pck: &Pck,
    front: i32,
    front_shape: OccultShape,
    back: i32,
    back_shape: OccultShape,
    observer: i32,
    abcorr: Aberration,
    occultation_type: OccultationType,
    windows: &[Interval],
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    if !options.step.is_finite()
        || options.step <= 0.0
        || !options.tolerance.is_finite()
        || options.tolerance <= 0.0
    {
        return Err(GeoError::BadSearch(
            "occultation search step and tolerance must be finite and positive",
        ));
    }
    if (front_shape == OccultShape::Point || back_shape == OccultShape::Point)
        && occultation_type != OccultationType::Any
    {
        return Err(GeoError::BadSearch(
            "point shapes require occultation type ANY",
        ));
    }
    if front_shape == OccultShape::Point && back_shape == OccultShape::Point {
        return Err(GeoError::BadSearch(
            "at least one occultation body must be extended",
        ));
    }
    let mut result = Vec::new();
    for window in normalize_windows(windows)? {
        result.extend(solve_boolean(
            &|et| {
                Ok(occultation_type.matches(occultation_state(
                    kernel,
                    pck,
                    front,
                    front_shape,
                    back,
                    back_shape,
                    observer,
                    et,
                    abcorr,
                )?))
            },
            window,
            options.step,
            options.tolerance,
        )?);
    }
    normalize_windows(&result)
}

pub(crate) fn inverse3(matrix: &Mat3) -> Result<Mat3, GeoError> {
    let [(a, b, c), (d, e, f), (g, h, i)] = [
        (matrix[0][0], matrix[0][1], matrix[0][2]),
        (matrix[1][0], matrix[1][1], matrix[1][2]),
        (matrix[2][0], matrix[2][1], matrix[2][2]),
    ];
    let determinant = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if determinant.abs() < 1e-300 {
        return Err(GeoError::BadSearch("degenerate projection"));
    }
    let inverse = [
        [
            (e * i - f * h) / determinant,
            (c * h - b * i) / determinant,
            (b * f - c * e) / determinant,
        ],
        [
            (f * g - d * i) / determinant,
            (a * i - c * g) / determinant,
            (c * d - a * f) / determinant,
        ],
        [
            (d * h - e * g) / determinant,
            (b * g - a * h) / determinant,
            (a * e - b * d) / determinant,
        ],
    ];
    Ok(inverse)
}

fn separation(first: [f64; 3], second: [f64; 3]) -> f64 {
    let first_length = norm(first);
    let second_length = norm(second);
    if first_length == 0.0 || second_length == 0.0 {
        return 0.0;
    }
    let u1 = first.map(|x| x / first_length);
    let u2 = second.map(|x| x / second_length);
    let cosine = dot(u1, u2);
    if cosine > 0.0 {
        2.0 * (norm(subtract(u1, u2)) * 0.5).asin()
    } else if cosine < 0.0 {
        std::f64::consts::PI - 2.0 * (norm(add(u1, u2)) * 0.5).asin()
    } else {
        std::f64::consts::FRAC_PI_2
    }
}

fn norm(a: [f64; 3]) -> f64 {
    crate::surface::norm(a)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn without_stellar(correction: Aberration) -> Aberration {
    match correction {
        Aberration::LightTimeStellar => Aberration::LightTime,
        Aberration::ConvergedLightTimeStellar => Aberration::ConvergedLightTime,
        Aberration::TransmissionLightTimeStellar => Aberration::TransmissionLightTime,
        Aberration::TransmissionConvergedLightTimeStellar => {
            Aberration::TransmissionConvergedLightTime
        }
        c => c,
    }
}

fn point_occultation_state(
    kernel: &SpkKernel,
    pck: &Pck,
    front: i32,
    front_shape: OccultShape,
    back: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    front_position: [f64; 3],
    back_position: [f64; 3],
) -> Result<OccultationState, GeoError> {
    let point_occulted = front_shape == OccultShape::Ellipsoid;
    let extended = if point_occulted { front } else { back };
    let radii = pck
        .radii(extended)
        .ok_or(GeoError::MissingPckData(extended))?;
    let minimum = radii[0].min(radii[1]).min(radii[2]);
    let maximum = radii[0].max(radii[1]).max(radii[2]);
    if minimum <= 0.0 {
        return Err(GeoError::BadSearch("ellipsoid axes must be positive"));
    }
    let front_distance = norm(front_position);
    let back_distance = norm(back_position);
    let (distance, point_distance) = if point_occulted {
        (front_distance, back_distance)
    } else {
        (back_distance, front_distance)
    };
    let back_to_front = subtract(front_position, back_position);
    if distance <= minimum || point_distance == 0.0 || norm(back_to_front) <= minimum {
        return Err(GeoError::BadSearch(
            "occultation bodies must be disjoint from each other and observer",
        ));
    }
    let separation_angle = separation(back_position, front_position);
    let outer_radius = maximum * 1.01;
    if outer_radius < distance && separation_angle > (outer_radius / distance).asin() {
        return Ok(OccultationState::None);
    }
    let min_angle = (minimum * 0.98999999999999999 / distance).asin();
    if separation_angle < min_angle {
        let overlaps = if point_occulted {
            let front_to_back = back_to_front.map(|x| -x);
            separation(front_position.map(|x| -x), front_to_back)
                > std::f64::consts::FRAC_PI_2 - min_angle
        } else {
            separation(back_position.map(|x| -x), back_to_front)
                < std::f64::consts::FRAC_PI_2 - min_angle
        };
        return Ok(if !overlaps {
            OccultationState::None
        } else if point_occulted {
            OccultationState::Full
        } else {
            OccultationState::Annular
        });
    }
    let direction = if point_occulted {
        back_position
    } else {
        front_position
    };
    let intercept = match crate::surface::surface_intercept_geometry(
        kernel, pck, extended, observer, et, abcorr, direction,
    ) {
        Ok(g) => g,
        Err(GeoError::BadSearch("ray does not intersect the target")) => {
            return Ok(OccultationState::None)
        }
        Err(error) => return Err(error),
    };
    let range = norm(intercept.surface_vector);
    Ok(if point_occulted && range < back_distance {
        OccultationState::Full
    } else if !point_occulted && range > front_distance {
        OccultationState::Annular
    } else {
        OccultationState::None
    })
}

#[cfg(test)]
// Keep each source primitive's independent regression adjacent to its implementation.
#[allow(clippy::items_after_test_module)]
mod exact_point_occultation_regressions {
    use super::*;
    #[test]
    fn point_transit_and_depth_order_match_cspice() {
        let kernel = SpkKernel::open("tests/golden/gf_fixed.bsp").unwrap();
        let pck = Pck::from_text("\\begindata\nBODY10002_RADII=(1000 1000 1000)\n").unwrap();
        let state = occultation_state(
            &kernel,
            &pck,
            10001,
            OccultShape::Point,
            10002,
            OccultShape::Ellipsoid,
            0,
            0.0,
            Aberration::None,
        )
        .unwrap();
        assert_eq!(state, OccultationState::Annular);
        let behind = occultation_state(
            &kernel,
            &pck,
            10002,
            OccultShape::Ellipsoid,
            10001,
            OccultShape::Point,
            0,
            0.0,
            Aberration::None,
        )
        .unwrap();
        assert_eq!(behind, OccultationState::None);
        assert!(occultation_state(
            &kernel,
            &pck,
            10001,
            OccultShape::Point,
            10002,
            OccultShape::Point,
            0,
            0.0,
            Aberration::None
        )
        .is_err());
        assert!(find_occultation(
            &kernel,
            &pck,
            10001,
            OccultShape::Point,
            10002,
            OccultShape::Ellipsoid,
            0,
            Aberration::None,
            OccultationType::Full,
            &[Interval {
                start: 0.0,
                end: 1.0
            }],
            FinderOptions::default()
        )
        .is_err());
    }
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
