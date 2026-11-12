// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::too_many_arguments)]
//! Surface geometry on a triaxial ellipsoid.
//!
//! Provides the near-side sub-observer and sub-solar points used by the
//! WebGeocalc sub-point family. The ellipsoid axes come from a text PCK
//! and the body-fixed orientation from the IAU model in
//! [`Pck::body_rotation`].
//!
//! The nearest point is found by solving the standard Lagrange equation
//!
//! ```text
//! f(lambda) = sum (a_i^2 p_i^2) / (a_i^2 + lambda)^2 - 1 = 0
//! ```
//!
//! using CSPICE NEARPT scaling and normal refinement, including interior
//! points and their negative signed altitude.

use crate::error::GeoError;
use crate::frame::{Frames, Mat3};
use crate::pck::Pck;
use crate::spk::{Aberration, FrameSource, SpkKernel};
use crate::state_rep::time_at_target;

/// A point on a body's surface and the observer's altitude above it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePoint {
    /// Surface point in the body-fixed frame, kilometres.
    pub point: [f64; 3],
    /// Distance from the observer to the surface point, kilometres.
    pub altitude: f64,
}

/// How a sub-observer or sub-solar point is defined, matching the
/// WebGeocalc `subPointType` values and the CSPICE `subpnt` method strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubPointMethod {
    /// Nearest point on the reference ellipsoid to the observer
    /// (`'NEAR POINT/ELLIPSOID'`).
    NearPoint,
    /// Intercept of the line through the observer and the target centre
    /// (`'INTERCEPT/ELLIPSOID'`).
    Intercept,
}

impl SubPointMethod {
    /// Parses a WebGeocalc `subPointType` or a CSPICE method string.
    pub fn parse(text: &str) -> Option<Self> {
        let normalized: String = text
            .chars()
            .map(|character| match character {
                ':' | '/' | '-' | '_' => ' ',
                other => other.to_ascii_uppercase(),
            })
            .collect();
        let words: Vec<&str> = normalized.split_whitespace().collect();
        match words.as_slice() {
            ["NEAR", "POINT", "ELLIPSOID"] | ["NADIR", "ELLIPSOID"] => Some(Self::NearPoint),
            ["INTERCEPT", "ELLIPSOID"] => Some(Self::Intercept),
            _ => None,
        }
    }
}

/// Near-side sub-observer point of `target` as seen from `observer`.
///
/// The result is expressed in the IAU body-fixed frame of `target`,
/// matching CSPICE `subpnt` with method `NEAR POINT: ELLIPSOID`.
pub fn sub_observer_point(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<SurfacePoint, GeoError> {
    sub_observer_point_with_method(
        kernel,
        pck,
        target,
        observer,
        et,
        abcorr,
        SubPointMethod::NearPoint,
    )
}

/// Sub-observer point of `target` with an explicit definition.
///
/// [`SubPointMethod::Intercept`] is the intercept of the line through the
/// observer and the target centre. The two definitions coincide for a sphere; ellipsoids generally produce
/// distinct near points and center-line intercepts.
pub fn sub_observer_point_with_method(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    method: SubPointMethod,
) -> Result<SurfacePoint, GeoError> {
    let geometry = sub_observer_geometry(kernel, pck, target, observer, et, abcorr, method)?;
    Ok(SurfacePoint {
        point: geometry.point,
        altitude: geometry.altitude,
    })
}

/// Intercept of the line through `observer` (body-fixed) and the body
/// centre, as CSPICE `surfpt` with `u = -observer`.
///
/// Along that line every point is `lambda * observer` for `0 < lambda < 1`,
/// so the ellipsoid equation reduces to `lambda^2 * Q(observer) = 1`.
fn intercept_through_center(observer: [f64; 3], radii: [f64; 3]) -> Result<SurfacePoint, GeoError> {
    if radii.iter().any(|axis| *axis <= 0.0) {
        return Err(GeoError::BadSearch("ellipsoid axes must be positive"));
    }
    let level = ellipsoid_level(observer, radii);
    if level <= 1.0 || !level.is_finite() {
        return Err(GeoError::BadSearch(
            "the observer is inside the target ellipsoid, so the ray through the centre has no exterior intercept",
        ));
    }
    let point = ray_surface_intersection(radii, observer, negate(observer))?
        .ok_or(GeoError::BadSearch("center ray misses ellipsoid"))?;
    Ok(SurfacePoint {
        point,
        altitude: norm(subtract(observer, point)),
    })
}

/// `(x/a)^2 + (y/b)^2 + (z/c)^2`; `1` on the ellipsoid, `< 1` inside.
fn ellipsoid_level(point: [f64; 3], radii: [f64; 3]) -> f64 {
    let x = point[0] / radii[0];
    let y = point[1] / radii[1];
    let z = point[2] / radii[2];
    x * x + y * y + z * z
}

/// Near-side sub-solar point of `target`. The Sun is NAIF id 10.
pub fn sub_solar_point(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<SurfacePoint, GeoError> {
    sub_solar_point_with_method(kernel, pck, target, et, abcorr, SubPointMethod::NearPoint)
}

/// Sub-solar point of `target` with an explicit definition.
///
/// Matches CSPICE `subslr` with `OBSRVR = 'SUN'`: the target-to-observer
/// light time fixes the target epoch, and the Sun's apparent position is
/// then taken **as seen from the target** at that epoch. Computing it as
/// the negation of the target's apparent position seen from the Sun is
/// not equivalent once stellar aberration is on, because aberration is
/// not symmetric between the two bodies.
pub fn sub_solar_point_with_method(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    et: f64,
    abcorr: Aberration,
    method: SubPointMethod,
) -> Result<SurfacePoint, GeoError> {
    let geometry = sub_solar_geometry(kernel, pck, target, 10, et, abcorr, method)?;
    Ok(SurfacePoint {
        point: geometry.point,
        altitude: geometry.altitude,
    })
}

/// A CSPICE surface point, target epoch, and apparent observer-to-point vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceGeometry {
    /// Surface point in target body-fixed coordinates, km.
    pub point: [f64; 3],
    /// Epoch at the surface point, ET seconds.
    pub epoch: f64,
    /// Observer-to-surface light time, seconds.
    pub light_time: f64,
    /// Apparent observer-to-surface vector in the target frame, km.
    pub surface_vector: [f64; 3],
    /// Length of the observer-to-surface vector, km.
    pub altitude: f64,
}

// SPKEZP applies STELAB to positions, while SPKEZR uses the state
// aberration routines. Their mathematically identical position results
// can differ in the final bits, which matter in a single LT iteration.
pub(crate) fn apparent_position(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    et: f64,
    correction: Aberration,
) -> Result<[f64; 3], GeoError> {
    Ok(crate::spkapo::corrected_position(kernel, target, observer, et, correction)?.0)
}

// SPKEZP's non-inertial branch computes SPKAPO in J2000, then rotates
// at the corrected frame-center epoch. SPKEZR uses different stellar math.
pub(crate) fn apparent_position_in_frame(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    et: f64,
    correction: Aberration,
    frame: &FrameSource<'_>,
) -> Result<([f64; 3], f64), GeoError> {
    if let FrameSource::Named {
        frames,
        pck,
        frame_name,
    } = frame
    {
        return kernel.position_corrected_in_frame(
            target,
            observer,
            et,
            correction,
            frames,
            Some(pck),
            frame_name,
        );
    }
    let (position, lt) =
        crate::spkapo::corrected_position(kernel, target, observer, et, correction)?;
    let rotation = frame.rotation_at(time_at_target(et, lt, correction))?;
    Ok((rotate(&rotation, position), lt))
}

fn subpoint(
    position: [f64; 3],
    radii: [f64; 3],
    method: SubPointMethod,
) -> Result<SurfacePoint, GeoError> {
    match method {
        SubPointMethod::NearPoint => nearest_point_on_ellipsoid(position, radii),
        SubPointMethod::Intercept => intercept_through_center(position, radii),
    }
}

/// Ellipsoidal CSPICE `subpnt`, including iteration at the surface epoch.
pub fn sub_observer_geometry(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    method: SubPointMethod,
) -> Result<SurfaceGeometry, GeoError> {
    sub_observer_geometry_in(
        kernel,
        pck,
        &Frames::new(),
        &format!("IAU_{target}"),
        target,
        observer,
        et,
        abcorr,
        method,
    )
}

/// The same CSPICE calculation in an explicit target-centered body-fixed frame.
pub fn sub_observer_geometry_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    method: SubPointMethod,
) -> Result<SurfaceGeometry, GeoError> {
    validate_target_frame(frames, target_frame, target)?;
    if target == observer {
        return Err(GeoError::BadSearch("observer and target coincide"));
    }
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let frame = FrameSource::Named {
        frames,
        pck,
        frame_name: target_frame,
    };
    let (position, _) = apparent_position_in_frame(kernel, target, observer, et, abcorr, &frame)?;
    let mut obspos = negate(position);
    let mut result = subpoint(obspos, radii, method)?;
    let (uselt, cn, stellar, transmission) = abcorr.flags();
    let mut lt = result.altitude / crate::spk::CLIGHT;
    let mut epoch = time_at_target(et, lt, abcorr);
    if uselt {
        let obs = kernel.state_in_frame(observer, 0, et, frames, Some(pck), "J2000")?;
        for _ in 0..if cn { 5 } else { 1 } {
            let center = kernel.state_in_frame(target, 0, epoch, frames, Some(pck), "J2000")?;
            let rotation = frame.rotation_at(epoch)?;
            obspos = rotate(&rotation, subtract(obs.position, center.position));
            if stellar {
                let subvec = subtract(result.point, obspos);
                let inertial = rotate(&crate::frame::transpose(&rotation), subvec);
                let velocity = if transmission {
                    negate(obs.velocity)
                } else {
                    obs.velocity
                };
                let corrected = stellar_aberration_position(inertial, velocity);
                let offset = subtract(rotate(&rotation, corrected), subvec);
                obspos = subtract(obspos, offset);
            }
            result = subpoint(obspos, radii, method)?;
            let next_lt = result.altitude / crate::spk::CLIGHT;
            let next_epoch = time_at_target(et, next_lt, abcorr);
            let stop = (next_lt - lt).abs() <= next_lt.abs() * 1e-17 || next_epoch == epoch;
            lt = next_lt;
            epoch = next_epoch;
            if stop {
                break;
            }
        }
    }
    let surface_vector = subtract(result.point, obspos);
    Ok(SurfaceGeometry {
        point: result.point,
        epoch,
        light_time: lt,
        surface_vector,
        altitude: norm(surface_vector),
    })
}

