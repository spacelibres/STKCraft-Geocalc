// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(
    clippy::excessive_precision,
    clippy::needless_range_loop,
    clippy::assign_op_pattern
)]
//! N0067 IAU1976 precession, IAU1980 nutation and TEME state transforms.
use crate::{FrameRotation, GeoError};
pub(crate) fn precession(et: f64) -> FrameRotation {
    let century = 31557600.0 * 100.0;
    let t = et / century;
    let scale = (std::f64::consts::PI / 180.0) / 3600.0;
    let zeta = t * (t * (t * 0.017998 + 0.30188) + 2306.2181) * scale;
    let z = t * (t * (t * 0.018203 + 1.09468) + 2306.2181) * scale;
    let theta = t * (t * (t * -0.041833 - 0.42665) + 2004.3109) * scale;
    let ts = 1.0 / century;
    let dzeta = ts * (t * (t * 3.0 * 0.017998 + 0.60375999999999996) + 2306.2181) * scale;
    let dz = ts * (t * (t * 3.0 * 0.018203 + 2.1893600000000002) + 2306.2181) * scale;
    let dtheta = ts * (t * (t * 3.0 * -0.041833 - 0.85329999999999995) + 2004.3109) * scale;
    crate::orientation::euler_state([-z, theta, -zeta, -dz, dtheta, -dzeta], [2, 1, 2]).unwrap()
}
pub(crate) fn nutation(et: f64) -> FrameRotation {
    let d = wahr(et);
    let t = et / 31557600.0 / 100.0;
    let rad = std::f64::consts::PI / 180.0;
    let mean = rad / 3600.0 * (t * (t * (t * 0.001813 - 5.9e-4) - 46.815) + 84381.448);
    let rate = rad / 3600.0
        * (t * (t * 3.0 * 0.001813 - 0.0011800000000000001) - 46.815)
        * (1.0 / (31557600.0 * 100.0));
    crate::orientation::euler_state(
        [-mean - d[1], -d[0], mean, -rate - d[3], -d[2], rate],
        [0, 2, 0],
    )
    .unwrap()
}
fn modulo(value: f64, divisor: f64) -> f64 {
    value - divisor * (value / divisor).trunc()
}
pub(crate) fn wahr(et: f64) -> [f64; 4] {
    let radians = 180.0 / std::f64::consts::PI;
    let factor = (radians * 3600.0) * 1e4;
    let coefficients = [
        [
            134.96298138888886,
            13.064992947243136,
            6.5192872572139397e-4,
            3.6484365631332527e-7,
        ],
        [
            357.52772333333331,
            0.98560028309377146,
            -1.201414483363923e-5,
            -6.8408185558748495e-8,
        ],
        [
            93.271910277777778,
            13.229350240603848,
            -2.760338267929901e-4,
            6.2707503428852773e-8,
        ],
        [
            297.85036305555559,
            12.190749116495549,
            -1.4348262053484912e-4,
            1.0831296046801845e-7,
        ],
        [
            125.04452222222223,
            -0.052953764841432813,
            1.5522608272925558e-4,
            4.5605457039165659e-8,
        ],
    ];
    let dj = et / 86400.0;
    let dd = dj / 1e4;
    let dddj = dd / 1e4;
    let dd2 = dd * dd;
    let t = dj / 365250.0;
    let mut angles = [0.0; 5];
    let mut rates = [0.0; 5];
    for (j, c) in coefficients.iter().enumerate() {
        angles[j] = modulo(c[0] + dj * c[1] + (c[2] + dd * c[3]) * dd2, 360.0) / radians;
        rates[j] = modulo(c[1] + dddj * (c[2] * 2.0 + dd * 3.0 * c[3]), 360.0) / radians;
    }
    let mut result = [0.0; 4];
    for term in crate::wahr_table::WAHR {
        let mut arg = 0.0;
        let mut rate = 0.0;
        for j in 0..5 {
            if term[j] != 0 {
                arg += term[j] as f64 * angles[j];
                rate += term[j] as f64 * rates[j];
                arg = modulo(arg, std::f64::consts::TAU);
            }
        }
        let mut cl = term[5] as f64;
        if term[6] != 0 {
            cl += term[6] as f64 * t;
        }
        let mut ce = term[7] as f64;
        if term[8] != 0 {
            ce += term[8] as f64 * t;
        }
        let cos = arg.cos();
        let sin = arg.sin();
        result[0] += cl * sin / factor;
        result[1] += ce * cos / factor;
        result[2] += cl * cos * rate / factor;
        result[3] -= ce * sin * rate / factor;
    }
    result[2] /= 86400.0;
    result[3] /= 86400.0;
    result
}
/// TEME to J2000, with the source two-vector derivative construction.
pub(crate) fn teme(et: f64) -> Result<FrameRotation, GeoError> {
    let precession = inverse(precession(et));
    let nutation = inverse(nutation(et));
    let x = column(precession, 0);
    let z = apply(precession, column(nutation, 2));
    let col3 = normalize(z);
    let col2 = unit_cross_state(z, x);
    let col1 = unit_cross_state(col2, z);
    if col1[..3] == [0.0; 3] {
        return Err(GeoError::BadSearch("SPICE(DEPENDENTVECTORS)"));
    }
    let columns = [col1, col2, col3];
    Ok(FrameRotation {
        rotation: std::array::from_fn(|i| std::array::from_fn(|j| columns[j][i])),
        rotation_dot: std::array::from_fn(|i| std::array::from_fn(|j| columns[j][i + 3])),
    })
}
fn column(x: FrameRotation, j: usize) -> [f64; 6] {
    [
        x.rotation[0][j],
        x.rotation[1][j],
        x.rotation[2][j],
        x.rotation_dot[0][j],
        x.rotation_dot[1][j],
        x.rotation_dot[2][j],
    ]
}
pub(crate) fn apply(x: FrameRotation, v: [f64; 6]) -> [f64; 6] {
    std::array::from_fn(|i| {
        let mut sum = 0.0;
        for j in 0..6 {
            let a = if i < 3 && j < 3 {
                x.rotation[i][j]
            } else if i >= 3 && j >= 3 {
                x.rotation[i - 3][j - 3]
            } else if i >= 3 {
                x.rotation_dot[i - 3][j]
            } else {
                0.0
            };
            sum += a * v[j];
        }
        sum
    })
}
pub(crate) fn normalize(v: [f64; 6]) -> [f64; 6] {
    let (p, d) = crate::spk::unit_and_derivative([v[0], v[1], v[2]], [v[3], v[4], v[5]]);
    [p[0], p[1], p[2], d[0], d[1], d[2]]
}
pub(crate) fn unit_cross_state(mut a: [f64; 6], mut b: [f64; 6]) -> [f64; 6] {
    for state in [&mut a, &mut b] {
        let maximum = state[..3].iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
        if maximum > 0.0 {
            let factor = 1.0 / maximum;
            for value in state {
                *value = factor * *value;
            }
        }
    }
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let p = cross([a[0], a[1], a[2]], [b[0], b[1], b[2]]);
    let first = cross([a[3], a[4], a[5]], [b[0], b[1], b[2]]);
    let second = cross([a[0], a[1], a[2]], [b[3], b[4], b[5]]);
    normalize([
        p[0],
        p[1],
        p[2],
        first[0] + second[0],
        first[1] + second[1],
        first[2] + second[2],
    ])
}
#[cfg(test)]
// Keep each source primitive's independent regression adjacent to its implementation.
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    fn expand(x: FrameRotation) -> Vec<f64> {
        (0..6)
            .flat_map(|i| {
                (0..6).map(move |j| {
                    if i < 3 && j < 3 {
                        x.rotation[i][j]
                    } else if i >= 3 && j >= 3 {
                        x.rotation[i - 3][j - 3]
                    } else if i >= 3 {
                        x.rotation_dot[i - 3][j]
                    } else {
                        0.0
                    }
                })
            })
            .collect()
    }
    #[test]
    fn native_earth_orientation_transforms_exact() {
        let mut failures = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/earth_orientation.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let et = f[1].parse().unwrap();
            let expected: Vec<f64> = f[2..].iter().map(|v| v.parse().unwrap()).collect();
            let actual = match f[0] {
                "zzwahr_" => wahr(et).to_vec(),
                "zzeprc76_" => expand(precession(et)),
                "zzenut80_" => expand(nutation(et)),
                "zzteme_" => expand(inverse(teme(et).unwrap())),
                "teme_inverse" => expand(teme(et).unwrap()),
                _ => panic!("kind"),
            };
            if actual != expected {
                failures.push(format!("{} {}: {actual:?} != {expected:?}", f[0], et));
            }
            count += 1;
        }
        assert_eq!(count, 545);
        assert!(
            failures.is_empty(),
            "{}/{} failed:\n{}",
            failures.len(),
            count,
            failures
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
fn inverse(x: FrameRotation) -> FrameRotation {
    FrameRotation {
        rotation: crate::frame::transpose(&x.rotation),
        rotation_dot: crate::frame::transpose(&x.rotation_dot),
    }
}
