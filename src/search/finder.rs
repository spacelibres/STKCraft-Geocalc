// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::neg_multiply,
    clippy::if_same_then_else
)]
//! Geometry Finder scalar and boolean searches and closed-window operations.
//!
//! Astronomy searches use the N0067 ZZGFREL/ZZGFSOLV schedules and source
//! derivatives, including coordinate branch-cut handling from ZZGFLONG.
//! [`search_scalar_with_decreasing`] accepts the decreasing predicate needed
//! to reproduce CSPICE GFUDS. The value-only [`search_scalar`] uses native
//! UDDC(1 second); [`search_scalar_with_uddc`] makes that step explicit.
//! Separation events are the isolated epochs returned by native GFSEP.
//!
//! Search steps must resolve every feature; convergence tolerances refer to
//! time, never to a scalar equality band. Window set operations use exact
//! endpoint comparisons without any convergence tolerance.

use std::f64::consts::PI;

use crate::coords::{coordinate_value, coordinate_value_with_pck, CoordinateSystem};
use crate::error::GeoError;
use crate::frame::Frames;
use crate::pck::Pck;
use crate::spk::{Aberration, SpkKernel};

/// The geometric model used for a target in an angular-separation
/// calculation, matching the WebGeocalc `shape1`/`shape2` values that
/// CSPICE `trgsep` accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngularShape {
    /// Treat the body as a point; its radius is zero.
    Point,
    /// Treat the body as a sphere whose radius is the largest of the
    /// body's radii in the text PCK, matching CSPICE `trgsep`.
    Sphere,
}

impl AngularShape {
    /// Parses a WebGeocalc `shape1`/`shape2` string.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "POINT" => Some(Self::Point),
            "SPHERE" => Some(Self::Sphere),
            _ => None,
        }
    }

    /// The radius this shape uses for `target`, in kilometres.
    fn radius(self, pck: &Pck, target: i32) -> Result<f64, GeoError> {
        match self {
            Self::Point => Ok(0.0),
            Self::Sphere => pck
                .radii(target)
                .map(|radii| radii[0].max(radii[1]).max(radii[2]))
                .ok_or(GeoError::BadSearch(
                    "the PCK has no radii for a body modelled as a sphere",
                )),
        }
    }
}

/// Angular separation in radians between two targets seen from an
/// observer, corrected with `abcorr`.
///
/// The angle between the two apparent position vectors, using CSPICE `VSEP`
/// semantics. WebGeocalc returns zero when a point target is the observer;
/// CSPICE `trgsep` instead rejects non-distinct bodies.
pub fn angular_separation(
    kernel: &SpkKernel,
    observer: i32,
    target1: i32,
    target2: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<f64, GeoError> {
    let first = crate::surface::apparent_position(kernel, target1, observer, et, abcorr)?;
    let second = crate::surface::apparent_position(kernel, target2, observer, et, abcorr)?;
    Ok(angle_between(first, second))
}

/// Angular separation in radians between two shaped targets, matching
/// CSPICE `trgsep` for distinct bodies. For a point target coincident with
/// the observer, WebGeocalc uses a zero vector instead of the native
/// `SPICE(BODIESNOTDISTINCT)` error.
///
/// With [`AngularShape::Sphere`] the angle is measured between the
/// apparent **limbs** rather than the centers: the result is
/// `theta - asin(r1/range1) - asin(r2/range2)`. It is therefore negative
/// when the apparent disks overlap, and exactly zero when they touch,
/// exactly as `trgsep` documents. A sphere's radius is the largest of the
/// body's PCK radii.
#[allow(clippy::too_many_arguments)]
pub fn angular_separation_shaped(
    kernel: &SpkKernel,
    pck: &Pck,
    observer: i32,
    target1: i32,
    target2: i32,
    et: f64,
    abcorr: Aberration,
    shape1: AngularShape,
    shape2: AngularShape,
) -> Result<f64, GeoError> {
    let first = crate::surface::apparent_position(kernel, target1, observer, et, abcorr)?;
    let second = crate::surface::apparent_position(kernel, target2, observer, et, abcorr)?;
    let separation = angle_between(first, second);
    let angle1 = match shape1 {
        AngularShape::Point => 0.0,
        AngularShape::Sphere => apparent_angular_radius(first, shape1.radius(pck, target1)?)?,
    };
    let angle2 = match shape2 {
        AngularShape::Point => 0.0,
        AngularShape::Sphere => apparent_angular_radius(second, shape2.radius(pck, target2)?)?,
    };
    Ok(separation - angle1 - angle2)
}

/// `asin(radius / range)`, rejecting the case where the observer is inside
/// the body (CSPICE `SPICE(INSIDEBODY)`).
fn apparent_angular_radius(position: [f64; 3], radius: f64) -> Result<f64, GeoError> {
    let range = norm(position);
    if range <= radius || !range.is_finite() {
        return Err(GeoError::BadSearch(
            "the observer is inside a body modelled as a sphere",
        ));
    }
    Ok((radius / range).clamp(-1.0, 1.0).asin())
}

/// Phase angle in radians: the angle at the target between the observer
/// and the illuminator, using apparent positions. Matches CSPICE
/// `phaseq`.
pub fn phase_angle(
    kernel: &SpkKernel,
    target: i32,
    illuminator: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<f64, GeoError> {
    if target == illuminator || target == observer || illuminator == observer {
        return Err(GeoError::BadSearch(
            "phase angle requires distinct target, illuminator and observer",
        ));
    }
    let (_, _, _, transmission) = abcorr.flags();
    if transmission {
        return Err(GeoError::BadAberration(
            "phase angle supports reception corrections only".to_string(),
        ));
    }
    let (target_to_observer, light_time) =
        crate::spkapo::corrected_position(kernel, target, observer, et, abcorr)?;

    // `zzgfpaq` evaluates the illuminator relative to the target at
    // ET - LT for corrected cases, and at ET when no correction is used.
    let illuminator_time = if abcorr == Aberration::None {
        et
    } else {
        et - light_time
    };
    let (illuminator_to_target, _) =
        crate::spkapo::corrected_position(kernel, illuminator, target, illuminator_time, abcorr)?;

    let separation = angle_between(target_to_observer, illuminator_to_target);
    Ok(std::f64::consts::PI - separation)
}

/// Angular size computation method, matching WebGeocalc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngularSizeMethod {
    /// Triaxial-ellipsoid limb extent.
    LimbExtents,
    /// Sphere using the largest PCK radius (WebGeocalc default).
    MaxRadius,
    /// Sphere using the smallest PCK radius.
    MinRadius,
}

impl AngularSizeMethod {
    /// Parses a WebGeocalc `computationMethod` string.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "LIMB_EXTENTS" => Some(Self::LimbExtents),
            "MAX_RADIUS" => Some(Self::MaxRadius),
            "MIN_RADIUS" => Some(Self::MinRadius),
            _ => None,
        }
    }
}

/// Angular size in radians of a target as seen from an observer.
///
/// The default matches WebGeocalc: a sphere with the largest PCK
/// radius. Use [`angular_size_with_method`] for the other methods.
pub fn angular_size(
    kernel: &SpkKernel,
    pck: &Pck,
    observer: i32,
    target: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<f64, GeoError> {
    angular_size_with_method(
        kernel,
        pck,
        observer,
        target,
        et,
        abcorr,
        AngularSizeMethod::MaxRadius,
    )
}

/// Angular size in radians using the selected WebGeocalc method.
pub fn angular_size_with_method(
    kernel: &SpkKernel,
    pck: &Pck,
    observer: i32,
    target: i32,
    et: f64,
    abcorr: Aberration,
    method: AngularSizeMethod,
) -> Result<f64, GeoError> {
    let (center, light_time) =
        crate::spkapo::corrected_position(kernel, target, observer, et, abcorr)?;
    let distance = norm(center);
    if distance == 0.0 && method != AngularSizeMethod::LimbExtents {
        return Err(GeoError::BadSearch("observer and target coincide"));
    }
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;

    match method {
        AngularSizeMethod::MaxRadius => {
            let radius = radii.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            Ok(2.0 * (radius / distance).asin())
        }
        AngularSizeMethod::MinRadius => {
            let radius = radii.iter().copied().fold(f64::INFINITY, f64::min);
            Ok(2.0 * (radius / distance).asin())
        }
        AngularSizeMethod::LimbExtents => {
            let epoch = crate::state_rep::time_at_target(et, light_time, abcorr);
            let rotation = pck
                .body_rotation(target, epoch)
                .ok_or(GeoError::MissingPckData(target))?;
            Ok(crate::occult_ellipsoid::limb_axis_extents(center, radii, &rotation)?[0])
        }
    }
}

/// The kind of event found during a separation search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// A local minimum of the separation.
    LocalMinimum,
    /// A local maximum of the separation.
    LocalMaximum,
    /// The separation crossed `threshold` while increasing.
    CrossingUp,
    /// The separation crossed `threshold` while decreasing.
    CrossingDown,
}

/// One event located by [`search_separation`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeparationEvent {
    /// Event epoch, TDB seconds past J2000.
    pub et: f64,
    /// Separation in radians at `et`.
    pub separation: f64,
    /// What kind of event this is.
    pub kind: EventKind,
}

/// Controls for [`search_separation`].
#[derive(Clone, Copy, Debug)]
pub struct SearchOptions {
    /// Sampling step in seconds. Must resolve every feature in the
    /// window.
    pub step: f64,
    /// Event time tolerance in seconds.
    pub tolerance: f64,
    /// When set, also report crossings of this separation value.
    pub threshold: Option<f64>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            step: 3600.0,
            tolerance: 1e-6,
            threshold: None,
        }
    }
}

/// Searches `[start, end]` for local extrema and, when a threshold is
/// set, crossings of the angular separation.
#[allow(clippy::too_many_arguments)]
pub fn search_separation(
    kernel: &SpkKernel,
    observer: i32,
    target1: i32,
    target2: i32,
    abcorr: Aberration,
    start: f64,
    end: f64,
    options: SearchOptions,
) -> Result<Vec<SeparationEvent>, GeoError> {
    search_separation_shaped(
        kernel,
        &Pck::default(),
        observer,
        target1,
        target2,
        abcorr,
        start,
        end,
        options,
        AngularShape::Point,
        AngularShape::Point,
    )
}

/// Searches `[start, end]` for local extrema and threshold crossings of a
/// shaped angular separation, matching CSPICE `gfsep` with `POINT` or
/// `SPHERE` shapes.
#[allow(clippy::too_many_arguments)]
pub fn search_separation_shaped(
    kernel: &SpkKernel,
    pck: &Pck,
    observer: i32,
    target1: i32,
    target2: i32,
    abcorr: Aberration,
    start: f64,
    end: f64,
    options: SearchOptions,
    shape1: AngularShape,
    shape2: AngularShape,
) -> Result<Vec<SeparationEvent>, GeoError> {
    let radius1 = shape1.radius(pck, target1)?;
    let radius2 = shape2.radius(pck, target2)?;
    let windows = [Interval { start, end }];
    let native_options = FinderOptions {
        step: options.step,
        tolerance: options.tolerance,
        adjustment: 0.0,
    };
    search_separation_with_native(
        |condition, reference| {
            find_angular_separation(
                kernel,
                pck,
                observer,
                target1,
                target2,
                abcorr,
                shape1,
                shape2,
                &windows,
                condition,
                reference,
                native_options,
            )
        },
        |et| {
            angular_separation_shaped(
                kernel, pck, observer, target1, target2, et, abcorr, shape1, shape2,
            )
        },
        |et| {
            separation_decreasing(
                kernel, observer, target1, target2, et, abcorr, radius1, radius2,
            )
        },
        start,
        end,
        options,
    )
}

fn search_separation_with_native<F, V, D>(
    mut find: F,
    evaluate: V,
    decreasing: D,
    start: f64,
    end: f64,
    options: SearchOptions,
) -> Result<Vec<SeparationEvent>, GeoError>
where
    F: FnMut(Condition, f64) -> Result<Vec<Interval>, GeoError>,
    V: Fn(f64) -> Result<f64, GeoError>,
    D: Fn(f64) -> Result<bool, GeoError>,
{
    if !start.is_finite() || !end.is_finite() || end <= start {
        return Err(GeoError::BadSearch(
            "search window must be finite and non-empty",
        ));
    }
    if !options.step.is_finite()
        || options.step <= 0.0
        || !options.tolerance.is_finite()
        || options.tolerance <= 0.0
    {
        return Err(GeoError::BadSearch(
            "step and tolerance must be positive and finite",
        ));
    }
    if options.threshold.is_some_and(|x| !x.is_finite()) {
        return Err(GeoError::BadSearch("separation threshold must be finite"));
    }
    let mut events = Vec::new();
    for (condition, kind) in [
        (Condition::LocalMinimum, EventKind::LocalMinimum),
        (Condition::LocalMaximum, EventKind::LocalMaximum),
    ] {
        for interval in find(condition, 0.0)? {
            if interval.start != interval.end {
                return Err(GeoError::Unsupported(
                    "non-isolated extrema require the native window API".into(),
                ));
            }
            events.push(SeparationEvent {
                et: interval.start,
                separation: evaluate(interval.start)?,
                kind,
            });
        }
    }
    if let Some(reference) = options.threshold {
        for interval in find(Condition::Equal, reference)? {
            if interval.start != interval.end {
                return Err(GeoError::Unsupported(
                    "non-isolated equality requires the native window API".into(),
                ));
            }
            let et = interval.start;
            events.push(SeparationEvent {
                et,
                separation: evaluate(et)?,
                kind: if decreasing(et)? {
                    EventKind::CrossingDown
                } else {
                    EventKind::CrossingUp
                },
            });
        }
    }
    events.sort_by(|a, b| a.et.total_cmp(&b.et));
    Ok(events)
}

/// CSPICE `VSEP`: stable angle between two position vectors.
fn angle_between(first: [f64; 3], second: [f64; 3]) -> f64 {
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
        PI - 2.0 * (norm(add(u1, u2)) * 0.5).asin()
    } else {
        PI * 0.5
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

// ---------------------------------------------------------------------
// General Geometry Finder engine
// ---------------------------------------------------------------------

/// A closed time interval in TDB seconds past J2000.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    /// Start of the interval.
    pub start: f64,
    /// End of the interval.
    pub end: f64,
}

/// A CSPICE `RELATE` condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// The scalar is greater than the reference value.
    GreaterThan,
    /// The scalar is less than the reference value.
    LessThan,
    /// The scalar equals the reference value (time tolerance only).
    Equal,
    /// The scalar attains its absolute maximum.
    AbsoluteMaximum,
    /// The scalar attains its absolute minimum.
    AbsoluteMinimum,
    /// The scalar attains a local maximum.
    LocalMaximum,
    /// The scalar attains a local minimum.
    LocalMinimum,
}

impl Condition {
    /// Parses a CSPICE `RELATE` string.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            ">" => Some(Self::GreaterThan),
            "<" => Some(Self::LessThan),
            "=" => Some(Self::Equal),
            "ABSMAX" => Some(Self::AbsoluteMaximum),
            "ABSMIN" => Some(Self::AbsoluteMinimum),
            "LOCMAX" => Some(Self::LocalMaximum),
            "LOCMIN" => Some(Self::LocalMinimum),
            _ => None,
        }
    }
}