pub(crate) fn validate_target_frame(
    frames: &Frames,
    name: &str,
    target: i32,
) -> Result<(), GeoError> {
    if frames.center(name)? != target || frames.is_inertial(name)? {
        return Err(GeoError::BadSearch(
            "target frame must be non-inertial and centered on the target",
        ));
    }
    Ok(())
}

fn fixed_surface(
    target: i32,
    target_frame: &str,
    point: [f64; 3],
) -> crate::location::PointLocation {
    crate::location::PointLocation::fixed(target, target_frame, point)
}

/// Ellipsoidal CSPICE `subslr`, using the caller's actual observer.
/// Transmission aberration corrections are rejected, as in CSPICE.
pub fn sub_solar_geometry(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    method: SubPointMethod,
) -> Result<SurfaceGeometry, GeoError> {
    sub_solar_geometry_in(
        kernel,
        pck,
        &Frames::new(),
        &format!("IAU_{target}"),
        target,
        observer,
        et,
        abcorr,
        method,
    )
}

/// The same CSPICE calculation in an explicit target-centered body-fixed frame.
pub fn sub_solar_geometry_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    method: SubPointMethod,
) -> Result<SurfaceGeometry, GeoError> {
    validate_target_frame(frames, target_frame, target)?;
    use crate::location::{state_between, Endpoint};
    if target == observer {
        return Err(GeoError::BadSearch("observer and target coincide"));
    }
    let (uselt, cn, stellar, transmission) = abcorr.flags();
    if transmission {
        return Err(GeoError::BadAberration("SPICE(NOTSUPPORTED)".into()));
    }
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let frame = FrameSource::Named {
        frames,
        pck,
        frame_name: target_frame,
    };
    let (initial_target, initial_lt) =
        apparent_position_in_frame(kernel, target, observer, et, abcorr, &frame)?;
    let initial_epoch = time_at_target(et, initial_lt, abcorr);
    let mut obspos = negate(initial_target);
    let sun = kernel
        .position_corrected_in_frame(
            10,
            target,
            initial_epoch,
            abcorr,
            frames,
            Some(pck),
            target_frame,
        )?
        .0;
    let mut point = subpoint(sun, radii, method)?.point;
    let mut lt = norm(subtract(point, obspos)) / crate::spk::CLIGHT;
    let mut epoch = time_at_target(et, lt, abcorr);
    if uselt {
        let obs = kernel.state_in_frame(observer, 0, et, frames, Some(pck), "J2000")?;
        for _ in 0..if cn { 10 } else { 1 } {
            let center = kernel.state_in_frame(target, 0, epoch, frames, Some(pck), "J2000")?;
            let rotation = frame.rotation_at(epoch)?;
            obspos = rotate(&rotation, subtract(obs.position, center.position));
            let location = fixed_surface(target, target_frame, point);
            let sun = state_between(
                kernel,
                frames,
                pck,
                Endpoint::Object(10),
                Endpoint::Point(&location),
                epoch,
                abcorr,
            )?;
            let (state_rotation, _) = frame.at(epoch)?;
            let sunpos = add(rotate(&state_rotation, sun.state.position), point);
            point = subpoint(sunpos, radii, method)?.point;
            let next_lt = norm(subtract(point, obspos)) / crate::spk::CLIGHT;
            let next_epoch = time_at_target(et, next_lt, abcorr);
            let stop = (next_lt - lt).abs() <= next_lt.abs() * 1e-17 || next_epoch == epoch;
            lt = next_lt;
            epoch = next_epoch;
            // SUBSLR refreshes the un-aberrated observer at the updated epoch.
            let center = kernel.state_in_frame(target, 0, epoch, frames, Some(pck), "J2000")?;
            let rotation = frame.rotation_at(epoch)?;
            obspos = rotate(&rotation, subtract(obs.position, center.position));
            if stop {
                break;
            }
        }
    }
    let surface_vector = if stellar {
        let location = fixed_surface(target, target_frame, point);
        let seen = state_between(
            kernel,
            frames,
            pck,
            Endpoint::Point(&location),
            Endpoint::Object(observer),
            et,
            abcorr,
        )?;
        let (rotation, _) = frame.at(time_at_target(et, seen.light_time, abcorr))?;
        rotate(&rotation, seen.state.position)
    } else {
        subtract(point, obspos)
    };
    Ok(SurfaceGeometry {
        point,
        epoch,
        light_time: lt,
        surface_vector,
        altitude: norm(surface_vector),
    })
}

