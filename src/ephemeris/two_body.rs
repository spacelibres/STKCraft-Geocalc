// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::assign_op_pattern)]
//! N0067 PROP2B and STMP03 universal-variable conic propagation.
use crate::surface::norm;
use crate::{GeoError, State};
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn propagate(gm: f64, initial: State, dt: f64) -> Result<State, GeoError> {
    if !gm.is_finite()
        || gm <= 0.0
        || !dt.is_finite()
        || !initial
            .position
            .iter()
            .chain(initial.velocity.iter())
            .all(|v| v.is_finite())
    {
        return Err(GeoError::BadSearch(
            "two-body inputs must be finite and GM positive",
        ));
    }
    let pos = initial.position;
    let vel = initial.velocity;
    let r0 = norm(pos);
    let rv = dot(pos, vel);
    let angular = cross(pos, vel);
    let h2 = dot(angular, angular);
    if r0 == 0.0 || norm(vel) == 0.0 || h2 == 0.0 {
        return Err(GeoError::BadSearch("SPICE(NONCONICMOTION)"));
    }
    let tmp = cross(vel, angular);
    let eq = std::array::from_fn(|i| (1.0 / gm) * tmp[i] + (-1.0 / r0) * pos[i]);
    let e = norm(eq);
    let q = h2 / (gm * (e + 1.0));
    let f = 1.0 - e;
    let b = (q / gm).sqrt();
    let br0 = b * r0;
    let b2rv = b * b * rv;
    let bq = b * q;
    let qovr0 = q / r0;
    let maximum = 1.0_f64
        .max(br0.abs())
        .max(b2rv.abs())
        .max(bq.abs())
        .max((qovr0 / bq).abs());
    let bound = if f < 0.0 {
        let fixed = (f64::MAX / 2.0).ln() - maximum.ln();
        let root = (-f).sqrt();
        (fixed / root).min((fixed + (-f).ln() * 1.5) / root)
    } else {
        ((1.5_f64.ln() + f64::MAX.ln() - maximum.ln()) / 3.0).exp()
    };
    let clamp = |x: f64| x.max(-bound).min(bound);
    let mut x = clamp(dt / bq);
    let mut c = stumpff(f * x * x)?;
    let mut kfun = x * (br0 * c[1] + x * (b2rv * c[2] + x * (bq * c[3])));
    let (mut lower, mut upper);
    if dt < 0.0 {
        upper = 0.0;
        lower = x;
        while kfun > dt {
            upper = lower;
            lower *= 2.0;
            let previous = x;
            x = clamp(lower);
            if x == previous {
                return Err(GeoError::BadSearch("SPICE(DTOUTOFRANGE)"));
            }
            c = stumpff(f * x * x)?;
            kfun = x * (br0 * c[1] + x * (b2rv * c[2] + x * (bq * c[3])));
        }
    } else if dt > 0.0 {
        lower = 0.0;
        upper = x;
        while kfun < dt {
            lower = upper;
            upper *= 2.0;
            let previous = x;
            x = clamp(upper);
            if x == previous {
                return Err(GeoError::BadSearch("SPICE(DTOUTOFRANGE)"));
            }
            c = stumpff(f * x * x)?;
            kfun = x * (br0 * c[1] + x * (b2rv * c[2] + x * bq * c[3]));
        }
    } else {
        return Ok(initial);
    }
    x = upper.min(lower.max((lower + upper) / 2.0));
    c = stumpff(f * x * x)?;
    let mut count = 0;
    let mut limit = 1000;
    while x > lower && x < upper && count < limit {
        kfun = x * (br0 * c[1] + x * (b2rv * c[2] + x * bq * c[3]));
        if kfun > dt {
            upper = x;
        } else if kfun < dt {
            lower = x;
        } else {
            lower = x;
            upper = x;
        }
        if limit > 64 && upper != 0.0 && lower != 0.0 {
            limit = 64;
            count = 0;
        }
        x = upper.min(lower.max((lower + upper) / 2.0));
        c = stumpff(f * x * x)?;
        count += 1;
    }
    let x2 = x * x;
    let x3 = x2 * x;
    let br = br0 * c[0] + x * (b2rv * c[1] + x * (bq * c[2]));
    let pc = 1.0 - qovr0 * x2 * c[2];
    let vc = dt - bq * x3 * c[3];
    let pcdot = -(qovr0 / br) * x * c[1];
    let vcdot = 1.0 - bq / br * x2 * c[2];
    Ok(State {
        position: std::array::from_fn(|i| pc * pos[i] + vc * vel[i]),
        velocity: std::array::from_fn(|i| pcdot * pos[i] + vcdot * vel[i]),
    })
}
fn stumpff(x: f64) -> Result<[f64; 4], GeoError> {
    let y = 2.0_f64.ln() + f64::MAX.ln();
    if x <= -y * y {
        return Err(GeoError::BadSearch("SPICE(VALUEOUTOFRANGE)"));
    }
    if x < -1.0 {
        let z = (-x).sqrt();
        let c0 = z.cosh();
        let c1 = z.sinh() / z;
        return Ok([c0, c1, (1.0 - c0) / x, (1.0 - c1) / x]);
    }
    if x > 1.0 {
        let z = x.sqrt();
        let c0 = z.cos();
        let c1 = z.sin() / z;
        return Ok([c0, c1, (1.0 - c0) / x, (1.0 - c1) / x]);
    }
    let pair = |i: usize| 1.0 / (i as f64 * (i + 1) as f64);
    let mut c3 = 1.0;
    for j in (2..=9).rev() {
        let i = j * 2;
        c3 = 1.0 - x * pair(i) * c3;
    }
    c3 = pair(2) * c3;
    let mut c2 = 1.0;
    for j in (1..=8).rev() {
        let i = j * 2 + 1;
        c2 = 1.0 - x * pair(i) * c2;
    }
    c2 = pair(1) * c2;
    Ok([1.0 - x * c2, 1.0 - x * c3, c2, c3])
}