/// Controls for the scalar event search.
#[derive(Clone, Copy, Debug)]
pub struct FinderOptions {
    /// Sampling step in seconds. It must resolve every feature of the
    /// scalar within the confinement window.
    pub step: f64,
    /// Convergence tolerance for event times, seconds.
    pub tolerance: f64,
    /// Allowed variation from an absolute extremum; zero for other relations.
    pub adjustment: f64,
}

impl Default for FinderOptions {
    fn default() -> Self {
        Self {
            step: 3600.0,
            tolerance: 1.0e-6,
            adjustment: 0.0,
        }
    }
}

/// Native GFUDS with UDDC as its decreasing callback, using the supplied
/// nonzero UDDF abscissa step. The callback evaluates `evaluate` at ET +/-
/// this step, including outside confinement. This is the explicit native
/// UDDC contract; use `search_scalar_with_decreasing` for another predicate.
pub fn search_scalar_with_uddc<F>(
    evaluate: F,
    derivative_step: f64,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError>
where
    F: Fn(f64) -> Result<f64, GeoError>,
{
    if !derivative_step.is_finite() || derivative_step == 0.0 {
        return Err(GeoError::BadSearch(
            "UDDC derivative step must be finite and nonzero",
        ));
    }
    search_scalar_with_decreasing(
        &evaluate,
        |et| Ok(user_function_derivative(&evaluate, et, derivative_step)? < 0.0),
        windows,
        condition,
        reference,
        options,
    )
}

// N0067 UDDF -> QDERIV -> VLCOMG, retaining coefficient/multiply/add order.
fn user_function_derivative<F>(evaluate: &F, et: f64, step: f64) -> Result<f64, GeoError>
where
    F: Fn(f64) -> Result<f64, GeoError>,
{
    let before = evaluate(et - step)?;
    let after = evaluate(et + step)?;
    Ok((0.5 / step) * after + (-0.5 / step) * before)
}

#[cfg(test)]
mod native_uddf_golden {
    use super::*;
    #[test]
    fn exact_native_uddf_and_uddc_primitives() {
        for line in include_str!("../../tests/golden/uddf_precision.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let value = |t: f64| {
                Ok(match f[0] {
                    "linear" => t,
                    "quadratic" => t * t,
                    "negative_quadratic" => -t * t,
                    "cubic" => t * (t - 2.) * (t + 2.),
                    "quartic" => (t - 1.) * (t - 1.) * (t + 2.) * (t + 2.),
                    "constant" => 2.,
                    "small_linear" => 1e-12 * t,
                    "small_quadratic" => 1e-12 * (t - 0.37) * (t - 0.37),
                    _ => unreachable!(),
                })
            };
            let actual =
                user_function_derivative(&value, f[1].parse().unwrap(), f[2].parse().unwrap())
                    .unwrap();
            assert_eq!(
                actual.to_bits(),
                f[3].parse::<f64>().unwrap().to_bits(),
                "{line}"
            );
            assert_eq!(actual < 0., f[4] == "1", "{line}");
        }
    }
}

/// Native GFUDS with the UDDC callback and a one-second UDDF abscissa step.
///
/// The returned intervals are ordered and non-overlapping. Local and
/// absolute extrema are reported as zero-width intervals, matching the
/// CSPICE Geometry Finder convention.
pub fn search_scalar<F>(
    evaluate: F,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError>
where
    F: Fn(f64) -> Result<f64, GeoError>,
{
    search_scalar_with_uddc(evaluate, 1.0, windows, condition, reference, options)
}

// ---------------------------------------------------------------------
// Scalar finders
// ---------------------------------------------------------------------

/// Distance from `observer` to `target`.
#[allow(clippy::too_many_arguments)]
pub fn find_distance(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    search_scalar_with_decreasing(
        |et| {
            Ok(norm(
                crate::spkapo::corrected_position(kernel, target, observer, et, abcorr)?.0,
            ))
        },
        |et| {
            let state = kernel.state_corrected(target, observer, et, abcorr)?.state;
            Ok(dot(state.position, state.velocity) < 0.0)
        },
        windows,
        condition,
        reference,
        options,
    )
}

/// Range rate from `observer` to `target`, km/s.
#[allow(clippy::too_many_arguments)]
pub fn find_range_rate(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    search_scalar_with_decreasing(
        |et| {
            let state = kernel.state_corrected(target, observer, et, abcorr)?.state;
            let range = norm(state.position);
            if range == 0.0 {
                return Ok(0.0);
            }
            Ok(dot(state.velocity, state.position.map(|x| x / range)))
        },
        |et| {
            // N0067 ZZGFRRDC uses QDERIV with DT=1 second.
            let before = kernel
                .state_corrected(target, observer, et - 1.0, abcorr)?
                .state;
            let after = kernel
                .state_corrected(target, observer, et + 1.0, abcorr)?
                .state;
            let acceleration =
                std::array::from_fn(|i| 0.5 * after.velocity[i] + (-0.5) * before.velocity[i]);
            let state = kernel.state_corrected(target, observer, et, abcorr)?.state;
            let (unit, derivative) = unit_state(state.position, state.velocity);
            Ok(dot(acceleration, unit) + dot(state.velocity, derivative) < 0.0)
        },
        windows,
        condition,
        reference,
        options,
    )
}

/// Phase angle of `target` lit by `illuminator` and seen from `observer`.
#[allow(clippy::too_many_arguments)]
pub fn find_phase_angle(
    kernel: &SpkKernel,
    target: i32,
    illuminator: i32,
    observer: i32,
    abcorr: Aberration,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    search_scalar_with_decreasing(
        |et| phase_angle(kernel, target, illuminator, observer, et, abcorr),
        |et| {
            let first = kernel.state_corrected(target, observer, et, abcorr)?;
            let time = if abcorr == Aberration::None {
                et
            } else {
                et - first.light_time
            };
            let mut second = kernel
                .state_corrected(illuminator, target, time, abcorr)?
                .state;
            if abcorr != Aberration::None {
                let unit = first.state.position.map(|x| x / norm(first.state.position));
                let dlt = dot(unit, first.state.velocity) / crate::spk::CLIGHT;
                second.velocity = second.velocity.map(|x| (1.0 - dlt) * x);
            }
            let (u1, v1) = unit_state(first.state.position, first.state.velocity);
            let (u2, v2) = unit_state(second.position, second.velocity);
            Ok(dot(u1, v2) + dot(v1, u2) < 0.0)
        },
        windows,
        condition,
        reference,
        options,
    )
}

/// The illumination angle selected by [`IlluminationAngle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IlluminationAngle {
    /// Solar incidence angle at the surface point.
    Incidence,
    /// Emission angle at the surface point.
    Emission,
    /// Phase angle at the surface point.
    Phase,
}

/// An illumination angle at a fixed body-fixed surface point.
#[allow(clippy::too_many_arguments)]
pub fn find_illumination_angle(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    point: [f64; 3],
    angle: IlluminationAngle,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    find_illumination_angle_with_illuminator(
        kernel, pck, target, 10, observer, abcorr, point, angle, windows, condition, reference,
        options,
    )
}

/// A coordinate of the observer-target position vector in `frame`.
#[allow(clippy::too_many_arguments)]
pub fn find_position_coordinate(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    frames: &Frames,
    frame: &str,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    let needs_radii = matches!(
        system,
        CoordinateSystem::Geodetic | CoordinateSystem::Planetographic
    );
    if needs_radii {
        return Err(GeoError::BadSearch(
            "use find_position_coordinate_with_radii for ellipsoidal systems",
        ));
    }
    if longitude_index(system) == Some(coordinate) {
        return search_longitude_state(
            |et| transformed_state(kernel, frames, target, observer, et, abcorr, frame, None),
            |et| {
                Ok(crate::spkapo::corrected_position_in_frame(
                    kernel, target, observer, et, abcorr, frames, frame, None,
                )?
                .0)
            },
            windows,
            system,
            false,
            condition,
            reference,
            options,
        );
    }
    search_scalar_with_decreasing(
        |et| {
            let vector = crate::spkapo::corrected_position_in_frame(
                kernel, target, observer, et, abcorr, frames, frame, None,
            )?
            .0;
            coordinate_value(vector, system, coordinate, None, target)
        },
        |et| {
            let (p, v) =
                transformed_state(kernel, frames, target, observer, et, abcorr, frame, None)?;
            coordinate_decreasing(p, v, system, coordinate, None, false)
        },
        windows,
        condition,
        reference,
        options,
    )
}

/// A coordinate of the observer-target position vector, including the
/// ellipsoidal systems that require body radii.
#[allow(clippy::too_many_arguments)]
pub fn find_position_coordinate_with_radii(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    frames: &Frames,
    frame: &str,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    let body = frames.center(frame)?;
    let radii = if matches!(
        system,
        CoordinateSystem::Geodetic | CoordinateSystem::Planetographic
    ) {
        Some(pck.radii(body).ok_or(GeoError::MissingPckData(body))?)
    } else {
        None
    };
    let east = pck_coordinate_sense(pck, system, body)?;
    if longitude_index(system) == Some(coordinate) {
        return search_longitude_state(
            |et| {
                transformed_state(
                    kernel,
                    frames,
                    target,
                    observer,
                    et,
                    abcorr,
                    frame,
                    Some(pck),
                )
            },
            |et| {
                Ok(crate::spkapo::corrected_position_in_frame(
                    kernel,
                    target,
                    observer,
                    et,
                    abcorr,
                    frames,
                    frame,
                    Some(pck),
                )?
                .0)
            },
            windows,
            system,
            east,
            condition,
            reference,
            options,
        );
    }
    search_scalar_with_decreasing(
        |et| {
            let vector = crate::spkapo::corrected_position_in_frame(
                kernel,
                target,
                observer,
                et,
                abcorr,
                frames,
                frame,
                Some(pck),
            )?
            .0;
            coordinate_value_with_pck(vector, system, coordinate, radii, body, pck)
        },
        |et| {
            let (p, v) = transformed_state(
                kernel,
                frames,
                target,
                observer,
                et,
                abcorr,
                frame,
                Some(pck),
            )?;
            coordinate_decreasing(p, v, system, coordinate, radii, east)
        },
        windows,
        condition,
        reference,
        options,
    )
}

/// Which sub-point a [`find_sub_point_coordinate`] search uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubPointKind {
    /// Sub-observer point.
    Observer,
    /// Sub-solar point.
    Solar,
}

/// A coordinate of the sub-observer or sub-solar point.
#[allow(clippy::too_many_arguments)]
pub fn find_sub_point_coordinate(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    kind: SubPointKind,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    find_sub_point_coordinate_with_method(
        kernel,
        pck,
        target,
        observer,
        abcorr,
        kind,
        crate::surface::SubPointMethod::NearPoint,
        system,
        coordinate,
        windows,
        condition,
        reference,
        options,
    )
}

/// A coordinate of a sub-point with explicit CSPICE sub-point method.
#[allow(clippy::too_many_arguments)]
pub fn find_sub_point_coordinate_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    kind: SubPointKind,
    method: crate::SubPointMethod,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    if frames.center(target_frame)? != target {
        return Err(GeoError::BadSearch(
            "target frame is not centered on target",
        ));
    }
    if kind == SubPointKind::Solar && windows.iter().any(|w| w.start < w.end) {
        return Err(GeoError::Unsupported("CSPICE GFSubC supports sub-observer points; solar coordinate interval searches require a separate exact derivative solver".into()));
    }
    let east = pck_coordinate_sense(pck, system, target)?;
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    if kind == SubPointKind::Observer {
        let state = |et| {
            subpoint_surface_state_in(
                kernel,
                pck,
                frames,
                target_frame,
                target,
                observer,
                et,
                abcorr,
                method,
            )
        };
        if longitude_index(system) == Some(coordinate) {
            return search_longitude_state(
                state,
                |et| {
                    Ok(crate::surface::sub_observer_geometry_in(
                        kernel,
                        pck,
                        frames,
                        target_frame,
                        target,
                        observer,
                        et,
                        abcorr,
                        method,
                    )?
                    .point)
                },
                windows,
                system,
                east,
                condition,
                reference,
                options,
            );
        }
        return search_scalar_with_decreasing(
            |et| {
                let point = crate::surface::sub_observer_geometry_in(
                    kernel,
                    pck,
                    frames,
                    target_frame,
                    target,
                    observer,
                    et,
                    abcorr,
                    method,
                )?;
                coordinate_value_with_pck(point.point, system, coordinate, Some(radii), target, pck)
            },
            |et| {
                let (p, v) = state(et)?;
                coordinate_decreasing(p, v, system, coordinate, Some(radii), east)
            },
            windows,
            condition,
            reference,
            options,
        );
    }
    search_scalar(
        |et| {
            let point = match kind {
                SubPointKind::Observer => crate::surface::sub_observer_geometry_in(
                    kernel,
                    pck,
                    frames,
                    target_frame,
                    target,
                    observer,
                    et,
                    abcorr,
                    method,
                )?,
                SubPointKind::Solar => crate::surface::sub_solar_geometry_in(
                    kernel,
                    pck,
                    frames,
                    target_frame,
                    target,
                    observer,
                    et,
                    abcorr,
                    method,
                )?,
            };
            coordinate_value_with_pck(point.point, system, coordinate, Some(radii), target, pck)
        },
        windows,
        condition,
        reference,
        options,
    )
}

/// A coordinate of a sub-point in the target's IAU frame.
#[allow(clippy::too_many_arguments)]
pub fn find_sub_point_coordinate_with_method(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    kind: SubPointKind,
    method: crate::surface::SubPointMethod,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    find_sub_point_coordinate_in(
        kernel,
        pck,
        &Frames::new(),
        &format!("IAU_{target}"),
        target,
        observer,
        abcorr,
        kind,
        method,
        system,
        coordinate,
        windows,
        condition,
        reference,
        options,
    )
}

/// A coordinate of the surface intercept point of the ray from the
/// observer toward the target centre.
#[allow(clippy::too_many_arguments)]
pub fn find_surface_intercept_coordinate(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    find_sub_point_coordinate_with_method(
        kernel,
        pck,
        target,
        observer,
        abcorr,
        SubPointKind::Observer,
        crate::surface::SubPointMethod::Intercept,
        system,
        coordinate,
        windows,
        condition,
        reference,
        options,
    )
}