/// CSPICE NEARPT nearest point and signed altitude on a triaxial ellipsoid.
pub fn nearest_point_on_ellipsoid(
    point: [f64; 3],
    radii: [f64; 3],
) -> Result<SurfacePoint, GeoError> {
    if radii.iter().any(|axis| *axis <= 0.0) {
        return Err(GeoError::BadSearch("ellipsoid axes must be positive"));
    }

    // CSPICE NEARPT scales by the smallest semi-axis, solves the lambda
    // equation, projects the result back to the ellipsoid, then sharpens
    // its normal. These operations also determine its last floating bits.
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| radii[i].partial_cmp(&radii[j]).unwrap());
    let scaling = 1.0 / radii[order[0]];
    let axes = [
        radii[order[0]] * scaling,
        radii[order[1]] * scaling,
        radii[order[2]] * scaling,
    ];
    let original = [
        point[order[0]] * scaling,
        point[order[1]] * scaling,
        point[order[2]] * scaling,
    ];
    let original_norm = norm(original);
    let squares = [axes[0] * axes[0], axes[1] * axes[1], axes[2] * axes[2]];
    let mut current = original;
    let mut best = [0.0; 3];
    let mut best_height = 0.0;
    let mut previous_error = f64::MAX;
    let mut sign = 1.0;
    for pass in 1..=6 {
        let mut copy = current;
        for i in 0..3 {
            if current[i] * 0.5 + axes[i] == axes[i] || current[i] * 0.5 - axes[i] == -axes[i] {
                current[i] = 0.0;
            }
        }
        let mut q = ellipsoid_level(current, axes) - 1.0;
        let mut inside = q < 0.0;
        if pass == 1 {
            sign = if q >= 0.0 { 1.0 } else { -1.0 };
        }
        let singular = current.iter().position(|&v| v != 0.0).unwrap_or(3);
        let (mut lower, mut upper, mut lambda) = if q == 0.0 {
            (0.0, 0.0, 0.0)
        } else if q > 0.0 {
            let lo = ((current[0].abs() * 0.5 - axes[0]) * axes[0])
                .max((current[1].abs() * 0.5 - axes[1]) * axes[1])
                .max((current[2].abs() * 0.5 - axes[2]) * axes[2])
                .max(0.0);
            let hi = (axes[0] * current[0])
                .abs()
                .max((axes[1] * current[1]).abs())
                .max((axes[2] * current[2]).abs())
                * 2.0;
            (lo, hi, hi)
        } else if singular == 3 {
            current[0] = if copy[0] < 0.0 { -axes[0] } else { axes[0] };
            copy = [current[0], 0.0, 0.0];
            q = 0.0;
            inside = false;
            (0.0, 0.0, 0.0)
        } else {
            let mut lo = -squares[2];
            for i in 0..3 {
                if current[i] != 0.0 {
                    lo = lo.max(axes[i] * (current[i].abs() * 0.5 - axes[i]));
                }
            }
            (lo, lo.max(0.0), lo)
        };
        let evaluate = |lambda: f64| {
            let mut terms = [0.0; 3];
            for i in 0..3 {
                if current[i] != 0.0 {
                    let denominator = axes[i] + lambda / axes[i];
                    let factor = if inside && current[i].abs() * 0.5 > denominator {
                        2.0
                    } else {
                        current[i] / denominator
                    };
                    terms[i] = factor * factor;
                }
            }
            terms[0] + terms[1] + terms[2] - 1.0
        };
        let other = evaluate(lambda);
        let mut qlower = if inside { other } else { q };
        let mut qupper = if inside { q } else { other };
        qlower = qlower.max(0.0);
        qupper = qupper.min(0.0);
        lambda = upper;
        q = qupper;
        for _ in 0..2048 {
            if upper - lower <= 0.0 {
                break;
            }
            lower = lower.min(upper);
            qlower = qlower.max(0.0);
            qupper = qupper.min(0.0);
            if q == 0.0 {
                lower = lambda;
                upper = lambda;
            } else {
                if q < 0.0 {
                    upper = lambda;
                    qupper = q;
                } else {
                    lower = lambda;
                    qlower = q;
                }
                lambda = (lower * 0.5 + upper * 0.5).clamp(lower, upper);
            }
            if (lambda - lower).abs() <= 1e-16 || (lambda - upper).abs() <= 1e-16 {
                if qlower.abs() < qupper.abs() {
                    upper = lower;
                } else {
                    lower = upper;
                }
            }
            if upper - lower > 0.0 {
                q = evaluate(lambda);
            }
        }
        let mut surface = [0.0; 3];
        for i in 0..3 {
            if current[i] != 0.0 {
                surface[i] = copy[i] / (lower / squares[i] + 1.0);
            }
        }
        // Interior near points may lie off the central plane even when
        // the input lies exactly in it (NEARPT's extra candidate).
        if inside
            && (singular == 1 || singular == 2)
            && axes[0] != axes[singular]
            && current[1].abs() <= axes[1] - squares[0] / axes[1]
            && current[2].abs() <= axes[2] - squares[0] / axes[2]
        {
            let mut extra = [0.0; 3];
            let mut valid = false;
            if axes[0] == axes[1] {
                let denom = 1.0 - squares[0] / squares[2];
                if denom > 0.0 {
                    extra[2] = current[2] / denom;
                    let t = extra[2] / axes[2];
                    let t = 1.0 - 0.0 - t * t;
                    if t > 0.0 {
                        extra[0] = axes[0] * t.max(0.0).sqrt();
                        valid = true;
                    }
                }
            } else {
                let d2 = 1.0 - squares[0] / squares[1];
                let d3 = 1.0 - squares[0] / squares[2];
                if d2 > 0.0 && d3 > 0.0 {
                    extra[1] = current[1] / d2;
                    extra[2] = current[2] / d3;
                    let t2 = extra[1] / axes[1];
                    let t3 = extra[2] / axes[2];
                    let t = 1.0 - t2 * t2 - t3 * t3;
                    if t > 0.0 {
                        extra[0] = axes[0] * t.sqrt();
                        valid = true;
                    }
                }
            }
            if valid && norm(subtract(extra, current)) < norm(subtract(surface, current)) {
                surface = extra;
            }
        }
        if pass == 1 {
            current = surface;
            best = surface;
            best_height = norm(subtract(best, original));
            continue;
        }
        let height = sign * norm(subtract(surface, original));
        let normal = ellipsoid_normal(surface, axes);
        let error = subtract(original, add(surface, scale(normal, height)));
        let new_error = norm(error);
        if pass > 2 && new_error >= previous_error {
            break;
        }
        previous_error = new_error;
        best = surface;
        best_height = height;
        let mut perpendicular = perpendicular(error, normal);
        if perpendicular == [0.0; 3] {
            break;
        }
        if sign >= 0.0 {
            perpendicular = scale(perpendicular, norm(surface) / original_norm);
        }
        current = add(surface, perpendicular);
        if current == surface {
            break;
        }
    }
    best = scale(best, 1.0 / scaling);
    let mut surface = [0.0; 3];
    for i in 0..3 {
        surface[order[i]] = best[i];
    }
    Ok(SurfacePoint {
        point: surface,
        altitude: best_height / scaling,
    })
}