/// SPKE15 conic propagation followed by optional J2 precession.
pub(crate) fn evaluate_type15(record: &[f64], et: f64) -> Result<State, GeoError> {
    use crate::surface::{project, unit};
    if record.len() != 16 {
        return Err(GeoError::BadDaf("type15 record must contain16 words"));
    }
    let p = record[7];
    let eccentricity = record[8];
    let flag = record[9] as i32;
    let gm = record[13];
    let j2 = record[14];
    let radius = record[15];
    let peri = unit([record[4], record[5], record[6]]);
    let pole = unit([record[1], record[2], record[3]]);
    let central_pole = unit([record[10], record[11], record[12]]);
    if p <= 0.0
        || eccentricity < 0.0
        || gm <= 0.0
        || radius < 0.0
        || peri == [0.0; 3]
        || pole == [0.0; 3]
        || central_pole == [0.0; 3]
        || dot(peri, pole).abs() > 1e-5
    {
        return Err(GeoError::BadDaf("invalid type15 orbit/pole parameters"));
    }
    let near = p / (eccentricity + 1.0);
    let speed = (gm / p).sqrt() * (eccentricity + 1.0);
    let velocity = cross(pole, peri);
    let initial = State {
        position: peri.map(|v| near * v),
        velocity: velocity.map(|v| v * speed),
    };
    let dt = et - record[0];
    let mut state = propagate(gm, initial, dt)?;
    if flag != 3 && j2 != 0.0 && eccentricity < 1.0 && near > radius {
        let one_minus_e2 = 1.0 - eccentricity * eccentricity;
        let mean_rate = one_minus_e2 / p * (gm * one_minus_e2 / p).sqrt();
        let mean = mean_rate * dt;
        let tau = std::f64::consts::TAU;
        let mut theta = mean - tau * (mean / tau).trunc();
        if theta.abs() > std::f64::consts::PI {
            theta -= tau.copysign(theta);
        }
        let whole_turns = mean - theta;
        let anomaly = crate::fov::separation(peri, state.position)
            .abs()
            .copysign(if theta < 0.0 { -1.0 } else { 1.0 })
            + whole_turns;
        let cosinc = dot(central_pole, pole);
        let ratio = radius / p;
        let z = anomaly * 1.5 * j2 * (ratio * ratio);
        let node = -z * cosinc;
        let periapse = z * (cosinc * cosinc * 2.5 - 0.5);
        // VROTV normalizes again, projects, crosses, then forms two sums.
        let rotate = |v: [f64; 3], axis: [f64; 3], angle: f64| {
            let axis = unit(axis);
            let projection = project(v, axis);
            let planar = std::array::from_fn(|i| v[i] - projection[i]);
            let perpendicular = cross(axis, planar);
            let c = angle.cos();
            let s = angle.sin();
            std::array::from_fn(|i| (c * planar[i] + s * perpendicular[i]) + projection[i])
        };
        if flag != 1 {
            state.position = rotate(state.position, pole, periapse);
            state.velocity = rotate(state.velocity, pole, periapse);
        }
        if flag != 2 {
            state.position = rotate(state.position, central_pole, node);
            state.velocity = rotate(state.velocity, central_pole, node);
        }
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_prop2b_states_equal_cspice() {
        let mut failures = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/prop2b.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<f64> = line.split(',').map(|v| v.parse().unwrap()).collect();
            let initial = State {
                position: [f[1], f[2], f[3]],
                velocity: [f[4], f[5], f[6]],
            };
            let state = propagate(f[0], initial, f[7]).unwrap();
            let actual = [state.position, state.velocity].concat();
            if actual != f[8..] {
                failures.push(format!("dt{}: {actual:?} != {:?}", f[7], &f[8..]));
            }
            count += 1;
        }
        assert_eq!(count, 288);
        assert!(
            failures.is_empty(),
            "{} failed:\n{}",
            failures.len(),
            failures
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