/// A coordinate of the surface intercept point of a fixed ray from the
/// observer.
#[allow(clippy::too_many_arguments)]
pub fn find_surface_intercept_ray_coordinate_with_frame(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    direction: [f64; 3],
    direction_frame: &str,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    if frames.center(target_frame)? != target {
        return Err(GeoError::BadSearch(
            "target frame is not centered on target",
        ));
    }
    let east = pck_coordinate_sense(pck, system, target)?;
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let state = |et| {
        ray_intercept_surface_state_with_frame(
            kernel,
            pck,
            frames,
            target_frame,
            target,
            observer,
            et,
            abcorr,
            direction,
            direction_frame,
            radii,
        )
    };
    let mut existence = Vec::new();
    for w in normalize_windows(windows)? {
        existence.extend(solve_boolean(
            &|et| Ok(state(et)?.is_some()),
            w,
            options.step,
            options.tolerance,
        )?);
    }
    let existence = contract_windows(&existence, options.tolerance + 1.0, options.tolerance + 1.0)?;
    if longitude_index(system) == Some(coordinate) {
        return search_longitude_state(
            |et| state(et)?.ok_or(GeoError::BadSearch("intercept state not computable")),
            |et| {
                Ok(intercept_geometry_with_frame(
                    kernel,
                    pck,
                    frames,
                    target_frame,
                    target,
                    observer,
                    et,
                    abcorr,
                    direction,
                    direction_frame,
                )?
                .point)
            },
            &existence,
            system,
            east,
            condition,
            reference,
            options,
        );
    }
    search_scalar_with_decreasing(
        |et| {
            let geometry = intercept_geometry_with_frame(
                kernel,
                pck,
                frames,
                target_frame,
                target,
                observer,
                et,
                abcorr,
                direction,
                direction_frame,
            )?;
            coordinate_value_with_pck(geometry.point, system, coordinate, Some(radii), target, pck)
        },
        |et| {
            let (p, v) = state(et)?.ok_or(GeoError::BadSearch("intercept state not computable"))?;
            coordinate_decreasing(p, v, system, coordinate, Some(radii), east)
        },
        &existence,
        condition,
        reference,
        options,
    )
}

/// A coordinate of a ray in a named direction frame, using the IAU target frame.
#[allow(clippy::too_many_arguments)]
pub fn find_surface_intercept_ray_coordinate_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    direction: [f64; 3],
    direction_frame: &str,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    find_surface_intercept_ray_coordinate_with_frame(
        kernel,
        pck,
        frames,
        &format!("IAU_{target}"),
        target,
        observer,
        abcorr,
        direction,
        direction_frame,
        system,
        coordinate,
        windows,
        condition,
        reference,
        options,
    )
}

/// GFSNTC for WebGeocalc's four fixed-ray direction specifications.
/// Instrument vectors retain their original magnitude. The vector remains in
/// its named source frame so the native state calculation includes its motion.
#[allow(clippy::too_many_arguments)]
pub fn find_surface_intercept_direction_coordinate(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: Option<&crate::Fov>,
    frames: &Frames,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    direction: &crate::RayDirection,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    find_surface_intercept_direction_coordinate_in(
        kernel,
        pck,
        fov,
        frames,
        &format!("IAU_{target}"),
        target,
        observer,
        abcorr,
        direction,
        system,
        coordinate,
        windows,
        condition,
        reference,
        options,
    )
}

/// GFSNTC fixed-ray specification with an explicit target ellipsoid frame.
#[allow(clippy::too_many_arguments)]
pub fn find_surface_intercept_direction_coordinate_in(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: Option<&crate::Fov>,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    direction: &crate::RayDirection,
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    use crate::RayDirection;
    let instrument = || {
        fov.ok_or(GeoError::BadSearch(
            "instrument direction requires an instrument FOV",
        ))
    };
    let (frame,vector)=match direction {
        RayDirection::InstrumentBoresight=>{let fov=instrument()?;(fov.frame.as_str(),fov.original_boresight())},
        RayDirection::VectorInFov {vector}=>{let fov=instrument()?;(fov.frame.as_str(),*vector)},
        RayDirection::FrameAxis {frame,axis,anti}=>{
            let vector=match axis {crate::FrameAxis::X=>[1.,0.,0.],crate::FrameAxis::Y=>[0.,1.,0.],crate::FrameAxis::Z=>[0.,0.,1.]};
            (frame.as_str(),if *anti {vector.map(|x|-x)}else{vector})
        },
        RayDirection::VectorInFrame {frame,vector,anti,stellar_aberration}=>{
            if *stellar_aberration {return Err(GeoError::BadSearch("GF surface intercept requires a fixed vector in its source frame"));}
            (frame.as_str(),if *anti {vector.map(|x|-x)}else{*vector})
        },
        RayDirection::ToObject {..}|RayDirection::InstrumentFovBoundaryVectors=>return Err(GeoError::BadSearch("GF surface intercept accepts only boresight, frame axis, instrument vector, or reference-frame vector")),
    };
    if !vector.iter().all(|v| v.is_finite()) || vector == [0.; 3] {
        return Err(GeoError::BadSearch(
            "ray direction must be finite and nonzero",
        ));
    }
    if !options.step.is_finite()
        || options.step <= 0.
        || !options.tolerance.is_finite()
        || options.tolerance <= 0.
    {
        return Err(GeoError::BadSearch(
            "step and tolerance must be positive and finite",
        ));
    }
    find_surface_intercept_ray_coordinate_with_frame(
        kernel,
        pck,
        frames,
        target_frame,
        target,
        observer,
        abcorr,
        vector,
        frame,
        system,
        coordinate,
        windows,
        condition,
        reference,
        options,
    )
}

/// A coordinate of a ray expressed in J2000.
#[allow(clippy::too_many_arguments)]
pub fn find_surface_intercept_ray_coordinate(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
    system: CoordinateSystem,
    coordinate: usize,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    find_surface_intercept_ray_coordinate_in(
        kernel,
        pck,
        &Frames::new(),
        target,
        observer,
        abcorr,
        direction_j2000,
        "J2000",
        system,
        coordinate,
        windows,
        condition,
        reference,
        options,
    )
}