/// Illumination angles at a surface point, in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IlluminationAngles {
    /// Phase angle between the observer and the illuminator.
    pub phase: f64,
    /// Solar incidence angle: normal to the Sun.
    pub solar: f64,
    /// Emission angle: normal to the observer.
    pub emission: f64,
}

/// Surface intercept point along the observer-to-center line.
///
/// Matches CSPICE `sincpt` when the ray direction is the vector from
/// the observer to the target center.
pub fn surface_intercept_point(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<SurfacePoint, GeoError> {
    let direction = kernel
        .state_corrected(target, observer, et, abcorr)?
        .state
        .position;
    let result = surface_intercept_geometry(kernel, pck, target, observer, et, abcorr, direction)?;
    Ok(SurfacePoint {
        point: result.point,
        altitude: result.altitude,
    })
}

/// Intercept point of an arbitrary ray on the target ellipsoid.
///
/// `direction_j2000` is the ray direction in J2000, from the observer.
/// Matches CSPICE `sincpt` with method `ELLIPSOID`.
pub fn surface_intercept_ray(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
) -> Result<SurfacePoint, GeoError> {
    let result =
        surface_intercept_geometry(kernel, pck, target, observer, et, abcorr, direction_j2000)?;
    Ok(SurfacePoint {
        point: result.point,
        altitude: result.altitude,
    })
}

/// Ellipsoidal CSPICE `sincpt` for a ray expressed in J2000.
/// Returns an error when the corrected ray misses the target.
pub fn surface_intercept_geometry(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
) -> Result<SurfaceGeometry, GeoError> {
    surface_intercept_geometry_in(
        kernel,
        pck,
        &Frames::new(),
        &format!("IAU_{target}"),
        target,
        observer,
        et,
        abcorr,
        direction_j2000,
    )
}

/// The same CSPICE calculation in an explicit target-centered body-fixed frame.
pub fn surface_intercept_geometry_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
) -> Result<SurfaceGeometry, GeoError> {
    surface_intercept_geometry_with_model_in(
        kernel,
        pck,
        frames,
        target_frame,
        target,
        observer,
        et,
        abcorr,
        direction_j2000,
        SurfaceModel::Ellipsoid,
    )
}

/// Illumination angles at `point` on the target ellipsoid.
///
/// `point` is expressed in the body-fixed frame. The Sun is NAIF id 10.
/// Matches CSPICE `ilumin` with method `ELLIPSOID`.
pub fn illumination_angles(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    point: [f64; 3],
) -> Result<IlluminationAngles, GeoError> {
    Ok(illumination_geometry(kernel, pck, target, 10, observer, et, abcorr, point)?.angles)
}

/// CSPICE illumination result at an ellipsoid surface point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IlluminationGeometry {
    /// Phase, incidence, and emission angles in radians.
    pub angles: IlluminationAngles,
    /// Surface epoch, ET seconds.
    pub epoch: f64,
    /// Observer-to-surface light time, seconds.
    pub light_time: f64,
    /// Apparent observer-to-surface vector in the target frame, km.
    pub surface_vector: [f64; 3],
    /// Whether the surface point is visible to the observer.
    pub visible: bool,
    /// Whether the surface point is illuminated.
    pub illuminated: bool,
}

/// Ellipsoidal CSPICE `illumg`/`illumf` for an arbitrary illuminator.
pub fn illumination_geometry(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    illuminator: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    point: [f64; 3],
) -> Result<IlluminationGeometry, GeoError> {
    illumination_geometry_in(
        kernel,
        pck,
        &Frames::new(),
        &format!("IAU_{target}"),
        target,
        illuminator,
        observer,
        et,
        abcorr,
        point,
    )
}

/// The same CSPICE calculation in an explicit target-centered body-fixed frame.
pub fn illumination_geometry_in(
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
) -> Result<IlluminationGeometry, GeoError> {
    illumination_geometry_with_model_in(
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
        SurfaceModel::Ellipsoid,
    )
}

/// Planetocentric solar longitude in radians, in `[0, 2*pi)`.
///
/// Matches the WebGeocalc definition: the longitude of the body-Sun
/// vector in a right-handed frame whose `+Z` is the instantaneous
/// angular velocity of the body's orbit about the Sun, `+X` is the
/// body's north spin axis crossed with `+Z`, and `+Y = Z x X`.
pub fn planetocentric_solar_longitude(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    et: f64,
    abcorr: Aberration,
) -> Result<f64, GeoError> {
    let (_, _, _, transmission) = abcorr.flags();
    if transmission {
        return Err(GeoError::BadAberration(
            "planetocentric solar longitude supports reception corrections only".to_string(),
        ));
    }
    // Geometric orbit of the body about the Sun (NAIF id 10).
    let orbit = kernel.state_corrected(target, 10, et, Aberration::None)?;
    let momentum = unit_cross(orbit.state.position, orbit.state.velocity);
    if momentum == [0.0; 3] {
        return Err(GeoError::BadSearch("degenerate body orbit"));
    }

    // The body's north spin axis in J2000 is the third row of the
    // J2000-to-body matrix.
    let rotation = pck
        .body_rotation(target, et)
        .ok_or(GeoError::MissingPckData(target))?;
    let spin_axis = [rotation[2][0], rotation[2][1], rotation[2][2]];
    let x = unit_cross(spin_axis, momentum);
    let y = unit_cross(momentum, x);

    // Sun relative to the body, with the requested aberration correction.
    let sun = kernel.state_corrected(10, target, et, abcorr)?;
    let sun_vector = sun.state.position;

    let mut longitude = dot(sun_vector, y).atan2(dot(sun_vector, x));
    if longitude < 0.0 {
        longitude += std::f64::consts::TAU;
    }
    Ok(longitude)
}

/// Planetodetic (geodetic) latitude, longitude, and altitude to
/// rectangular coordinates on an ellipsoid with `a == b`.
///
/// Angles in radians, altitude in kilometres. The triaxial `a != b`
/// planetodetic definition is not implemented.
pub fn planetodetic_to_rectangular(
    latitude: f64,
    longitude: f64,
    altitude: f64,
    radii: [f64; 3],
) -> Result<[f64; 3], GeoError> {
    let [a, b, c] = radii;
    if (a - b).abs() > 1e-9 * a {
        return Err(GeoError::BadSearch(
            "planetodetic conversion requires equatorial axes a == b",
        ));
    }

    if a <= 0.0 || c <= 0.0 {
        return Err(GeoError::BadSearch("ellipsoid axes must be positive"));
    }
    let flattening = 1.0 - c / a;
    let polar = a - flattening * a;
    let cp = latitude.cos();
    let sp = latitude.sin();
    let cl = longitude.cos();
    let sl = longitude.sin();
    let big = (a * cp).abs().max((polar * sp).abs());
    let x = a * cp / big;
    let y = polar * sp / big;
    let factor = 1.0 / (big * (x * x + y * y).sqrt());
    let base = [
        factor * a * a * cl * cp,
        factor * a * a * sl * cp,
        factor * polar * polar * sp,
    ];
    let normal = ellipsoid_normal(base, [a, a, polar]);
    Ok(add(base, scale(normal, altitude)))
}

/// CSPICE `VSEP`.
fn angle_between(first: [f64; 3], second: [f64; 3]) -> f64 {
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
    let cosine = u1[0] * u2[0] + u1[1] * u2[1] + u1[2] * u2[2];
    if cosine > 0.0 {
        2.0 * (norm(subtract(u1, u2)) * 0.5).asin()
    } else if cosine < 0.0 {
        std::f64::consts::PI - 2.0 * (norm(add(u1, u2)) * 0.5).asin()
    } else {
        std::f64::consts::PI * 0.5
    }
}

fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub(crate) fn unit(a: [f64; 3]) -> [f64; 3] {
    let length = norm(a);
    if length == 0.0 {
        [0.0; 3]
    } else {
        [a[0] / length, a[1] / length, a[2] / length]
    }
}

// Operation order follows CSPICE STELAB -> VROTV -> VPROJ.
pub(crate) fn stellar_aberration_position(position: [f64; 3], velocity: [f64; 3]) -> [f64; 3] {
    let axis = cross(unit(position), scale(velocity, 1.0 / crate::spk::CLIGHT));
    let sine = norm(axis);
    if sine == 0.0 {
        return position;
    }
    let axis = unit(axis);
    let projection = project(position, axis);
    let plane = subtract(position, projection);
    let second = cross(axis, plane);
    let angle = sine.asin();
    add(
        add(scale(plane, angle.cos()), scale(second, angle.sin())),
        projection,
    )
}

// CSPICE SURFPT: scale to the unit sphere and resolve the ray into
// parallel and perpendicular components, avoiding quadratic cancellation.
pub(crate) fn ray_surface_intersection(
    radii: [f64; 3],
    origin: [f64; 3],
    direction: [f64; 3],
) -> Result<Option<[f64; 3]>, GeoError> {
    if radii.iter().any(|&r| r <= 0.0) {
        return Err(GeoError::BadSearch("ellipsoid axes must be positive"));
    }
    if direction == [0.0; 3] {
        return Err(GeoError::BadSearch("ray direction is zero"));
    }
    let x = [
        direction[0] / radii[0],
        direction[1] / radii[1],
        direction[2] / radii[2],
    ];
    let y = [
        origin[0] / radii[0],
        origin[1] / radii[1],
        origin[2] / radii[2],
    ];
    let p = perpendicular(y, x);
    let projection = subtract(y, p);
    let ymag = norm(y);
    let pmag = norm(p);
    let ux = unit(x);
    let sign = if ymag > 1.0 {
        if pmag > 1.0 || dot(projection, x) > 0.0 {
            return Ok(None);
        }
        -1.0
    } else if ymag == 1.0 {
        return Ok(Some(origin));
    } else {
        1.0
    };
    let factor = (1.0 - pmag * pmag).max(0.0).sqrt();
    let result = add(p, scale(ux, sign * factor));
    Ok(Some([
        result[0] * radii[0],
        result[1] * radii[1],
        result[2] * radii[2],
    ]))
}

fn ellipsoid_normal(point: [f64; 3], radii: [f64; 3]) -> [f64; 3] {
    let minimum = radii[0].min(radii[1]).min(radii[2]);
    let ratios = [minimum / radii[0], minimum / radii[1], minimum / radii[2]];
    unit([
        point[0] * (ratios[0] * ratios[0]),
        point[1] * (ratios[1] * ratios[1]),
        point[2] * (ratios[2] * ratios[2]),
    ])
}
pub(crate) fn perpendicular(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let ma = a[0].abs().max(a[1].abs()).max(a[2].abs());
    let mb = b[0].abs().max(b[1].abs()).max(b[2].abs());
    if ma == 0.0 {
        return [0.0; 3];
    }
    if mb == 0.0 {
        return a;
    }
    let t = [a[0] / ma, a[1] / ma, a[2] / ma];
    let r = [b[0] / mb, b[1] / mb, b[2] / mb];
    scale(subtract(t, project(t, r)), ma)
}

pub(crate) fn project(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let ma = a[0].abs().max(a[1].abs()).max(a[2].abs());
    let mb = b[0].abs().max(b[1].abs()).max(b[2].abs());
    if ma == 0.0 || mb == 0.0 {
        return [0.0; 3];
    }
    let t = [a[0] / ma, a[1] / ma, a[2] / ma];
    let r = [b[0] / mb, b[1] / mb, b[2] / mb];
    scale(r, dot(t, r) * ma / dot(r, r))
}