fn rotate(matrix: &crate::frame::Mat3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn norm(a: [f64; 3]) -> f64 {
    let maximum = a[0].abs().max(a[1].abs()).max(a[2].abs());
    if maximum == 0.0 {
        return 0.0;
    }
    let v = [a[0] / maximum, a[1] / maximum, a[2] / maximum];
    maximum * dot(v, v).sqrt()
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[cfg(test)]
mod tests {
    use super::angle_between;
    use std::f64::consts::PI;

    #[test]
    fn angle_between_axes() {
        assert!((angle_between([1.0, 0.0, 0.0], [1.0, 0.0, 0.0])).abs() < 1e-15);
        assert!((angle_between([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]) - PI / 2.0).abs() < 1e-15);
        assert!((angle_between([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]) - PI).abs() < 1e-15);
        assert!(angle_between([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]).abs() < 1e-15);
    }
}

#[cfg(test)]
mod parity_regressions {
    use super::*;
    fn options() -> FinderOptions {
        FinderOptions {
            step: 1.0,
            tolerance: 1e-6,
            adjustment: 0.0,
        }
    }
    #[test]
    fn equality_has_no_scalar_tolerance_band() {
        let result = search_scalar(
            |t| Ok(1e-9 * t),
            &[Interval {
                start: -10.0,
                end: 10.0,
            }],
            Condition::Equal,
            0.0,
            options(),
        )
        .unwrap();
        assert_eq!(
            result,
            vec![Interval {
                start: -3.129243850708008e-7,
                end: -3.129243850708008e-7
            }]
        );
    }
    #[test]
    fn equality_preserves_every_root_and_closed_endpoint() {
        let result = search_scalar(
            |t| Ok(t * (t - 2.0) * (t - 4.0)),
            &[Interval {
                start: 0.0,
                end: 4.0,
            }],
            Condition::Equal,
            0.0,
            options(),
        )
        .unwrap();
        assert_eq!(
            result,
            vec![
                Interval {
                    start: 3.973639195464784e-8,
                    end: 3.973639195464784e-8
                },
                Interval {
                    start: 2.000000476836931,
                    end: 2.000000476836931
                },
                Interval {
                    start: 3.999999960263608,
                    end: 3.999999960263608
                }
            ]
        );
    }
    #[test]
    fn singleton_confinement_is_searchable() {
        let window = [Interval {
            start: 3.0,
            end: 3.0,
        }];
        assert_eq!(
            search_scalar(Ok, &window, Condition::GreaterThan, 2.0, options()).unwrap(),
            window
        );
    }
    #[test]
    fn overlap_is_normalized_without_closing_small_gaps() {
        let windows = [
            Interval {
                start: 2.0,
                end: 4.0,
            },
            Interval {
                start: 0.0,
                end: 3.0,
            },
            Interval {
                start: 4.0000001,
                end: 5.0,
            },
        ];
        assert_eq!(
            search_scalar(
                |_| Ok(1.0),
                &windows,
                Condition::GreaterThan,
                0.0,
                options()
            )
            .unwrap(),
            vec![
                Interval {
                    start: 0.0,
                    end: 4.0
                },
                windows[2]
            ]
        );
    }
    #[test]
    fn constant_function_has_no_local_extrema() {
        assert!(search_scalar(
            |_| Ok(2.0),
            &[Interval {
                start: 0.0,
                end: 4.0
            }],
            Condition::LocalMinimum,
            0.0,
            options()
        )
        .unwrap()
        .is_empty());
    }
}

#[cfg(test)]
mod window_parity_tests {
    use super::*;
    #[test]
    fn cspice_window_examples() {
        let w = [
            Interval {
                start: 1.0,
                end: 3.0,
            },
            Interval {
                start: 7.0,
                end: 11.0,
            },
            Interval {
                start: 23.0,
                end: 27.0,
            },
            Interval {
                start: 29.0,
                end: 29.0,
            },
        ];
        assert_eq!(
            contract_windows(&w, 2.0, 1.0).unwrap(),
            vec![
                Interval {
                    start: 9.0,
                    end: 10.0
                },
                Interval {
                    start: 25.0,
                    end: 26.0
                }
            ]
        );
        assert_eq!(
            complement_windows(&w, 0.0, 30.0).unwrap(),
            vec![
                Interval {
                    start: 0.0,
                    end: 1.0
                },
                Interval {
                    start: 3.0,
                    end: 7.0
                },
                Interval {
                    start: 11.0,
                    end: 23.0
                },
                Interval {
                    start: 27.0,
                    end: 30.0
                }
            ]
        );
        assert_eq!(filter_windows(&w, 4.0).unwrap(), Vec::<Interval>::new());
        assert_eq!(
            fill_windows(&w, 4.0).unwrap(),
            vec![
                Interval {
                    start: 1.0,
                    end: 11.0
                },
                Interval {
                    start: 23.0,
                    end: 29.0
                }
            ]
        );
    }
    #[test]
    fn range_and_wrapped_range() {
        let w = [Interval {
            start: -4.0,
            end: 4.0,
        }];
        let opt = FinderOptions {
            step: 0.5,
            tolerance: 1e-6,
            adjustment: 0.0,
        };
        assert_eq!(
            search_scalar_constraint(
                Ok,
                &w,
                ScalarConstraint::Range {
                    lower: -1.0,
                    upper: 2.0
                },
                opt
            )
            .unwrap(),
            vec![Interval {
                start: -1.0000004768371582,
                end: 2.00000022649769
            }]
        );
        assert!(matches!(
            search_scalar_constraint(
                Ok,
                &w,
                ScalarConstraint::AngularRange {
                    lower: 3.0,
                    upper: -3.0,
                    period: 8.0
                },
                opt
            ),
            Err(GeoError::Unsupported(_))
        ));
    }
}

/// Canonicalize a finite closed window, merging overlapping and touching intervals.
/// No convergence tolerance is applied to window set operations (CSPICE WNVALD).
pub fn normalize_windows(windows: &[Interval]) -> Result<Vec<Interval>, GeoError> {
    if windows
        .iter()
        .any(|w| !w.start.is_finite() || !w.end.is_finite() || w.start > w.end)
    {
        return Err(GeoError::BadSearch(
            "window endpoints must be finite and ordered",
        ));
    }
    let mut sorted = windows.to_vec();
    sorted.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut result: Vec<Interval> = Vec::new();
    for w in sorted {
        if let Some(last) = result.last_mut() {
            if w.start <= last.end {
                last.end = last.end.max(w.end);
                continue;
            }
        }
        result.push(w);
    }
    Ok(result)
}

/// Closure of the complement in `[left,right]`, following CSPICE WNCOMD.
pub fn complement_windows(
    windows: &[Interval],
    left: f64,
    right: f64,
) -> Result<Vec<Interval>, GeoError> {
    normalize_windows(&[Interval {
        start: left,
        end: right,
    }])?;
    let w = normalize_windows(windows)?;
    if w.is_empty() || w[0].start >= right || w.last().unwrap().end <= left {
        return Ok(vec![Interval {
            start: left,
            end: right,
        }]);
    }
    // Preserve WNCOMD's endpoint selection, including input intervals
    // outside the bounding interval; do not replace it with a clipped
    // mathematical set difference.
    let mut result = Vec::new();
    let mut i = 0;
    while i < w.len() && w[i].end < left {
        i += 1;
    }
    if i < w.len() && w[i].start > left {
        result.push(Interval {
            start: left,
            end: w[i].start,
        });
    }
    while i + 1 < w.len() && w[i + 1].start < right {
        result.push(Interval {
            start: w[i].end,
            end: w[i + 1].start,
        });
        i += 1;
    }
    if i < w.len() && w[i].end < right {
        result.push(Interval {
            start: w[i].end,
            end: right,
        });
    }
    normalize_windows(&result)
}

/// Expand each endpoint by signed amounts, dropping reversed intervals (WNEXPD).
pub fn expand_windows(
    windows: &[Interval],
    left: f64,
    right: f64,
) -> Result<Vec<Interval>, GeoError> {
    if !left.is_finite() || !right.is_finite() {
        return Err(GeoError::BadSearch("window offsets must be finite"));
    }
    let result: Vec<_> = normalize_windows(windows)?
        .into_iter()
        .filter_map(|w| {
            let (start, end) = (w.start - left, w.end + right);
            (start <= end).then_some(Interval { start, end })
        })
        .collect();
    normalize_windows(&result)
}

/// Contract each endpoint by signed amounts (WNCOND).
pub fn contract_windows(
    windows: &[Interval],
    left: f64,
    right: f64,
) -> Result<Vec<Interval>, GeoError> {
    expand_windows(windows, -left, -right)
}

/// Fill gaps with width at most `gap` (WNFILD).
pub fn fill_windows(windows: &[Interval], gap: f64) -> Result<Vec<Interval>, GeoError> {
    if !gap.is_finite() {
        return Err(GeoError::BadSearch("gap must be finite"));
    }
    let mut result: Vec<Interval> = Vec::new();
    for w in normalize_windows(windows)? {
        if let Some(last) = result.last_mut() {
            if last.end + gap >= w.start {
                last.end = w.end;
                continue;
            }
        }
        result.push(w);
    }
    Ok(result)
}

/// Remove intervals with width at most `width` (WNFLTD).
pub fn filter_windows(windows: &[Interval], width: f64) -> Result<Vec<Interval>, GeoError> {
    if !width.is_finite() {
        return Err(GeoError::BadSearch("filter width must be finite"));
    }
    Ok(normalize_windows(windows)?
        .into_iter()
        .filter(|w| w.end - w.start > width)
        .collect())
}

/// Typed scalar or periodic range, alongside the CSPICE RELATE conditions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarConstraint {
    /// A CSPICE relation with a reference value.
    Relation {
        condition: Condition,
        reference: f64,
    },
    /// Inclusive lower and upper bounds in scalar units.
    Range { lower: f64, upper: f64 },
    /// Inclusive positive-direction arc; `lower > upper` crosses the wrap.
    AngularRange { lower: f64, upper: f64, period: f64 },
}

/// Native GFUDS/UDDC with a linear relation or the source serial RANGE schedule.
/// Periodic ranges require a native coordinate domain and are unsupported here;
/// use `apply_scalar_constraint` with the corresponding native finder instead.
pub fn search_scalar_constraint<F>(
    evaluate: F,
    windows: &[Interval],
    constraint: ScalarConstraint,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError>
where
    F: Fn(f64) -> Result<f64, GeoError>,
{
    if matches!(constraint, ScalarConstraint::AngularRange { .. }) {
        return Err(GeoError::Unsupported("periodic value-only searches require an explicit native coordinate domain; use apply_scalar_constraint".into()));
    }
    crate::gf_constraints::apply_scalar_constraint(
        |condition, reference, cnfine| {
            search_scalar(&evaluate, cnfine, condition, reference, options)
        },
        windows,
        constraint,
        crate::gf_constraints::ScalarDomain::Linear,
    )
}

#[cfg(test)]
mod pck_coordinate_regressions {
    use super::*;
    #[test]
    fn gf_planetographic_uses_loaded_pck_longitude_direction() {
        let kernel = SpkKernel::open("tests/golden/gf_fixed.bsp").unwrap();
        let pck = Pck::from_text("\\begindata\nBODY299_RADII = (6051.8 6051.8 6051.8)\nBODY299_POLE_RA = (0 0 0)\nBODY299_POLE_DEC = (90 0 0)\nBODY299_PM = (0 -1 0)\n").unwrap();
        assert_eq!(
            coordinate_value_with_pck(
                [1e8; 3],
                CoordinateSystem::Planetographic,
                0,
                pck.radii(299),
                299,
                &pck
            )
            .unwrap(),
            PI / 4.0
        );
        // GF equality is a crossing search; a constant scalar has no roots.
        let frames = Frames::from_text(
            r#"\begindata
FRAME_VENUS_FIXED_TEST=1499299
FRAME_1499299_NAME='VENUS_FIXED_TEST'
FRAME_1499299_CLASS=4
FRAME_1499299_CLASS_ID=1499299
FRAME_1499299_CENTER=299
TKFRAME_1499299_RELATIVE='J2000'
TKFRAME_1499299_SPEC='ANGLES'
TKFRAME_1499299_ANGLES=(0 0 0)
TKFRAME_1499299_AXES=(1 2 3)
"#,
        )
        .unwrap();
        let window = [Interval {
            start: 0.0,
            end: 0.0,
        }];
        let result = find_position_coordinate_with_radii(
            &kernel,
            &pck,
            299,
            0,
            Aberration::None,
            &frames,
            "VENUS_FIXED_TEST",
            CoordinateSystem::Planetographic,
            0,
            &window,
            Condition::LessThan,
            PI / 2.0,
            FinderOptions::default(),
        )
        .unwrap();
        assert_eq!(result, window);
    }
}

#[cfg(test)]
mod exact_cspice_window_golden {
    use super::*;
    fn parse(text: &str) -> Vec<Interval> {
        if text.is_empty() {
            return vec![];
        }
        text.split(';')
            .map(|pair| {
                let (a, b) = pair.split_once(':').unwrap();
                Interval {
                    start: a.parse().unwrap(),
                    end: b.parse().unwrap(),
                }
            })
            .collect()
    }
    #[test]
    fn exact_cspice_window_operations() {
        let mut compared = 0;
        for line in include_str!("../../tests/golden/gf_windows.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let (a, b) = (f[1].parse().unwrap(), f[2].parse().unwrap());
            let input = parse(f[3]);
            let actual = match f[0] {
                "normalize" => normalize_windows(&input),
                "complement" => complement_windows(&input, a, b),
                "expand" => expand_windows(&input, a, b),
                "contract" => contract_windows(&input, a, b),
                "fill" => fill_windows(&input, a),
                "filter" => filter_windows(&input, a),
                _ => panic!("unknown operation"),
            }
            .unwrap();
            assert_eq!(actual, parse(f[4]), "{line}");
            compared += 1;
        }
        assert_eq!(compared, 320);
    }
}

#[cfg(test)]
mod solar_observer_regressions {
    use super::*;
    #[test]
    fn solar_coordinate_follows_native_uddc_singleton_contract() {
        let Ok(kernel) = SpkKernel::open("target/kernels/de440s.bsp") else {
            return;
        };
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let point = crate::surface::sub_solar_geometry(
            &kernel,
            &pck,
            301,
            399,
            0.0,
            Aberration::ConvergedLightTime,
            crate::surface::SubPointMethod::NearPoint,
        )
        .unwrap();
        let reference = point.point[0];
        let native_reference = include_str!("../../tests/golden/gf_uddc_regressions.csv")
            .lines()
            .find(|line| line.starts_with("solar_sublr,"))
            .unwrap()
            .split(',')
            .nth(1)
            .unwrap()
            .parse::<f64>()
            .unwrap();
        assert_eq!(reference.to_bits(), native_reference.to_bits());
        let window = [Interval {
            start: 0.0,
            end: 0.0,
        }];
        let result = find_sub_point_coordinate(
            &kernel,
            &pck,
            301,
            399,
            Aberration::ConvergedLightTime,
            SubPointKind::Solar,
            CoordinateSystem::Rectangular,
            0,
            &window,
            Condition::Equal,
            reference,
            FinderOptions::default(),
        )
        .unwrap();
        // Native GFUDS with the same SUBSLR callback and UDDC(1s) returns
        // an empty result: its equality root is outside this singleton.
        assert_eq!(result, Vec::<Interval>::new());
    }
}

#[cfg(test)]
mod scalar_cspice_golden {
    use super::*;
    #[test]
    fn gfuds_exact_linear_and_quadratic() {
        let w = [Interval {
            start: -4.0,
            end: 4.0,
        }];
        let opt = FinderOptions {
            step: 1.0,
            tolerance: 1e-6,
            adjustment: 0.0,
        };
        assert_eq!(
            search_scalar_with_decreasing(
                |t| Ok(1e-9 * t),
                |_| Ok(false),
                &w,
                Condition::Equal,
                0.0,
                opt
            )
            .unwrap(),
            vec![Interval {
                start: -2.682209014892578e-7,
                end: -2.682209014892578e-7
            }]
        );
        assert_eq!(
            search_scalar_with_decreasing(
                |t| Ok(t * t),
                |t| Ok(t < 0.0),
                &w,
                Condition::LocalMinimum,
                0.0,
                opt
            )
            .unwrap(),
            vec![Interval {
                start: -4.76837158203125e-7,
                end: -4.76837158203125e-7
            }]
        );
    }
}

fn intersect_windows(a: &[Interval], b: &[Interval]) -> Result<Vec<Interval>, GeoError> {
    let mut out = Vec::new();
    for x in a {
        for y in b {
            let (start, end) = (x.start.max(y.start), x.end.min(y.end));
            if start <= end {
                out.push(Interval { start, end });
            }
        }
    }
    normalize_windows(&out)
}

fn difference_windows(a: &[Interval], b: &[Interval]) -> Result<Vec<Interval>, GeoError> {
    crate::gf_constraints::relative_complement(b, a)
}

// Literal midpoint refinement and interval stepping from N0067 ZZGFSOLV.
pub(crate) fn solve_boolean<F>(
    predicate: &F,
    window: Interval,
    step: f64,
    tolerance: f64,
) -> Result<Vec<Interval>, GeoError>
where
    F: Fn(f64) -> Result<bool, GeoError>,
{
    let mut t = window.start;
    let mut state = predicate(t)?;
    let mut begin = t;
    let mut out = Vec::new();
    while t < window.end {
        let next = (t + step).min(window.end);
        if next == t {
            return Err(GeoError::BadSearch("search step does not advance time"));
        }
        let next_state = predicate(next)?;
        if state != next_state {
            let (mut left, mut right) = (t, next);
            let mut previous = f64::MAX;
            let mut difference = (right - left).abs();
            while difference > tolerance && difference < previous {
                let middle = (left + right) * 0.5;
                if middle == left {
                    right = middle;
                } else if middle == right {
                    left = middle;
                } else if predicate(middle)? == state {
                    left = middle;
                } else {
                    right = middle;
                }
                previous = difference;
                difference = (right - left).abs();
            }
            let transition = ((left + right) * 0.5).clamp(left, right);
            if state {
                out.push(Interval {
                    start: begin,
                    end: transition,
                });
            } else {
                begin = transition;
            }
        }
        t = next;
        state = next_state;
    }
    if state {
        out.push(Interval {
            start: begin,
            end: window.end,
        });
    }
    normalize_windows(&out)
}

/// CSPICE GFUDS search with an explicit decreasing predicate.
///
/// Implements N0067 ZZGFREL/ZZGFSOLV schedules, equality-window expansion,
/// midpoint refinement, and absolute-extremum tie selection. The caller
/// supplies the same decreasing predicate as CSPICE; it is not estimated.
pub fn search_scalar_with_decreasing<F, D>(
    evaluate: F,
    decreasing: D,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError>
where
    F: Fn(f64) -> Result<f64, GeoError>,
    D: Fn(f64) -> Result<bool, GeoError>,
{
    if !options.step.is_finite()
        || options.step <= 0.0
        || !options.tolerance.is_finite()
        || options.tolerance <= 0.0
        || !options.adjustment.is_finite()
        || options.adjustment < 0.0
        || !reference.is_finite()
    {
        return Err(GeoError::BadSearch("invalid scalar search controls"));
    }
    let absolute = matches!(
        condition,
        Condition::AbsoluteMinimum | Condition::AbsoluteMaximum
    );
    if !absolute && options.adjustment != 0.0 {
        return Err(GeoError::BadSearch(
            "adjustment applies only to absolute extrema",
        ));
    }
    let original = normalize_windows(windows)?;
    let expanded = if condition == Condition::Equal {
        expand_windows(&original, 0.5, 0.5)?
    } else {
        original.clone()
    };
    let mut dec = Vec::new();
    for w in &expanded {
        dec.extend(solve_boolean(
            &decreasing,
            *w,
            options.step,
            options.tolerance,
        )?);
    }
    let dec = normalize_windows(&dec)?;
    if matches!(condition, Condition::LocalMinimum | Condition::LocalMaximum) {
        let out: Vec<_> = dec
            .iter()
            .filter_map(|w| {
                let t = if condition == Condition::LocalMinimum {
                    w.end
                } else {
                    w.start
                };
                original
                    .iter()
                    .any(|x| x.start < t && t < x.end)
                    .then_some(Interval { start: t, end: t })
            })
            .collect();
        return normalize_windows(&out);
    }
    let inc = difference_windows(&expanded, &dec)?;
    let mut threshold = reference;
    if absolute {
        let d = intersect_windows(&dec, &original)?;
        let i = difference_windows(&original, &d)?;
        let maximum = condition == Condition::AbsoluteMaximum;
        let mut best = if maximum {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        let mut candidate = None;
        for (schedule, is_decreasing) in [(&d, true), (&i, false)] {
            for w in schedule {
                let t = if maximum == is_decreasing {
                    w.start
                } else {
                    w.end
                };
                let value = evaluate(t)?;
                if (maximum && value >= best) || (!maximum && value <= best) {
                    best = value;
                    candidate = Some(t);
                }
            }
        }
        let Some(t) = candidate else {
            return Ok(vec![]);
        };
        if options.adjustment == 0.0 {
            return Ok(vec![Interval { start: t, end: t }]);
        }
        threshold = if maximum {
            best - options.adjustment
        } else {
            best + options.adjustment
        };
    }
    let less = |t| Ok(evaluate(t)? < threshold);
    let mut result = Vec::new();
    for schedule in [&dec, &inc] {
        for w in schedule {
            result.extend(solve_boolean(
                &less,
                *w,
                w.end - w.start,
                options.tolerance,
            )?);
        }
    }
    let result = normalize_windows(&result)?;
    match condition {
        Condition::LessThan | Condition::AbsoluteMinimum => intersect_windows(&original, &result),
        Condition::GreaterThan | Condition::AbsoluteMaximum => {
            difference_windows(&original, &result)
        }
        Condition::Equal => {
            let endpoints: Vec<_> = result
                .iter()
                .flat_map(|w| {
                    [
                        Interval {
                            start: w.start,
                            end: w.start,
                        },
                        Interval {
                            start: w.end,
                            end: w.end,
                        },
                    ]
                })
                .collect();
            intersect_windows(&original, &endpoints)
        }
        _ => unreachable!(),
    }
}

// CSPICE DVHAT/VPERP/VPROJ operation ordering, used by GF derivatives.
fn unit_state(position: [f64; 3], velocity: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let length = norm(position);
    if length == 0.0 {
        return ([0.0; 3], velocity);
    }
    let unit = position.map(|x| x / length);
    let amax = velocity.iter().map(|x| x.abs()).fold(0.0, f64::max);
    if amax == 0.0 {
        return (unit, [0.0; 3]);
    }
    let bmax = unit.iter().map(|x| x.abs()).fold(0.0, f64::max);
    let a = velocity.map(|x| x / amax);
    let b = unit.map(|x| x / bmax);
    let a2max = a.iter().map(|x| x.abs()).fold(0.0, f64::max);
    let b2max = b.iter().map(|x| x.abs()).fold(0.0, f64::max);
    let aa = a.map(|x| x / a2max);
    let bb = b.map(|x| x / b2max);
    let projection_scale = dot(aa, bb) * a2max / dot(bb, bb);
    let projection = bb.map(|x| projection_scale * x);
    let perpendicular = subtract(a, projection).map(|x| amax * x);
    (unit, perpendicular.map(|x| (1.0 / length) * x))
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let am = a.iter().map(|x| x.abs()).fold(0.0, f64::max);
    let bm = b.iter().map(|x| x.abs()).fold(0.0, f64::max);
    if am == 0.0 || bm == 0.0 {
        return [0.0; 3];
    }
    let c = cross(a.map(|x| x / am), b.map(|x| x / bm));
    let n = norm(c);
    if n == 0.0 {
        [0.0; 3]
    } else {
        c.map(|x| x / n)
    }
}
fn coordinate_decreasing(
    p: [f64; 3],
    v: [f64; 3],
    system: CoordinateSystem,
    index: usize,
    radii: Option<[f64; 3]>,
    east: bool,
) -> Result<bool, GeoError> {
    if index > 2 {
        return Err(GeoError::BadSearch("coordinate index out of range"));
    }
    // ZZGFCPRX normalizes velocity before all sign projections.
    let velocity_norm = norm(v);
    if velocity_norm == 0.0 {
        return Ok(false);
    }
    let v = v.map(|x| x / velocity_norm);
    if system == CoordinateSystem::Rectangular {
        return Ok(v[index] < 0.0);
    }
    let n = norm(p);
    if n == 0.0 {
        return Ok(false);
    }
    let normal = if matches!(
        system,
        CoordinateSystem::Geodetic | CoordinateSystem::Planetographic
    ) {
        let r = radii.ok_or(GeoError::BadSearch("ellipsoidal coordinates require radii"))?;
        let [lon, lat, _] = crate::coords::rectangular_to_planetodetic(p, r)?;
        [lon.cos() * lat.cos(), lon.sin() * lat.cos(), lat.sin()]
    } else if system == CoordinateSystem::Cylindrical {
        let r = norm([p[0], p[1], 0.0]);
        if r == 0.0 {
            return Ok(index == 2 && v[2] < 0.0);
        }
        [p[0] / r, p[1] / r, 0.0]
    } else {
        p.map(|x| x / n)
    };
    if p[0] == 0.0 && p[1] == 0.0 {
        let radial_index = match system {
            CoordinateSystem::Geodetic
            | CoordinateSystem::Planetographic
            | CoordinateSystem::AzimuthElevation { .. } => 2,
            _ => 0,
        };
        return Ok(index == radial_index && dot(p, v) < 0.0);
    }
    let lon = normal[1].atan2(normal[0]);
    let e = unit_cross([0.0, 0.0, 1.0], [lon.cos(), lon.sin(), 0.0]);
    let north = unit_cross(normal, e);
    let nr = norm(normal);
    let radial = normal.map(|x| x / nr);
    let r = dot(radial, v);
    let ev = dot(e, v);
    let nv = dot(north, v);
    let derivative = match system {
        CoordinateSystem::Latitudinal | CoordinateSystem::RightAscensionDeclination => {
            [r, ev, nv][index]
        }
        CoordinateSystem::Spherical => [r, -nv, ev][index],
        CoordinateSystem::Cylindrical => [r, ev, v[2]][index],
        CoordinateSystem::Geodetic => [ev, nv, r][index],
        CoordinateSystem::Planetographic => [if east { ev } else { -ev }, nv, r][index],
        CoordinateSystem::AzimuthElevation { az_ccw, el_plus_z } => [
            if az_ccw { ev } else { -ev },
            if el_plus_z { nv } else { -nv },
            r,
        ][index],
        CoordinateSystem::Rectangular => unreachable!(),
    };
    Ok(derivative < 0.0)
}
fn transformed_state(
    kernel: &SpkKernel,
    frames: &Frames,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frame: &str,
    pck: Option<&Pck>,
) -> Result<([f64; 3], [f64; 3]), GeoError> {
    if let Some(pck) = pck {
        let state = crate::location::state_between_in(
            kernel,
            frames,
            pck,
            crate::location::Endpoint::Object(target),
            crate::location::Endpoint::Object(observer),
            crate::location::OutputFrame::Spice(frame),
            crate::location::FrameLocus::Center,
            et,
            abcorr,
        )?
        .state;
        return Ok((state.position, state.velocity));
    }
    if frames.is_inertial(frame)? {
        let matrix = frames.pxform("J2000", frame, et)?;
        let state = kernel
            .state_corrected_inertial(target, observer, et, abcorr, &matrix)?
            .state;
        return Ok((state.position, state.velocity));
    }
    let s = kernel.state_corrected(target, observer, et, abcorr)?.state;
    let x = frames.sxform("J2000", frame, et)?;
    let input = [
        s.position[0],
        s.position[1],
        s.position[2],
        s.velocity[0],
        s.velocity[1],
        s.velocity[2],
    ];
    let out: [f64; 6] = std::array::from_fn(|i| {
        let mut sum = 0.0;
        for j in 0..6 {
            sum += x[i][j] * input[j];
        }
        sum
    });
    Ok(([out[0], out[1], out[2]], [out[3], out[4], out[5]]))
}

fn body_observer_state_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
) -> Result<([f64; 3], [f64; 3]), GeoError> {
    let state = kernel.state_in_frame(target, observer, et, frames, Some(pck), "J2000")?;
    let orientation = frames.pxform_with_derivative("J2000", target_frame, et, Some(pck))?;
    let (p, v) = transform_surface_state(
        &orientation.rotation,
        &orientation.rotation_dot,
        state.position,
        state.velocity,
        false,
    );
    Ok((p.map(|x| -x), v.map(|x| -x)))
}
// N0067 ZZDNPT (derivative of NEARPT); off-diagonal matrix entries are zero.
pub(crate) fn nearpoint_velocity(
    p: [f64; 3],
    v: [f64; 3],
    point: [f64; 3],
    radii: [f64; 3],
) -> Result<[f64; 3], GeoError> {
    let g = radii.map(|r| 1.0 / (r * r));
    let grad = std::array::from_fn(|i| g[i] * point[i]);
    let length = norm(grad);
    let normal = grad.map(|x| x / length);
    let l = dot(subtract(p, point), normal) / length;
    let diagonal = g.map(|x| l * x + 1.0);
    if diagonal.contains(&0.0) {
        return Err(GeoError::BadSearch("near-point velocity is undefined"));
    }
    let m = diagonal.map(|x| 1.0 / x);
    let vtmv = |a: [f64; 3], b: [f64; 3]| {
        let mut x = 0.0;
        for k in 0..3 {
            for j in 0..3 {
                x += a[k] * (if k == j { m[k] } else { 0.0 }) * b[j];
            }
        }
        x
    };
    let denominator = vtmv(grad, grad);
    if denominator == 0.0 {
        return Err(GeoError::BadSearch("near-point velocity is undefined"));
    }
    let lp = vtmv(grad, v) / denominator;
    let temp: [f64; 3] = std::array::from_fn(|i| v[i] + (-lp) * grad[i]);
    Ok(std::array::from_fn(|i| {
        let row: [f64; 3] = std::array::from_fn(|j| if i == j { m[i] } else { 0.0 });
        dot(row, temp)
    }))
}

// Geometric ZZGFSSIN/SURFPV state for a direction fixed in J2000.
fn ray_intercept_state_none(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    direction: [f64; 3],
    direction_velocity: [f64; 3],
    radii: [f64; 3],
) -> Result<Option<([f64; 3], [f64; 3])>, GeoError> {
    let (p, v) = body_observer_state_in(kernel, pck, frames, target_frame, target, observer, et)?;
    let orientation = frames.pxform_with_derivative("J2000", target_frame, et, Some(pck))?;
    let (rotation, derivative) = (orientation.rotation, orientation.rotation_dot);
    let (dp, dv) =
        transform_surface_state(&rotation, &derivative, direction, direction_velocity, false);
    let Some(point) = crate::surface::ray_surface_intersection(radii, p, dp)? else {
        return Ok(None);
    };
    let Some(velocity) = surface_velocity(p, v, dp, dv, point, radii)? else {
        return Ok(None);
    };
    Ok(Some((point, velocity)))
}

#[cfg(test)]
mod gfuds_all_conditions_golden {
    use super::*;
    #[test]
    fn exact_cspice_gfuds_conditions() {
        let mut compared = 0;
        for line in include_str!("../../tests/golden/gf_scalars.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let function = |t: f64| {
                Ok(match f[0] {
                    "small_linear" => 1e-9 * t,
                    "linear" => t,
                    "quadratic" => t * t,
                    "negative_quadratic" => -t * t,
                    "cubic" => t * (t - 2.0) * (t + 2.0),
                    "constant" => 2.0,
                    _ => panic!("unknown function"),
                })
            };
            let decreasing = |t: f64| {
                Ok(match f[0] {
                    "quadratic" => t < 0.0,
                    "negative_quadratic" => t > 0.0,
                    "cubic" => 3.0 * t * t - 4.0 < 0.0,
                    _ => false,
                })
            };
            let expected: Vec<_> = if f[8].is_empty() {
                vec![]
            } else {
                f[8].split(';')
                    .map(|s| {
                        let (a, b) = s.split_once(':').unwrap();
                        Interval {
                            start: a.parse().unwrap(),
                            end: b.parse().unwrap(),
                        }
                    })
                    .collect()
            };
            let found = search_scalar_with_decreasing(
                function,
                decreasing,
                &[Interval {
                    start: f[4].parse().unwrap(),
                    end: f[5].parse().unwrap(),
                }],
                Condition::parse(f[1]).unwrap(),
                f[2].parse().unwrap(),
                FinderOptions {
                    step: f[6].parse().unwrap(),
                    tolerance: f[7].parse().unwrap(),
                    adjustment: f[3].parse().unwrap(),
                },
            )
            .unwrap();
            assert_eq!(found, expected, "{line}");
            compared += 1;
        }
        assert_eq!(compared, 54);
    }
}

fn longitude_index(system: CoordinateSystem) -> Option<usize> {
    match system {
        CoordinateSystem::Latitudinal
        | CoordinateSystem::RightAscensionDeclination
        | CoordinateSystem::Cylindrical => Some(1),
        CoordinateSystem::Spherical => Some(2),
        CoordinateSystem::Geodetic | CoordinateSystem::Planetographic => Some(0),
        _ => None,
    }
}

// N0067 ZZGFLONG: trigonometric proxy searches and branch-avoiding sectors.
fn search_longitude_state<F, G>(
    state: F,
    position: G,
    windows: &[Interval],
    system: CoordinateSystem,
    east: bool,
    mut condition: Condition,
    mut reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError>
where
    F: Fn(f64) -> Result<([f64; 3], [f64; 3]), GeoError>,
    G: Fn(f64) -> Result<[f64; 3], GeoError>,
{
    let windows = normalize_windows(windows)?;
    let ra = matches!(
        system,
        CoordinateSystem::RightAscensionDeclination
            | CoordinateSystem::Cylindrical
            | CoordinateSystem::Planetographic
    );
    if (ra && reference.cos() == 1.0) || (!ra && reference.cos() == -1.0) {
        if condition == Condition::LessThan {
            return Ok(if ra { vec![] } else { windows });
        }
        if condition == Condition::GreaterThan {
            return Ok(if ra { windows } else { vec![] });
        }
    }
    if system == CoordinateSystem::Planetographic && !east {
        condition = match condition {
            Condition::AbsoluteMaximum => Condition::AbsoluteMinimum,
            Condition::AbsoluteMinimum => Condition::AbsoluteMaximum,
            Condition::LocalMaximum => Condition::LocalMinimum,
            Condition::LocalMinimum => Condition::LocalMaximum,
            Condition::GreaterThan => Condition::LessThan,
            Condition::LessThan => Condition::GreaterThan,
            c => c,
        };
        if matches!(
            condition,
            Condition::GreaterThan | Condition::LessThan | Condition::Equal
        ) {
            reference = 2.0 * PI - reference;
        }
    }
    let opt = FinderOptions {
        tolerance: 1e-7_f64.max(options.tolerance / 10.0),
        adjustment: 0.0,
        ..options
    };
    let point_longitude = |p: [f64; 3], positive: bool| {
        let mut angle = if p[0] == 0.0 && p[1] == 0.0 {
            0.0
        } else {
            p[1].atan2(p[0])
        };
        if positive && angle < 0.0 {
            angle += 2.0 * PI;
        }
        angle
    };
    let q = |t: f64, positive: bool| -> Result<f64, GeoError> {
        Ok(point_longitude(position(t)?, positive))
    };
    let state_sign = |p, v| -> Result<f64, GeoError> {
        if coordinate_decreasing(p, v, CoordinateSystem::Latitudinal, 1, None, true)? {
            Ok(-1.0)
        } else if coordinate_decreasing(
            p,
            v.map(|x| -x),
            CoordinateSystem::Latitudinal,
            1,
            None,
            true,
        )? {
            Ok(1.0)
        } else {
            Ok(0.0)
        }
    };
    let sign = |t: f64| -> Result<f64, GeoError> {
        let (p, v) = state(t)?;
        state_sign(p, v)
    };
    // mode 0=sin, 1=cos, 2=signed longitude, 3=RA.
    let proxy = |mode: usize,
                 rel: Condition,
                 value: f64,
                 w: &[Interval]|
     -> Result<Vec<Interval>, GeoError> {
        search_scalar_with_decreasing(
            |t| {
                let angle = q(t, if mode < 2 { ra } else { mode == 3 })?;
                Ok(match mode {
                    0 => angle.sin(),
                    1 => angle.cos(),
                    _ => angle,
                })
            },
            |t| {
                // ZZGFCOSD/ZZGFCOCD obtain both the coordinate and its
                // derivative sign from ZZGFCOST. Its position can differ
                // from the separate SINCPT value callback for LT searches.
                let (p, v) = state(t)?;
                let angle = point_longitude(p, if mode < 2 { ra } else { mode == 3 });
                let s = state_sign(p, v)?;
                Ok(match mode {
                    0 => angle.cos() * s < 0.0,
                    1 => -angle.sin() * s < 0.0,
                    _ => s < 0.0,
                })
            },
            w,
            rel,
            value,
            opt,
        )
    };
    if condition == Condition::Equal {
        let (cv, sv) = (reference.cos(), reference.sin());
        let cosine = sv.abs() >= 2.0_f64.sqrt() / 2.0;
        let roots = proxy(
            if cosine { 1 } else { 0 },
            Condition::Equal,
            if cosine { cv } else { sv },
            &windows,
        )?;
        let mut result = Vec::new();
        for root in roots {
            let angle = q(root.start, ra)?;
            let other = if cosine { angle.sin() } else { angle.cos() };
            let cmp = if cosine { sv } else { cv };
            if (cmp >= 0.0 && other >= 0.0) || (cmp < 0.0 && other < 0.0) {
                result.push(Interval {
                    start: root.start,
                    end: root.start,
                });
            }
        }
        return normalize_windows(&result);
    }
    if matches!(condition, Condition::LocalMinimum | Condition::LocalMaximum) {
        let left = proxy(1, Condition::LessThan, 1e-12_f64.cos(), &windows)?;
        let compl = proxy(1, Condition::LessThan, -1e-12_f64.cos(), &left)?;
        let right = difference_windows(&windows, &compl)?;
        let mut result = proxy(3, condition, 0.0, &left)?;
        result.extend(proxy(2, condition, 0.0, &right)?);
        return normalize_windows(&result);
    }
    let top = proxy(0, Condition::GreaterThan, 1e-11_f64.sin(), &windows)?;
    let bottom = proxy(0, Condition::LessThan, -1e-11_f64.sin(), &windows)?;
    let right = proxy(1, Condition::GreaterThan, 0.0, &windows)?;
    let left = difference_windows(&windows, &right)?;
    let q1 = intersect_windows(&right, &top)?;
    let q2 = difference_windows(&top, &right)?;
    let q3 = difference_windows(&bottom, &right)?;
    let q4 = intersect_windows(&right, &bottom)?;
    let union = |a: &[Interval], b: &[Interval]| {
        let mut out = a.to_vec();
        out.extend_from_slice(b);
        normalize_windows(&out)
    };
    if matches!(
        condition,
        Condition::AbsoluteMinimum | Condition::AbsoluteMaximum
    ) {
        let maximum = condition == Condition::AbsoluteMaximum;
        let regions = match (ra, maximum) {
            (false, true) => [(&q2, 3), (&right, 2), (&q3, 3)],
            (false, false) => [(&q3, 3), (&right, 2), (&q2, 3)],
            (true, true) => [(&q4, 2), (&left, 3), (&q1, 2)],
            (true, false) => [(&q1, 2), (&left, 3), (&q4, 2)],
        };
        let mut result = Vec::new();
        for (region, mode) in regions {
            // ZZGFLONG passes cosine as the extremum value callback,
            // while retaining the longitude derivative sign.
            result = search_scalar_with_decreasing(
                |t| Ok(q(t, mode == 3)?.cos()),
                |t| Ok(sign(t)? < 0.0),
                region,
                condition,
                0.0,
                opt,
            )?;
            if !result.is_empty() {
                break;
            }
        }
        if options.adjustment == 0.0 || result.is_empty() {
            return Ok(result);
        }
        let extreme = q(result[0].start, ra)?;
        reference = if maximum {
            (extreme - options.adjustment).max(if ra { 0.0 } else { -PI })
        } else {
            (extreme + options.adjustment).min(if ra { 2.0 * PI } else { PI })
        };
        condition = if maximum {
            Condition::GreaterThan
        } else {
            Condition::LessThan
        };
    }
    let loc = reference.sin().atan2(reference.cos());
    let quadrant = if loc < -PI / 2.0 {
        3
    } else if loc < 0.0 {
        4
    } else if loc < PI / 2.0 {
        1
    } else {
        2
    };
    let (sector, mode) = if !ra {
        match quadrant {
            1 | 4 => (&right, 2),
            2 => (&q2, 3),
            _ => (&q3, 3),
        }
    } else {
        match quadrant {
            1 => (&q1, 2),
            2 | 3 => (&left, 3),
            _ => (&q4, 2),
        }
    };
    let value = if mode == 3 && loc < 0.0 {
        loc + 2.0 * PI
    } else {
        loc
    };
    let automatic = if !ra {
        match quadrant {
            1 | 4 => q3.clone(),
            2 => union(&q3, &right)?,
            _ => vec![],
        }
    } else {
        match quadrant {
            1 => vec![],
            2 | 3 => q1.clone(),
            _ => union(&left, &q1)?,
        }
    };
    let result = union(
        &automatic,
        &proxy(mode, Condition::LessThan, value, sector)?,
    )?;
    if condition == Condition::GreaterThan {
        let whole = if ra {
            union(&union(&q1, &left)?, &q4)?
        } else {
            union(&union(&q2, &right)?, &q3)?
        };
        difference_windows(&whole, &result)
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod corrected_surface_gf_golden {
    use super::*;
    #[test]
    fn exact_corrected_surface_gf_conditions() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let d = kernel.state(301, 399, 0.0).unwrap().position;
        let length = norm(d);
        let direction = d.map(|x| x / length);
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_corrected_surfaces.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let expected: Vec<_> = if f[5].is_empty() {
                vec![]
            } else {
                f[5].split(';')
                    .map(|s| {
                        let (a, b) = s.split_once(':').unwrap();
                        Interval {
                            start: a.parse().unwrap(),
                            end: b.parse().unwrap(),
                        }
                    })
                    .collect()
            };
            let correction = Aberration::parse(f[2]).unwrap();
            let condition = Condition::parse(f[3]).unwrap();
            let found = if f[0] == "subpoint" {
                find_sub_point_coordinate_with_method(
                    &kernel,
                    &pck,
                    301,
                    399,
                    correction,
                    SubPointKind::Observer,
                    crate::surface::SubPointMethod::parse(f[1]).unwrap(),
                    CoordinateSystem::Planetographic,
                    1,
                    &[Interval {
                        start: 0.0,
                        end: 2592000.0,
                    }],
                    condition,
                    0.0,
                    FinderOptions::default(),
                )
            } else {
                find_surface_intercept_ray_coordinate(
                    &kernel,
                    &pck,
                    301,
                    399,
                    correction,
                    direction,
                    CoordinateSystem::Planetographic,
                    1,
                    &[Interval {
                        start: 0.0,
                        end: 300.0,
                    }],
                    condition,
                    0.0,
                    FinderOptions {
                        step: 10.0,
                        ..FinderOptions::default()
                    },
                )
            }
            .unwrap();
            if found != expected {
                errors.push(format!("{line}: actual {found:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count} cases:\n{}",
            errors.len(),
            errors
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn transform_surface_state(
    rotation: &crate::frame::Mat3,
    derivative: &crate::frame::Mat3,
    p: [f64; 3],
    v: [f64; 3],
    inverse: bool,
) -> ([f64; 3], [f64; 3]) {
    let r = if inverse {
        crate::frame::transpose(rotation)
    } else {
        *rotation
    };
    let d = if inverse {
        crate::frame::transpose(derivative)
    } else {
        *derivative
    };
    let position = rotate(&r, p);
    let velocity = std::array::from_fn(|i| {
        let mut sum = 0.0;
        for j in 0..3 {
            sum += d[i][j] * p[j];
        }
        for j in 0..3 {
            sum += r[i][j] * v[j];
        }
        sum
    });
    (position, velocity)
}
fn surface_velocity(
    p: [f64; 3],
    v: [f64; 3],
    direction: [f64; 3],
    direction_velocity: [f64; 3],
    point: [f64; 3],
    radii: [f64; 3],
) -> Result<Option<[f64; 3]>, GeoError> {
    let (u, du) = unit_state(direction, direction_velocity);
    let minimum = radii[0].min(radii[1]).min(radii[2]);
    let gradient = std::array::from_fn(|i| {
        let ratio = minimum / radii[i];
        point[i] * (ratio * ratio)
    });
    let gn = norm(gradient);
    let normal = gradient.map(|x| x / gn);
    let scaled: [f64; 3] = std::array::from_fn(|i| p[i] / radii[i]);
    if dot(scaled, scaled) == 1.0 {
        return Err(GeoError::BadSearch("ray vertex lies on ellipsoid"));
    }
    let udn = dot(u, normal);
    if udn == 0.0 {
        return Ok(None);
    }
    let vmx = subtract(p, point);
    let numerator = dot(vmx, normal);
    let second = du.map(|x| numerator * x);
    let dsnum = udn * dot(v, normal) - dot(vmx, normal) * dot(du, normal);
    let third = u.map(|x| dsnum * x);
    if norm(second).max(norm(third)).max(1.0) > f64::MAX / 10.0 * (udn * udn) {
        return Ok(None);
    }
    let r = 1.0 / udn;
    Ok(Some(std::array::from_fn(|i| {
        v[i] + (-r) * second[i] + (-(r * r)) * third[i]
    })))
}
fn subpoint_surface_state_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    method: crate::surface::SubPointMethod,
) -> Result<([f64; 3], [f64; 3]), GeoError> {
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let project = |p: [f64; 3], v: [f64; 3]| -> Result<([f64; 3], [f64; 3]), GeoError> {
        if method == crate::surface::SubPointMethod::NearPoint {
            let point = crate::surface::nearest_point_on_ellipsoid(p, radii)?.point;
            Ok((point, nearpoint_velocity(p, v, point, radii)?))
        } else {
            let direction = p.map(|x| -x);
            let point = crate::surface::ray_surface_intersection(radii, p, direction)?
                .ok_or(GeoError::BadSearch("sub-point not computable"))?;
            let velocity = surface_velocity(p, v, direction, v.map(|x| -x), point, radii)?
                .ok_or(GeoError::BadSearch("sub-point velocity not computable"))?;
            Ok((point, velocity))
        }
    };
    if abcorr == Aberration::None {
        let (p, v) =
            body_observer_state_in(kernel, pck, frames, target_frame, target, observer, et)?;
        return project(p, v);
    }
    let geometry = crate::surface::sub_observer_geometry_in(
        kernel,
        pck,
        frames,
        target_frame,
        target,
        observer,
        et,
        abcorr,
        method,
    )?;
    let observer_state = kernel.state_in_frame(observer, 0, et, frames, Some(pck), "J2000")?;
    let target_state =
        kernel.state_in_frame(target, 0, geometry.epoch, frames, Some(pck), "J2000")?;
    let orientation =
        frames.pxform_with_derivative("J2000", target_frame, geometry.epoch, Some(pck))?;
    let (rotation, derivative) = (orientation.rotation, orientation.rotation_dot);
    let (_, _, stellar, transmission) = abcorr.flags();
    let acceleration = if stellar {
        let before = kernel.state_in_frame(observer, 0, et - 1.0, frames, Some(pck), "J2000")?;
        let after = kernel.state_in_frame(observer, 0, et + 1.0, frames, Some(pck), "J2000")?;
        std::array::from_fn(|i| 0.5 * after.velocity[i] + (-0.5) * before.velocity[i])
    } else {
        [0.0; 3]
    };
    let (mut point, mut velocity) = (geometry.point, [0.0; 3]);
    let mut dlt = 0.0;
    for _ in 0..2 {
        let scale = if transmission { dlt + 1.0 } else { 1.0 - dlt };
        let center_p = subtract(target_state.position, observer_state.position);
        let center_v = subtract(
            target_state.velocity.map(|x| scale * x),
            observer_state.velocity,
        );
        let matrix_scale = if transmission {
            1.0 * dlt + 1.0
        } else {
            -1.0 * dlt + 1.0
        };
        let corrected_derivative = derivative.map(|row| row.map(|x| matrix_scale * x));
        let (ip, iv) =
            transform_surface_state(&rotation, &corrected_derivative, point, velocity, true);
        let relative = crate::spk::State {
            position: add(center_p, ip),
            velocity: add(center_v, iv),
        };
        let (offset_p, offset_v) = if stellar {
            SpkKernel::stellar_offsets(
                transmission,
                acceleration,
                observer_state.velocity,
                &relative,
            )
        } else {
            ([0.0; 3], [0.0; 3])
        };
        let corrected_center_p = if stellar {
            add(center_p, offset_p)
        } else {
            center_p
        };
        let corrected_center_v = if stellar {
            add(center_v, offset_v)
        } else {
            center_v
        };
        let (p, v) = transform_surface_state(
            &rotation,
            &corrected_derivative,
            corrected_center_p,
            corrected_center_v,
            false,
        );
        (point, velocity) = project(p.map(|x| -x), v.map(|x| -x))?;
        let (ip, iv) =
            transform_surface_state(&rotation, &corrected_derivative, point, velocity, true);
        let range_p = add(center_p, ip);
        let range_v = add(center_v, iv);
        let length = norm(range_p);
        let unit = range_p.map(|x| x / length);
        dlt = dot(range_v, unit) / crate::spk::CLIGHT;
    }
    Ok((point, velocity))
}

fn intercept_geometry_with_frame(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction: [f64; 3],
    direction_frame: &str,
) -> Result<crate::surface::SurfaceGeometry, GeoError> {
    let (light_time, _, _, transmission) = abcorr.flags();
    let mut epoch = et;
    if light_time && !frames.is_inertial(direction_frame)? {
        let center = frames.center(direction_frame)?;
        if center != observer {
            let (_, lt) = kernel.position_corrected_in_frame(
                center,
                observer,
                et,
                abcorr,
                frames,
                Some(pck),
                "J2000",
            )?;
            epoch = et + if transmission { lt } else { -lt };
        }
    }
    let rotation = frames.pxform_with_derivative(direction_frame, "J2000", epoch, Some(pck))?;
    let ray = rotate(&rotation.rotation, direction);
    crate::surface::surface_intercept_geometry_in(
        kernel,
        pck,
        frames,
        target_frame,
        target,
        observer,
        et,
        abcorr,
        ray,
    )
}

fn intercept_direction_state(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction: [f64; 3],
    direction_frame: &str,
) -> Result<([f64; 3], [f64; 3]), GeoError> {
    let (light_time, _, _, transmission) = abcorr.flags();
    let center = frames.center(direction_frame)?;
    let (epoch, factor) = if light_time && center != observer {
        let center_state = kernel.apparent_state_in(
            center,
            observer,
            et,
            abcorr,
            &crate::FrameSource::Named {
                frames,
                pck,
                frame_name: "J2000",
            },
        )?;
        let sign = if transmission { 1.0 } else { -1.0 };
        (
            et + sign * center_state.light_time,
            sign * center_state.light_time_rate + 1.0,
        )
    } else {
        (et, 1.0)
    };
    let rotation = frames.pxform_with_derivative(direction_frame, "J2000", epoch, Some(pck))?;
    let derivative = rotation.rotation_dot.map(|row| row.map(|x| factor * x));
    Ok(transform_surface_state(
        &rotation.rotation,
        &derivative,
        direction,
        [0.; 3],
        false,
    ))
}

#[cfg(test)]
fn ray_intercept_surface_state_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction: [f64; 3],
    direction_frame: &str,
    radii: [f64; 3],
) -> Result<Option<([f64; 3], [f64; 3])>, GeoError> {
    ray_intercept_surface_state_with_frame(
        kernel,
        pck,
        frames,
        &format!("IAU_{target}"),
        target,
        observer,
        et,
        abcorr,
        direction,
        direction_frame,
        radii,
    )
}
fn ray_intercept_surface_state_with_frame(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction: [f64; 3],
    direction_frame: &str,
    radii: [f64; 3],
) -> Result<Option<([f64; 3], [f64; 3])>, GeoError> {
    let (direction_p, direction_v) = intercept_direction_state(
        kernel,
        pck,
        frames,
        observer,
        et,
        abcorr,
        direction,
        direction_frame,
    )?;
    if abcorr == Aberration::None {
        return ray_intercept_state_none(
            kernel,
            pck,
            frames,
            target_frame,
            target,
            observer,
            et,
            direction_p,
            direction_v,
            radii,
        );
    }
    let geometry = match intercept_geometry_with_frame(
        kernel,
        pck,
        frames,
        target_frame,
        target,
        observer,
        et,
        abcorr,
        direction,
        direction_frame,
    ) {
        Ok(g) => g,
        Err(GeoError::BadSearch("ray does not intersect the target")) => return Ok(None),
        Err(e) => return Err(e),
    };
    let observer_state = kernel.state_in_frame(observer, 0, et, frames, Some(pck), "J2000")?;
    let target_state =
        kernel.state_in_frame(target, 0, geometry.epoch, frames, Some(pck), "J2000")?;
    let orientation =
        frames.pxform_with_derivative("J2000", target_frame, geometry.epoch, Some(pck))?;
    let (rotation, derivative) = (orientation.rotation, orientation.rotation_dot);
    let (_, _, stellar, transmission) = abcorr.flags();
    let acceleration = if stellar {
        let before = kernel.state_in_frame(observer, 0, et - 1.0, frames, Some(pck), "J2000")?;
        let after = kernel.state_in_frame(observer, 0, et + 1.0, frames, Some(pck), "J2000")?;
        std::array::from_fn(|i| 0.5 * after.velocity[i] + (-0.5) * before.velocity[i])
    } else {
        [0.0; 3]
    };
    let (mut point, mut velocity) = (geometry.point, [0.0; 3]);
    let mut dlt = 0.0;
    for _ in 0..3 {
        let scale = if transmission { dlt + 1.0 } else { 1.0 - dlt };
        let center_p = subtract(target_state.position, observer_state.position);
        let center_v = subtract(
            target_state.velocity.map(|x| scale * x),
            observer_state.velocity,
        );
        let matrix_scale = if transmission {
            1.0 * dlt + 1.0
        } else {
            -1.0 * dlt + 1.0
        };
        let corrected_derivative = derivative.map(|row| row.map(|x| matrix_scale * x));
        let (ip, iv) =
            transform_surface_state(&rotation, &corrected_derivative, point, velocity, true);
        let relative = crate::spk::State {
            position: add(center_p, ip),
            velocity: add(center_v, iv),
        };
        let (offset_p, offset_v) = if stellar {
            SpkKernel::stellar_offsets(
                transmission,
                acceleration,
                observer_state.velocity,
                &relative,
            )
        } else {
            ([0.0; 3], [0.0; 3])
        };
        let corrected_center_p = if stellar {
            add(center_p, offset_p)
        } else {
            center_p
        };
        let corrected_center_v = if stellar {
            add(center_v, offset_v)
        } else {
            center_v
        };
        let (p, v) = transform_surface_state(
            &rotation,
            &corrected_derivative,
            corrected_center_p,
            corrected_center_v,
            false,
        );
        let (dp, dv) = transform_surface_state(
            &rotation,
            &corrected_derivative,
            direction_p,
            direction_v,
            false,
        );
        let p = p.map(|x| -x);
        let v = v.map(|x| -x);
        point = match crate::surface::ray_surface_intersection(radii, p, dp)? {
            Some(point) => point,
            None => return Ok(None),
        };
        velocity = match surface_velocity(p, v, dp, dv, point, radii)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let (ip, iv) =
            transform_surface_state(&rotation, &corrected_derivative, point, velocity, true);
        let range_p = add(center_p, ip);
        let range_v = add(center_v, iv);
        let length = norm(range_p);
        let unit = range_p.map(|x| x / length);
        dlt = dot(range_v, unit) / crate::spk::CLIGHT;
    }
    Ok(Some((point, velocity)))
}

#[cfg(test)]
mod intercept_dref_states {
    use super::*;
    #[test]
    fn exact_cspice_named_intercept_direction_states() {
        let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let frames =
            Frames::from_text(include_str!("../../tests/golden/gf_intercept_dref.tf")).unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_intercept_dref_states_precision.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let et = f[0].parse().unwrap();
            let actual = ray_intercept_surface_state_in(
                &kernel,
                &pck,
                &frames,
                f[13].parse().unwrap(),
                f[14].parse().unwrap(),
                et,
                Aberration::parse(f[1]).unwrap(),
                [
                    f[3].parse().unwrap(),
                    f[4].parse().unwrap(),
                    f[5].parse().unwrap(),
                ],
                f[2],
                pck.radii(f[13].parse().unwrap()).unwrap(),
            )
            .unwrap();
            let expected: Vec<f64> = f[7..13].iter().map(|x| x.parse().unwrap()).collect();
            if actual.is_some() != (f[6] == "1") {
                errors.push(format!("FOUND {line}:actual{actual:?}"));
            } else if let Some((p, v)) = actual {
                let a: Vec<_> = p.into_iter().chain(v).collect();
                if a.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                    != expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                {
                    errors.push(format!("STATE {line}:actual{a:?}"));
                }
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(15)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
        eprintln!("{count} exact ZZGFSSIN direction-frame states");
    }
}

#[cfg(test)]
mod intercept_target_states {
    use super::*;
    #[test]
    fn exact_cspice_intercept_named_target_states() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let native = Pck::open_all(&[
            "target/kernels/pck00011.tpc",
            "target/kernels/moon_pa_de440_200625.bpc",
        ])
        .unwrap();
        let mut triaxial = native.clone();
        triaxial
            .furnsh("KPL/PCK\n\\begindata\nBODY301_RADII=(1738.1 1736.7 1734.2)\n\\begintext")
            .unwrap();
        let mut frames = Frames::new();
        for path in [
            "target/kernels/moon_de440_250416.tf",
            "target/kernels/moon_assoc_me.tf",
            "tests/golden/surface_target_frames.tf",
        ] {
            frames
                .furnsh(&std::fs::read_to_string(path).unwrap())
                .unwrap();
        }
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_intercept_target_states_precision.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let pck = if f[0] == "native" { &native } else { &triaxial };
            let actual = ray_intercept_surface_state_with_frame(
                &kernel,
                pck,
                &frames,
                f[1],
                301,
                399,
                f[2].parse().unwrap(),
                Aberration::parse(f[3]).unwrap(),
                [
                    f[5].parse().unwrap(),
                    f[6].parse().unwrap(),
                    f[7].parse().unwrap(),
                ],
                f[4],
                pck.radii(301).unwrap(),
            )
            .unwrap();
            let expected: Vec<_> = f[9..]
                .iter()
                .map(|x| x.parse::<f64>().unwrap().to_bits())
                .collect();
            if actual.is_some() != (f[8] == "1") {
                errors.push(format!("FOUND {line}:actual{actual:?}"));
            } else if let Some((p, v)) = actual {
                if p.into_iter()
                    .chain(v)
                    .map(|x| x.to_bits())
                    .collect::<Vec<_>>()
                    != expected
                {
                    errors.push(format!("STATE {line}:actual{p:?}{v:?}"));
                }
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
        eprintln!("{count} exact ZZGFSSIN named-target state cases");
    }
}

#[cfg(test)]
mod subpoint_target_states {
    use super::*;
    #[test]
    fn exact_cspice_subpoint_named_target_states() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let native = Pck::open_all(&[
            "target/kernels/pck00011.tpc",
            "target/kernels/moon_pa_de440_200625.bpc",
        ])
        .unwrap();
        let mut triaxial = native.clone();
        triaxial
            .furnsh("KPL/PCK\n\\begindata\nBODY301_RADII=(1738.1 1736.7 1734.2)\n\\begintext")
            .unwrap();
        let mut frames = Frames::new();
        for path in [
            "target/kernels/moon_de440_250416.tf",
            "target/kernels/moon_assoc_me.tf",
            "tests/golden/surface_target_frames.tf",
        ] {
            frames
                .furnsh(&std::fs::read_to_string(path).unwrap())
                .unwrap();
        }
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_subpoint_target_states_precision.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let pck = if f[0] == "native" { &native } else { &triaxial };
            let (p, v) = subpoint_surface_state_in(
                &kernel,
                pck,
                &frames,
                f[1],
                301,
                399,
                f[4].parse().unwrap(),
                Aberration::parse(f[2]).unwrap(),
                crate::SubPointMethod::parse(f[3]).unwrap(),
            )
            .unwrap();
            let expected: Vec<_> = f[5..]
                .iter()
                .map(|x| x.parse::<f64>().unwrap().to_bits())
                .collect();
            if p.into_iter()
                .chain(v)
                .map(|x| x.to_bits())
                .collect::<Vec<_>>()
                != expected
            {
                errors.push(format!("{line}:actual{p:?}{v:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
        eprintln!("{count} exact ZZGFSSOB named-target state cases");
    }
}
#[cfg(test)]
mod illumination_gf_golden {
    use super::*;
    #[test]
    fn exact_cspice_illumination_gf() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_illumination.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let expected: Vec<_> = if f[7].is_empty() {
                vec![]
            } else {
                f[7].split(';')
                    .map(|s| {
                        let (a, b) = s.split_once(':').unwrap();
                        Interval {
                            start: a.parse().unwrap(),
                            end: b.parse().unwrap(),
                        }
                    })
                    .collect()
            };
            let angle = match f[0] {
                "PHASE" => IlluminationAngle::Phase,
                "INCIDENCE" => IlluminationAngle::Incidence,
                _ => IlluminationAngle::Emission,
            };
            let result = find_illumination_angle(
                &kernel,
                &pck,
                301,
                399,
                Aberration::parse(f[1]).unwrap(),
                [
                    f[4].parse().unwrap(),
                    f[5].parse().unwrap(),
                    f[6].parse().unwrap(),
                ],
                angle,
                &[Interval {
                    start: 0.0,
                    end: 2592000.0,
                }],
                Condition::parse(f[2]).unwrap(),
                f[3].parse().unwrap(),
                FinderOptions::default(),
            )
            .unwrap();
            if result != expected {
                errors.push(format!("{line}:actual {result:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count} cases:\n{}",
            errors.len(),
            errors
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn illumination_decreasing_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    illuminator: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    point: [f64; 3],
    angle: IlluminationAngle,
) -> Result<bool, GeoError> {
    use crate::location::{state_between, Endpoint, PointLocation};
    let location = PointLocation::fixed(target, target_frame.to_string(), point);
    let seen = state_between(
        kernel,
        frames,
        pck,
        Endpoint::Point(&location),
        Endpoint::Object(observer),
        et,
        abcorr,
    )?;
    let epoch = crate::state_rep::time_at_target(et, seen.light_time, abcorr);
    let mut source = state_between(
        kernel,
        frames,
        pck,
        Endpoint::Object(illuminator),
        Endpoint::Point(&location),
        epoch,
        abcorr,
    )?
    .state;
    let orientation = frames.pxform_with_derivative("J2000", target_frame, epoch, Some(pck))?;
    let (rotation, derivative) = (orientation.rotation, orientation.rotation_dot);
    let (uselt, _, _, transmission) = abcorr.flags();
    if transmission {
        return Err(GeoError::BadSearch(
            "illumination supports reception corrections only",
        ));
    }
    let length = norm(seen.state.position);
    let unit = seen.state.position.map(|x| x / length);
    let dlt = if uselt {
        dot(seen.state.velocity, unit) / crate::spk::CLIGHT
    } else {
        0.0
    };
    let corrected_derivative = derivative.map(|row| row.map(|x| (1.0 - dlt) * x));
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let minimum = radii[0].min(radii[1]).min(radii[2]);
    let gradient = std::array::from_fn(|i| {
        let ratio = minimum / radii[i];
        point[i] * (ratio * ratio)
    });
    let n = norm(gradient);
    let normal = gradient.map(|x| x / n);
    // ZZGFILIN stores SURFNM's unit normal; ZZILUSTA normalizes again.
    let n = norm(normal);
    let normal = normal.map(|x| x / n);
    let (np, nv) =
        transform_surface_state(&rotation, &corrected_derivative, normal, [0.0; 3], true);
    if uselt {
        source.velocity = source.velocity.map(|x| (1.0 - dlt) * x);
    }
    let op = seen.state.position.map(|x| -x);
    let ov = seen.state.velocity.map(|x| -x);
    let (p1, v1, p2, v2) = match angle {
        IlluminationAngle::Phase => (op, ov, source.position, source.velocity),
        IlluminationAngle::Incidence => (np, nv, source.position, source.velocity),
        IlluminationAngle::Emission => (np, nv, op, ov),
    };
    let (u1, d1) = unit_state(p1, v1);
    let (u2, d2) = unit_state(p2, v2);
    if cross(u1, u2) == [0.0; 3] {
        return Ok(false);
    }
    Ok(dot(u1, d2) + dot(d1, u2) > 0.0)
}

#[cfg(test)]
mod center_intercept_gf_regressions {
    use super::*;
    #[test]
    fn center_ray_matches_subpoint_intercept_search() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let result = find_surface_intercept_coordinate(
            &kernel,
            &pck,
            301,
            399,
            Aberration::None,
            CoordinateSystem::Planetographic,
            1,
            &[Interval {
                start: 0.0,
                end: 2592000.0,
            }],
            Condition::LessThan,
            0.0,
            FinderOptions::default(),
        )
        .unwrap();
        assert_eq!(
            result,
            vec![
                Interval {
                    start: 0.0,
                    end: 586774.7589294317
                },
                Interval {
                    start: 1719767.709411936,
                    end: 2592000.0
                }
            ]
        );
    }
}

#[cfg(test)]
mod separation_gf_golden {
    use super::*;
    #[test]
    fn exact_cspice_separation_gf() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_separation.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let expected: Vec<_> = if f[5].is_empty() {
                vec![]
            } else {
                f[5].split(';')
                    .map(|s| {
                        let (a, b) = s.split_once(':').unwrap();
                        Interval {
                            start: a.parse().unwrap(),
                            end: b.parse().unwrap(),
                        }
                    })
                    .collect()
            };
            let actual = find_angular_separation(
                &kernel,
                &pck,
                399,
                301,
                10,
                Aberration::parse(f[2]).unwrap(),
                AngularShape::parse(f[0]).unwrap(),
                AngularShape::parse(f[1]).unwrap(),
                &[Interval {
                    start: 0.0,
                    end: 2592000.0,
                }],
                Condition::parse(f[3]).unwrap(),
                f[4].parse().unwrap(),
                FinderOptions::default(),
            )
            .unwrap();
            if actual != expected {
                errors.push(format!("{line}: actual {actual:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count} cases:\n{}",
            errors.len(),
            errors
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Source-compatible CSPICE GFSEP scalar-condition search for POINT/SPHERE bodies.
#[allow(clippy::too_many_arguments)]
pub fn find_angular_separation(
    kernel: &SpkKernel,
    pck: &Pck,
    observer: i32,
    target1: i32,
    target2: i32,
    abcorr: Aberration,
    shape1: AngularShape,
    shape2: AngularShape,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    if observer == target1 || observer == target2 || target1 == target2 {
        return Err(GeoError::BadSearch("SPICE(BODIESNOTDISTINCT)"));
    }
    let radius1 = shape1.radius(pck, target1)?;
    let radius2 = shape2.radius(pck, target2)?;
    search_scalar_with_decreasing(
        |et| {
            angular_separation_shaped(
                kernel, pck, observer, target1, target2, et, abcorr, shape1, shape2,
            )
        },
        |et| {
            separation_decreasing(
                kernel, observer, target1, target2, et, abcorr, radius1, radius2,
            )
        },
        windows,
        condition,
        reference,
        options,
    )
}

#[cfg(test)]
mod unsupported_solar_search {
    use super::*;
    #[test]
    fn rejects_solar_intervals_without_a_native_cspice_gf_solver() {
        let kernel = SpkKernel::open("tests/golden/gf_fixed.bsp").unwrap();
        let pck = Pck::from_text("\\begindata\nBODY299_RADII=(6051.8 6051.8 6051.8)\n").unwrap();
        let result = find_sub_point_coordinate(
            &kernel,
            &pck,
            299,
            0,
            Aberration::None,
            SubPointKind::Solar,
            CoordinateSystem::Rectangular,
            0,
            &[Interval {
                start: -1.0,
                end: 1.0,
            }],
            Condition::GreaterThan,
            0.0,
            FinderOptions::default(),
        );
        assert!(
            matches!(result, Err(GeoError::Unsupported(_))),
            "{result:?}"
        );
    }
}

#[cfg(test)]
mod separation_scalar_golden {
    use super::*;
    #[test]
    fn exact_cspice_trgsep_scalars() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_separation_values.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let expected: f64 = f[4].parse().unwrap();
            let actual = angular_separation_shaped(
                &kernel,
                &pck,
                399,
                301,
                10,
                f[3].parse().unwrap(),
                Aberration::parse(f[2]).unwrap(),
                AngularShape::parse(f[0]).unwrap(),
                AngularShape::parse(f[1]).unwrap(),
            )
            .unwrap();
            if actual != expected {
                errors.push(format!("{line}: actual{actual:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[cfg(test)]
mod angular_size_scalar_golden {
    use super::*;
    #[test]
    fn exact_cspice_angular_size_primitives() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_angular_size.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let expected: f64 = f[5].parse().unwrap();
            let method = AngularSizeMethod::parse(f[4]).unwrap();
            let actual = if method == AngularSizeMethod::LimbExtents {
                angular_size_center_ray_extents(
                    &kernel,
                    &pck,
                    f[1].parse().unwrap(),
                    f[0].parse().unwrap(),
                    f[3].parse().unwrap(),
                    Aberration::parse(f[2]).unwrap(),
                )
                .unwrap()[0]
            } else {
                angular_size_with_method(
                    &kernel,
                    &pck,
                    f[1].parse().unwrap(),
                    f[0].parse().unwrap(),
                    f[3].parse().unwrap(),
                    Aberration::parse(f[2]).unwrap(),
                    method,
                )
                .unwrap()
            };
            if actual != expected {
                errors.push(format!("{line}: actual{actual:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[cfg(test)]
mod body_frame_gf_golden {
    use super::*;
    #[test]
    fn exact_cspice_body_frame_position_gf() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let frames = Frames::new();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_body_frame_positions.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let system = CoordinateSystem::parse(f[3]).unwrap();
            let index = system.coordinate_index(f[4]).unwrap();
            let expected: Vec<_> = if f[8].is_empty() {
                vec![]
            } else {
                f[8].split(';')
                    .map(|s| {
                        let (a, b) = s.split_once(':').unwrap();
                        Interval {
                            start: a.parse().unwrap(),
                            end: b.parse().unwrap(),
                        }
                    })
                    .collect()
            };
            let actual = find_position_coordinate_with_radii(
                &kernel,
                &pck,
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                Aberration::parse(f[5]).unwrap(),
                &frames,
                f[2],
                system,
                index,
                &[Interval {
                    start: 0.0,
                    end: 2592000.0,
                }],
                Condition::parse(f[6]).unwrap(),
                f[7].parse().unwrap(),
                FinderOptions::default(),
            );
            match actual {
                Ok(a) if a == expected => {}
                other => errors.push(format!("{line}:actual{other:?}")),
            };
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[cfg(test)]
mod body_frame_scalar_regression {
    use super::*;
    #[test]
    fn exact_earth_planetographic_altitude_at_gf_root() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let frames = Frames::new();
        let p = crate::spkapo::corrected_position_in_frame(
            &kernel,
            301,
            399,
            876122.4604546295,
            Aberration::LightTime,
            &frames,
            "IAU_EARTH",
            Some(&pck),
        )
        .unwrap()
        .0;
        assert_eq!(
            p,
            [383468.9806588502, 48581.29463118005, -61382.67977790787]
        );
        let coord = coordinate_value_with_pck(
            p,
            CoordinateSystem::Planetographic,
            2,
            pck.radii(399),
            399,
            &pck,
        )
        .unwrap();
        assert_eq!(coord, 385000.00000001787);
    }
}

#[cfg(test)]
mod position_trace_regression {
    use super::*;
    #[test]
    fn exact_gf_position_callback_trace() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let frames = Frames::new();
        let mut errors = Vec::new();
        for line in include_str!("../../tests/golden/gf_position_trace.csv").lines() {
            let f: Vec<_> = line.split(',').collect();
            let t = f[1].parse().unwrap();
            if f[0] == "D" {
                let (p, v) = transformed_state(
                    &kernel,
                    &frames,
                    301,
                    399,
                    t,
                    Aberration::LightTime,
                    "IAU_EARTH",
                    Some(&pck),
                )
                .unwrap();
                let state = [p[0], p[1], p[2], v[0], v[1], v[2]];
                let expected: Vec<f64> = f[3..].iter().map(|s| s.parse().unwrap()).collect();
                if state != expected.as_slice() {
                    errors.push(format!("state {t}: {state:?}!={expected:?}"));
                }
                let dec = coordinate_decreasing(
                    p,
                    v,
                    CoordinateSystem::Planetographic,
                    2,
                    pck.radii(399),
                    true,
                )
                .unwrap();
                if dec != (f[2] == "1") {
                    errors.push(format!("sign {t}: {dec}!={}", f[2]));
                }
            } else {
                let p = crate::spkapo::corrected_position_in_frame(
                    &kernel,
                    301,
                    399,
                    t,
                    Aberration::LightTime,
                    &frames,
                    "IAU_EARTH",
                    Some(&pck),
                )
                .unwrap()
                .0;
                let alt = coordinate_value_with_pck(
                    p,
                    CoordinateSystem::Planetographic,
                    2,
                    pck.radii(399),
                    399,
                    &pck,
                )
                .unwrap();
                let expected: f64 = f[2].parse().unwrap();
                if alt != expected {
                    errors.push(format!("alt {t}: {alt:?}!={expected:?}"));
                }
            }
        }
        assert!(
            errors.is_empty(),
            "{} mismatches:\n{}",
            errors.len(),
            errors
                .iter()
                .take(15)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[cfg(test)]
mod angular_extents_golden {
    use super::*;
    #[test]
    fn exact_cspice_angular_extents_pair() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_angular_extents.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let expected = [f[4].parse::<f64>().unwrap(), f[5].parse().unwrap()];
            let actual = angular_size_center_ray_extents(
                &kernel,
                &pck,
                f[1].parse().unwrap(),
                f[0].parse().unwrap(),
                f[3].parse().unwrap(),
                Aberration::parse(f[2]).unwrap(),
            )
            .unwrap();
            if actual != expected {
                errors.push(format!("{line}:actual{actual:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// WGC-compatible full angular widths of the limb ellipse's minor and major axes.
///
/// Observed WGC display values agree with twice the arctangent of each limb
/// semi-axis length divided by the observer-to-limb-center distance. WGC's
/// unrounded internal results are not available for independent comparison.
pub fn angular_size_extents(
    kernel: &SpkKernel,
    pck: &Pck,
    observer: i32,
    target: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<[f64; 2], GeoError> {
    let (center, light_time) =
        crate::spkapo::corrected_position(kernel, target, observer, et, abcorr)?;
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let epoch = crate::state_rep::time_at_target(et, light_time, abcorr);
    let rotation = pck
        .body_rotation(target, epoch)
        .ok_or(GeoError::MissingPckData(target))?;
    crate::occult_ellipsoid::limb_axis_extents(center, radii, &rotation)
}

/// WGC limb-axis angular widths in an explicit target-centered body-fixed frame.
///
/// The frame is evaluated at the aberration-corrected target epoch. Output
/// consists of the minor and major full angular widths, in radians.
#[allow(clippy::too_many_arguments)]
pub fn angular_size_extents_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    observer: i32,
    target: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<[f64; 2], GeoError> {
    crate::surface::validate_target_frame(frames, target_frame, target)?;
    let (center, light_time) =
        crate::spkapo::corrected_position(kernel, target, observer, et, abcorr)?;
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let epoch = crate::state_rep::time_at_target(et, light_time, abcorr);
    let rotation = frames.pxform_in("J2000", target_frame, epoch, Some(pck))?;
    crate::occult_ellipsoid::limb_axis_extents(center, radii, &rotation)
}

/// Full angular extrema about the observer-to-target-center ray using ZZASRYEL.
pub fn angular_size_center_ray_extents(
    kernel: &SpkKernel,
    pck: &Pck,
    observer: i32,
    target: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<[f64; 2], GeoError> {
    let (center, light_time) =
        crate::spkapo::corrected_position(kernel, target, observer, et, abcorr)?;
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let epoch = crate::state_rep::time_at_target(et, light_time, abcorr);
    let rotation = pck
        .body_rotation(target, epoch)
        .ok_or(GeoError::MissingPckData(target))?;
    Ok(crate::occult_ellipsoid::limb_angular_extents(center, radii, &rotation)?.map(|x| 2.0 * x))
}

#[cfg(test)]
mod distance_phase_golden {
    use super::*;
    #[test]
    fn exact_native_phase_scalars() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let mut errors = Vec::new();
        for line in include_str!("../../tests/golden/gf_phase_scalars.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let expected: f64 = f[3].parse().unwrap();
            let actual = phase_angle(
                &kernel,
                f[0].parse().unwrap(),
                10,
                399,
                f[2].parse().unwrap(),
                Aberration::parse(f[1]).unwrap(),
            )
            .unwrap();
            if actual != expected {
                errors.push(format!("{line}:actual{actual:?}"));
            }
        }
        assert!(
            errors.is_empty(),
            "{} mismatches:\n{}",
            errors.len(),
            errors
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    #[test]
    fn exact_native_distance_phase_gf() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_distance_phase.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let target = f[1].parse().unwrap();
            let abcorr = Aberration::parse(f[2]).unwrap();
            let condition = Condition::parse(f[3]).unwrap();
            let reference = f[4].parse().unwrap();
            let expected: Vec<_> = if f[5].is_empty() {
                vec![]
            } else {
                f[5].split(';')
                    .map(|s| {
                        let (a, b) = s.split_once(':').unwrap();
                        Interval {
                            start: a.parse().unwrap(),
                            end: b.parse().unwrap(),
                        }
                    })
                    .collect()
            };
            let actual = if f[0] == "distance" {
                find_distance(
                    &kernel,
                    target,
                    399,
                    abcorr,
                    &[Interval {
                        start: 0.0,
                        end: 2592000.0,
                    }],
                    condition,
                    reference,
                    FinderOptions::default(),
                )
            } else {
                find_phase_angle(
                    &kernel,
                    target,
                    10,
                    399,
                    abcorr,
                    &[Interval {
                        start: 0.0,
                        end: 2592000.0,
                    }],
                    condition,
                    reference,
                    FinderOptions::default(),
                )
            }
            .unwrap();
            if actual != expected {
                errors.push(format!("{line}:actual{actual:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    #[test]
    fn phase_rejects_transmission_and_coincident_objects() {
        let kernel = SpkKernel::open("tests/golden/gf_fixed.bsp").unwrap();
        for correction in [
            Aberration::TransmissionLightTime,
            Aberration::TransmissionLightTimeStellar,
            Aberration::TransmissionConvergedLightTime,
            Aberration::TransmissionConvergedLightTimeStellar,
        ] {
            assert!(phase_angle(&kernel, 299, 10, 0, 0.0, correction).is_err());
        }
        for (t, i, o) in [(299, 299, 0), (299, 10, 299), (299, 0, 0)] {
            assert!(matches!(
                phase_angle(&kernel, t, i, o, 0.0, Aberration::None),
                Err(GeoError::BadSearch(_))
            ));
        }
    }
}

#[cfg(test)]
mod custom_illumination_golden {
    use super::*;
    #[test]
    fn exact_custom_illuminator_gf() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_custom_illumination.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let illuminator = f[0].parse().unwrap();
            let angle = match f[1] {
                "PHASE" => IlluminationAngle::Phase,
                "INCIDENCE" => IlluminationAngle::Incidence,
                _ => IlluminationAngle::Emission,
            };
            let expected: Vec<_> = if f[8].is_empty() {
                vec![]
            } else {
                f[8].split(';')
                    .map(|s| {
                        let (a, b) = s.split_once(':').unwrap();
                        Interval {
                            start: a.parse().unwrap(),
                            end: b.parse().unwrap(),
                        }
                    })
                    .collect()
            };
            let actual = find_illumination_angle_with_illuminator(
                &kernel,
                &pck,
                301,
                illuminator,
                399,
                Aberration::parse(f[2]).unwrap(),
                [
                    f[5].parse().unwrap(),
                    f[6].parse().unwrap(),
                    f[7].parse().unwrap(),
                ],
                angle,
                &[Interval {
                    start: 0.0,
                    end: 2592000.0,
                }],
                Condition::parse(f[3]).unwrap(),
                f[4].parse().unwrap(),
                FinderOptions::default(),
            )
            .unwrap();
            if actual != expected {
                errors.push(format!("{line}:actual{actual:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Illumination-angle search with an explicitly selected light source.
#[allow(clippy::too_many_arguments)]
pub fn find_illumination_angle_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    illuminator: i32,
    observer: i32,
    abcorr: Aberration,
    point: [f64; 3],
    angle: IlluminationAngle,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    if frames.center(target_frame)? != target {
        return Err(GeoError::BadSearch(
            "target frame is not centered on target",
        ));
    }
    if abcorr.flags().3 {
        return Err(GeoError::BadSearch(
            "illumination supports reception corrections only",
        ));
    }
    search_scalar_with_decreasing(
        |et| {
            let angles = crate::surface::illumination_geometry_in(
                kernel,
                pck,
                frames,
                target_frame,
                target,
                illuminator,
                observer,
                et,
                abcorr,
                point,
            )?
            .angles;
            Ok(match angle {
                IlluminationAngle::Incidence => angles.solar,
                IlluminationAngle::Emission => angles.emission,
                IlluminationAngle::Phase => angles.phase,
            })
        },
        |et| {
            illumination_decreasing_in(
                kernel,
                pck,
                frames,
                target_frame,
                target,
                illuminator,
                observer,
                et,
                abcorr,
                point,
                angle,
            )
        },
        windows,
        condition,
        reference,
        options,
    )
}

/// Illumination-angle search with an explicitly selected light source.
#[allow(clippy::too_many_arguments)]
pub fn find_illumination_angle_with_illuminator(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    illuminator: i32,
    observer: i32,
    abcorr: Aberration,
    point: [f64; 3],
    angle: IlluminationAngle,
    windows: &[Interval],
    condition: Condition,
    reference: f64,
    options: FinderOptions,
) -> Result<Vec<Interval>, GeoError> {
    find_illumination_angle_in(
        kernel,
        pck,
        &Frames::new(),
        &format!("IAU_{target}"),
        target,
        illuminator,
        observer,
        abcorr,
        point,
        angle,
        windows,
        condition,
        reference,
        options,
    )
}

fn pck_coordinate_sense(pck: &Pck, system: CoordinateSystem, body: i32) -> Result<bool, GeoError> {
    if matches!(
        system,
        CoordinateSystem::Geodetic | CoordinateSystem::Planetographic
    ) {
        let radii = pck.radii(body).ok_or(GeoError::MissingPckData(body))?;
        if radii[0] != radii[1] {
            return Err(GeoError::Unsupported(
                "GF geodetic and planetographic coordinates require equal equatorial radii".into(),
            ));
        }
    }
    if system == CoordinateSystem::Planetographic {
        pck.planetographic_east_positive(body)
            .ok_or(GeoError::MissingPckData(body))
    } else {
        Ok(false)
    }
}

fn separation_decreasing(
    kernel: &SpkKernel,
    observer: i32,
    target1: i32,
    target2: i32,
    et: f64,
    abcorr: Aberration,
    radius1: f64,
    radius2: f64,
) -> Result<bool, GeoError> {
    let first = kernel.state_corrected(target1, observer, et, abcorr)?.state;
    let second = kernel.state_corrected(target2, observer, et, abcorr)?.state;
    let (u1, d1) = unit_state(first.position, first.velocity);
    let (u2, d2) = unit_state(second.position, second.velocity);
    let perpendicular = cross(u1, u2);
    let dtheta = if perpendicular == [0.0; 3] {
        0.0
    } else {
        -(dot(u1, d2) + dot(d1, u2)) / norm(perpendicular)
    };
    let half_angle_rate = |s: crate::spk::State, radius: f64| -> Result<f64, GeoError> {
        if radius == 0.0 {
            return Ok(0.0);
        }
        let range = norm(s.position);
        let base = range * range - radius * radius;
        if base <= 0.0 {
            return Err(GeoError::BadSearch(
                "body radius equals or exceeds observer range",
            ));
        }
        let unit = s.position.map(|x| x / range);
        let range_rate = dot(unit, s.velocity);
        Ok(-(range_rate * radius) / (base.sqrt() * range))
    };
    Ok(dtheta - half_angle_rate(first, radius1)? - half_angle_rate(second, radius2)? < 0.0)
}