fn rotate(matrix: &Mat3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn negate(a: [f64; 3]) -> [f64; 3] {
    [-a[0], -a[1], -a[2]]
}

pub(crate) fn norm(a: [f64; 3]) -> f64 {
    let maximum = a[0].abs().max(a[1].abs()).max(a[2].abs());
    if maximum == 0.0 {
        return 0.0;
    }
    let x = a[0] / maximum;
    let y = a[1] / maximum;
    let z = a[2] / maximum;
    maximum * (x * x + y * y + z * z).sqrt()
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn unit_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
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

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[cfg(test)]
mod tests {
    use super::nearest_point_on_ellipsoid;

    #[test]
    fn nearest_point_on_axis() {
        let result =
            nearest_point_on_ellipsoid([5000.0, 0.0, 0.0], [1000.0, 2000.0, 3000.0]).unwrap();
        assert!((result.point[0] - 1000.0).abs() < 1e-9);
        assert!(result.point[1].abs() < 1e-9);
        assert!(result.point[2].abs() < 1e-9);
        assert!((result.altitude - 4000.0).abs() < 1e-9);
    }

    #[test]
    fn interior_point_has_negative_altitude() {
        let point = nearest_point_on_ellipsoid([1.0, 1.0, 1.0], [1000.0, 2000.0, 3000.0]).unwrap();
        assert!(point.altitude < 0.0);
    }
}

/// Surface used by a ray-intercept calculation.
#[derive(Clone, Copy, Debug)]
pub enum SurfaceModel<'a> {
    Ellipsoid,
    DskUnprioritized {
        kernels: &'a crate::DskSet,
        surfaces: &'a [i32],
    },
}
/// Native DSK sub-observer definitions. NADIR uses the reference ellipsoid
/// to define a ray and intersects that ray with the actual DSK surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DskSubPointMethod {
    Nadir,
    Intercept,
}
/// CSPICE SINCPT with an explicit ellipsoid or DSK surface model.
pub fn surface_intercept_geometry_with_model_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    direction_j2000: [f64; 3],
    model: SurfaceModel<'_>,
) -> Result<SurfaceGeometry, GeoError> {
    validate_target_frame(frames, target_frame, target)?;
    if target == observer {
        return Err(GeoError::BadSearch("observer and target coincide"));
    }
    if norm(direction_j2000) == 0.0 {
        return Err(GeoError::BadSearch("ray direction is zero"));
    }
    let radii = match model {
        SurfaceModel::Ellipsoid => pck.radii(target).ok_or(GeoError::MissingPckData(target))?,
        SurfaceModel::DskUnprioritized { kernels, surfaces } => {
            [kernels
                .radial_bounds(kernel, pck, frames, target, surfaces)?
                .1; 3]
        }
    };
    let intercept =
        |origin: [f64; 3], direction: [f64; 3], epoch: f64| -> Result<Option<[f64; 3]>, GeoError> {
            match model {
                SurfaceModel::Ellipsoid => ray_surface_intersection(radii, origin, direction),
                SurfaceModel::DskUnprioritized { kernels, surfaces } => Ok(kernels
                    .ray_intercept_in(
                        kernel,
                        pck,
                        frames,
                        target,
                        epoch,
                        target_frame,
                        origin,
                        direction,
                        surfaces,
                    )?
                    .map(|hit| hit.point)),
            }
        };
    let (uselt, cn, stellar, transmission) = abcorr.flags();
    let geometric_corr = match (uselt, cn, transmission) {
        (false, _, _) => Aberration::None,
        (true, false, false) => Aberration::LightTime,
        (true, true, false) => Aberration::ConvergedLightTime,
        (true, false, true) => Aberration::TransmissionLightTime,
        (true, true, true) => Aberration::TransmissionConvergedLightTime,
    };
    let frame = FrameSource::Named {
        frames,
        pck,
        frame_name: target_frame,
    };
    let (initial_position, initial_lt) =
        apparent_position_in_frame(kernel, target, observer, et, geometric_corr, &frame)?;
    let mut epoch = time_at_target(et, initial_lt, abcorr);
    let mut obspos = negate(initial_position);
    let obs = kernel.state_in_frame(observer, 0, et, frames, Some(pck), "J2000")?;
    let mut direction = direction_j2000;
    if stellar {
        // Invert STELAB/STLABX by the same five-step error iteration used
        // in CSPICE ZZSFXCOR, rather than negating an apparent position.
        let velocity = if transmission {
            negate(obs.velocity)
        } else {
            obs.velocity
        };
        direction = stellar_aberration_position(direction, negate(velocity));
        for _ in 0..5 {
            let estimate = stellar_aberration_position(direction, velocity);
            let error = subtract(direction_j2000, estimate);
            direction = add(direction, error);
            if norm(error) / norm(direction) <= 1e-17 {
                break;
            }
        }
    }
    let mut rotation = frame.rotation_at(epoch)?;
    let mut ray = rotate(&rotation, direction);
    if matches!(model, SurfaceModel::DskUnprioritized { .. }) {
        let range = norm(obspos);
        if range == 0. {
            return Err(GeoError::BadSearch("observer-target distance is zero"));
        }
        if range > radii[0] * 1.01
            && angle_between(negate(obspos), ray) > (radii[0] * 1.01 / range).asin()
        {
            return Err(GeoError::BadSearch("ray does not intersect the target"));
        }
    }
    let mut point = intercept(obspos, ray, epoch)?;
    // A near-limb miss can be caused by the initial target-center light
    // time. Re-estimate it from the closest point to the ray before
    // deciding that no intercept exists (ZZSFXCOR recovery loop).
    let recovered = point.is_none();
    if recovered {
        for _ in 0..if cn { 3 } else { 1 } {
            let (near, _) = crate::tangent::nearest_point_on_ellipsoid_to_line(radii, obspos, ray)?;
            let lt = norm(subtract(obspos, near)) / crate::spk::CLIGHT;
            epoch = time_at_target(et, lt, abcorr);
            let center = kernel.state_in_frame(target, 0, epoch, frames, Some(pck), "J2000")?;
            rotation = frame.rotation_at(epoch)?;
            obspos = rotate(&rotation, subtract(obs.position, center.position));
            ray = rotate(&rotation, direction);
            point = intercept(obspos, ray, epoch)?;
            if point.is_some() {
                break;
            }
        }
    }
    let mut point = point.ok_or(GeoError::BadSearch("ray does not intersect the target"))?;
    let mut final_found = true;
    let mut lt = norm(subtract(point, obspos)) / crate::spk::CLIGHT;
    if uselt || recovered {
        epoch = time_at_target(et, lt, abcorr);
        let mut previous_lt = 0.0;
        for iteration in 0..if cn { 10 } else { 1 } {
            let center = kernel.state_in_frame(target, 0, epoch, frames, Some(pck), "J2000")?;
            rotation = frame.rotation_at(epoch)?;
            obspos = rotate(&rotation, subtract(obs.position, center.position));
            ray = rotate(&rotation, direction);
            let intercept = intercept(obspos, ray, epoch)?;
            final_found = intercept.is_some();
            point = match intercept {
                Some(p) => p,
                None if cn && iteration < 9 => {
                    let (near, _) =
                        crate::tangent::nearest_point_on_ellipsoid_to_line(radii, obspos, ray)?;
                    add(project(subtract(near, obspos), ray), obspos)
                }
                None => return Err(GeoError::BadSearch("ray does not intersect the target")),
            };
            lt = norm(subtract(point, obspos)) / crate::spk::CLIGHT;
            let next_epoch = time_at_target(et, lt, abcorr);
            let stop = (lt - previous_lt).abs() <= lt.abs() * 1e-17 || next_epoch == epoch;
            previous_lt = lt;
            epoch = next_epoch;
            if stop {
                break;
            }
        }
    }
    if !final_found {
        return Err(GeoError::BadSearch("ray does not intersect the target"));
    }
    let altitude = norm(subtract(point, obspos));
    let surface_vector = if uselt || recovered {
        scale(unit(rotate(&rotation, direction_j2000)), altitude)
    } else {
        subtract(point, obspos)
    };
    Ok(SurfaceGeometry {
        point,
        epoch,
        // Derive the output from the returned SINCPT vector; the internal
        // iteration range can differ in its final binary64 bits.
        light_time: norm(surface_vector) / crate::spk::CLIGHT,
        surface_vector,
        altitude: norm(surface_vector),
    })
}

/// Native SUBPNT with DSK/UNPRIORITIZED and NADIR or INTERCEPT.
pub fn sub_observer_dsk_geometry_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    kernels: &crate::DskSet,
    surfaces: &[i32],
    method: DskSubPointMethod,
) -> Result<SurfaceGeometry, GeoError> {
    validate_target_frame(frames, target_frame, target)?;
    if target == observer {
        return Err(GeoError::BadSearch("observer and target coincide"));
    }
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let frame = FrameSource::Named {
        frames,
        pck,
        frame_name: target_frame,
    };
    let (position, center_lt) =
        apparent_position_in_frame(kernel, target, observer, et, abcorr, &frame)?;
    let initial_epoch = time_at_target(et, center_lt, abcorr);
    let subpoint = |position: [f64; 3], epoch: f64| -> Result<SurfacePoint, GeoError> {
        let direction = match method {
            DskSubPointMethod::Nadir => {
                subtract(nearest_point_on_ellipsoid(position, radii)?.point, position)
            }
            DskSubPointMethod::Intercept => negate(position),
        };
        let point = kernels
            .ray_intercept_in(
                kernel,
                pck,
                frames,
                target,
                epoch,
                target_frame,
                position,
                direction,
                surfaces,
            )?
            .ok_or(GeoError::BadSearch("DSK sub-observer point not found"))?
            .point;
        Ok(SurfacePoint {
            point,
            altitude: norm(subtract(point, position)),
        })
    };
    let mut obspos = negate(position);
    let mut result = subpoint(obspos, initial_epoch)?;
    let (uselt, cn, stellar, transmission) = abcorr.flags();
    let mut lt = result.altitude / crate::spk::CLIGHT;
    let mut epoch = time_at_target(et, lt, abcorr);
    if uselt {
        let obs = kernel.state_in_frame(observer, 0, et, frames, Some(pck), "J2000")?;
        for _ in 0..if cn { 5 } else { 1 } {
            let center = kernel.state_in_frame(target, 0, epoch, frames, Some(pck), "J2000")?;
            let rotation = frame.rotation_at(epoch)?;
            obspos = rotate(&rotation, subtract(obs.position, center.position));
            if stellar {
                let subvec = subtract(result.point, obspos);
                let inertial = rotate(&crate::frame::transpose(&rotation), subvec);
                let velocity = if transmission {
                    negate(obs.velocity)
                } else {
                    obs.velocity
                };
                let corrected = stellar_aberration_position(inertial, velocity);
                let offset = subtract(rotate(&rotation, corrected), subvec);
                obspos = subtract(obspos, offset);
            }
            result = subpoint(obspos, epoch)?;
            let next_lt = result.altitude / crate::spk::CLIGHT;
            let next_epoch = time_at_target(et, next_lt, abcorr);
            let stop = (next_lt - lt).abs() <= next_lt.abs() * 1e-17 || next_epoch == epoch;
            lt = next_lt;
            epoch = next_epoch;
            if stop {
                break;
            }
        }
    }
    let surface_vector = subtract(result.point, obspos);
    Ok(SurfaceGeometry {
        point: result.point,
        epoch,
        light_time: lt,
        surface_vector,
        altitude: norm(surface_vector),
    })
}
/// Native SUBSLR with DSK/UNPRIORITIZED and NADIR or INTERCEPT.
pub fn sub_solar_dsk_geometry_in(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target_frame: &str,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    kernels: &crate::DskSet,
    surfaces: &[i32],
    method: DskSubPointMethod,
) -> Result<SurfaceGeometry, GeoError> {
    validate_target_frame(frames, target_frame, target)?;
    use crate::location::{state_between, Endpoint};
    if target == observer {
        return Err(GeoError::BadSearch("observer and target coincide"));
    }
    let (uselt, cn, stellar, transmission) = abcorr.flags();
    if transmission {
        return Err(GeoError::BadAberration("SPICE(NOTSUPPORTED)".into()));
    }
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let frame = FrameSource::Named {
        frames,
        pck,
        frame_name: target_frame,
    };
    let (initial_target, initial_lt) =
        apparent_position_in_frame(kernel, target, observer, et, abcorr, &frame)?;
    let initial_epoch = time_at_target(et, initial_lt, abcorr);
    let mut obspos = negate(initial_target);
    let sun = kernel
        .position_corrected_in_frame(
            10,
            target,
            initial_epoch,
            abcorr,
            frames,
            Some(pck),
            target_frame,
        )?
        .0;
    let subpoint = |position: [f64; 3], epoch: f64| -> Result<[f64; 3], GeoError> {
        let direction = match method {
            DskSubPointMethod::Nadir => {
                subtract(nearest_point_on_ellipsoid(position, radii)?.point, position)
            }
            DskSubPointMethod::Intercept => negate(position),
        };
        Ok(kernels
            .ray_intercept_in(
                kernel,
                pck,
                frames,
                target,
                epoch,
                target_frame,
                position,
                direction,
                surfaces,
            )?
            .ok_or(GeoError::BadSearch("DSK sub-solar point not found"))?
            .point)
    };
    let mut point = subpoint(sun, initial_epoch)?;
    let mut lt = norm(subtract(point, obspos)) / crate::spk::CLIGHT;
    let mut epoch = time_at_target(et, lt, abcorr);
    if uselt {
        let obs = kernel.state_in_frame(observer, 0, et, frames, Some(pck), "J2000")?;
        for _ in 0..if cn { 10 } else { 1 } {
            let center = kernel.state_in_frame(target, 0, epoch, frames, Some(pck), "J2000")?;
            let rotation = frame.rotation_at(epoch)?;
            obspos = rotate(&rotation, subtract(obs.position, center.position));
            let location = fixed_surface(target, target_frame, point);
            let sun = state_between(
                kernel,
                frames,
                pck,
                Endpoint::Object(10),
                Endpoint::Point(&location),
                epoch,
                abcorr,
            )?;
            let (state_rotation, _) = frame.at(epoch)?;
            let sunpos = add(rotate(&state_rotation, sun.state.position), point);
            point = subpoint(sunpos, epoch)?;
            let next_lt = norm(subtract(point, obspos)) / crate::spk::CLIGHT;
            let next_epoch = time_at_target(et, next_lt, abcorr);
            let stop = (next_lt - lt).abs() <= next_lt.abs() * 1e-17 || next_epoch == epoch;
            lt = next_lt;
            epoch = next_epoch;
            // SUBSLR refreshes the un-aberrated observer at the updated epoch.
            let center = kernel.state_in_frame(target, 0, epoch, frames, Some(pck), "J2000")?;
            let rotation = frame.rotation_at(epoch)?;
            obspos = rotate(&rotation, subtract(obs.position, center.position));
            if stop {
                break;
            }
        }
    }
    let surface_vector = if stellar {
        let location = fixed_surface(target, target_frame, point);
        let seen = state_between(
            kernel,
            frames,
            pck,
            Endpoint::Point(&location),
            Endpoint::Object(observer),
            et,
            abcorr,
        )?;
        let (rotation, _) = frame.at(time_at_target(et, seen.light_time, abcorr))?;
        rotate(&rotation, seen.state.position)
    } else {
        subtract(point, obspos)
    };
    Ok(SurfaceGeometry {
        point,
        epoch,
        light_time: lt,
        surface_vector,
        altitude: norm(surface_vector),
    })
}

/// Native ILLUMF with an explicit surface model. N0067's DSK illumination
/// normal/occlusion path considers all loaded target surfaces, matching
/// its unconditional zero surface-list count after parsing the method.
pub fn illumination_geometry_with_model_in(
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
    model: SurfaceModel<'_>,
) -> Result<IlluminationGeometry, GeoError> {
    use crate::location::{state_between, Endpoint};
    if target == observer {
        return Err(GeoError::BadSearch("SPICE(BODIESNOTDISTINCT)"));
    }
    validate_target_frame(frames, target_frame, target).map_err(|error| match error {
        GeoError::BadSearch("target frame must be non-inertial and centered on the target") => {
            GeoError::BadSearch("SPICE(INVALIDFRAME)")
        }
        other => other,
    })?;

    let location = fixed_surface(target, target_frame, point);
    let seen = state_between(
        kernel,
        frames,
        pck,
        Endpoint::Point(&location),
        Endpoint::Object(observer),
        et,
        abcorr,
    )?;
    let epoch = time_at_target(et, seen.light_time, abcorr);
    let frame = FrameSource::Named {
        frames,
        pck,
        frame_name: target_frame,
    };
    let (rotation, _) = frame.at(epoch)?;
    let surface_vector = if abcorr == Aberration::None {
        add(
            point,
            kernel
                .state_in_frame(target, observer, et, frames, Some(pck), target_frame)?
                .position,
        )
    } else {
        rotate(&rotation, seen.state.position)
    };
    let source = state_between(
        kernel,
        frames,
        pck,
        Endpoint::Object(illuminator),
        Endpoint::Point(&location),
        epoch,
        abcorr,
    )?;
    let to_source = if abcorr == Aberration::None {
        subtract(
            kernel
                .state_in_frame(illuminator, target, epoch, frames, Some(pck), target_frame)?
                .position,
            point,
        )
    } else {
        rotate(&rotation, source.state.position)
    };
    let normal = match model {
        SurfaceModel::Ellipsoid => {
            let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
            ellipsoid_normal(point, radii)
        }
        // Source ILLUMF resets SVNSRF=0 unconditionally for the DSK
        // normal and self-occultation callbacks, even after parsing SURFACES.
        SurfaceModel::DskUnprioritized { kernels, .. } => {
            if kernels.kernels().is_empty() {
                return Err(GeoError::BadDas("SPICE(NOLOADEDDSKFILES)"));
            }
            if !kernels
                .kernels()
                .iter()
                .flat_map(|kernel| kernel.segments())
                .any(|segment| segment.descriptor.center == target)
            {
                return Err(GeoError::BadDas("SPICE(DSKDATANOTFOUND)"));
            }
            unit(
                kernels
                    .normal_at_in(kernel, pck, frames, target, epoch, target_frame, point, &[])
                    .map_err(|error| match error {
                        GeoError::BadDas(
                            "point is not on a DSK surface" | "point is not inside a DSK segment",
                        ) => GeoError::BadDas("SPICE(POINTOFFSURFACE)"),
                        other => other,
                    })?,
            )
        }
    };
    let to_observer = negate(surface_vector);
    let angles = IlluminationAngles {
        phase: angle_between(to_observer, to_source),
        solar: angle_between(normal, to_source),
        emission: angle_between(normal, to_observer),
    };
    let mut visible = angles.emission <= std::f64::consts::FRAC_PI_2;
    let mut illuminated = angles.solar <= std::f64::consts::FRAC_PI_2;
    if let SurfaceModel::DskUnprioritized { kernels, .. } = model {
        if visible || illuminated {
            let max_radius = kernels.radial_bounds(kernel, pck, frames, target, &[])?.1;
            let scale = max_radius * 1e-10;
            let vertex = std::array::from_fn(|i| 1. * point[i] + scale * normal[i]);
            if visible {
                visible = kernels
                    .ray_intercept_in(
                        kernel,
                        pck,
                        frames,
                        target,
                        epoch,
                        target_frame,
                        vertex,
                        to_observer,
                        &[],
                    )?
                    .is_none();
            }
            if illuminated {
                illuminated = kernels
                    .ray_intercept_in(
                        kernel,
                        pck,
                        frames,
                        target,
                        epoch,
                        target_frame,
                        vertex,
                        to_source,
                        &[],
                    )?
                    .is_none();
            }
        }
    }
    Ok(IlluminationGeometry {
        angles,
        epoch,
        light_time: seen.light_time,
        surface_vector,
        visible,
        illuminated,
    })
}
