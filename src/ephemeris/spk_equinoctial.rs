//! N0067 SPKE17/EQNCPV/KEPLEQ/KPSOLV evaluation order.
use crate::{GeoError, State};
pub(crate) fn evaluate(record: &[f64], et: f64) -> Result<State, GeoError> {
    if record.len() != 12 {
        return Err(GeoError::BadDaf("type17 record must contain12 words"));
    }
    let eq = &record[1..10];
    if eq[0] <= 0.0 || (eq[1] * eq[1] + eq[2] * eq[2]).sqrt() > 0.9 {
        return Err(GeoError::BadDaf(
            "invalid type17 semimajor axis or eccentricity",
        ));
    }
    let (sa, ca, sd, cd) = (
        record[10].sin(),
        record[10].cos(),
        record[11].sin(),
        record[11].cos(),
    );
    let transform = [
        [-sa, -ca * sd, ca * cd],
        [ca, -sa * sd, sa * cd],
        [0.0, cd, sd],
    ];
    let dt = et - record[0];
    let a = eq[0];
    let dlpdt = eq[6];
    let dlp = dt * dlpdt;
    let (can, san) = (dlp.cos(), dlp.sin());
    let h = eq[1] * can + eq[2] * san;
    let k = eq[2] * can - eq[1] * san;
    let nodedt = eq[8];
    let node = dt * nodedt;
    let (cn, sn) = (node.cos(), node.sin());
    let p = eq[4] * cn + eq[5] * sn;
    let q = eq[5] * cn - eq[4] * sn;
    let mldt = eq[7];
    let prate = dlpdt - nodedt;
    let b = 1.0 / ((1.0 - h * h - k * k).sqrt() + 1.0);
    let di = 1.0 / (p * p + 1.0 + q * q);
    let vf = [(1.0 - p * p + q * q) * di, p * 2.0 * q * di, p * -2.0 * di];
    let vg = [p * 2.0 * q * di, (p * p + 1.0 - q * q) * di, q * 2.0 * di];
    let change = mldt * dt;
    let tau = std::f64::consts::TAU;
    let ml = eq[3] + (change - tau * (change / tau).trunc());
    if h * h + k * k >= 0.81 {
        return Err(GeoError::BadSearch("SPICE(ECCOUTOFBOUNDS)"));
    }
    let evec = [-h * ml.cos() + k * ml.sin(), h * ml.sin() + k * ml.cos()];
    let eccentric_longitude = ml + kpsolv(evec);
    let sf = eccentric_longitude.sin();
    let cf = eccentric_longitude.cos();
    let x1 = a * ((1.0 - b * (h * h)) * cf + (h * k * b * sf - k));
    let y1 = a * ((1.0 - b * (k * k)) * sf + (h * k * b * cf - h));
    let rb = h * sf + k * cf;
    let r = a * (1.0 - rb);
    let ra = mldt * a * a / r;
    let dx1 = ra * (-sf + h * b * rb);
    let dy1 = ra * (cf - k * b * rb);
    let nfac = 1.0 - dlpdt / mldt;
    let dx = nfac * dx1 - prate * y1;
    let dy = nfac * dy1 + prate * x1;
    let position = std::array::from_fn(|i| x1 * vf[i] + y1 * vg[i]);
    let temp = [-nodedt * position[1], nodedt * position[0], 0.0];
    let velocity = std::array::from_fn(|i| 1.0 * temp[i] + dx * vf[i] + dy * vg[i]);
    let rotate = |v: [f64; 3]| {
        std::array::from_fn(|i| {
            transform[i][0] * v[0] + transform[i][1] * v[1] + transform[i][2] * v[2]
        })
    };
    Ok(State {
        position: rotate(position),
        velocity: rotate(velocity),
    })
}
fn kpsolv(evec: [f64; 2]) -> f64 {
    let (h, k) = (evec[0], evec[1]);
    let ecc = (h * h + k * k).sqrt();
    if h == 0.0 {
        return 0.0;
    }
    let (mut lower, mut upper) = if -h > 0.0 { (-ecc, 0.0) } else { (0.0, ecc) };
    let count = ((1.0 / (1.0 - ecc)).round() as usize).clamp(1, 32);
    let mut midpoint = 0.0;
    for _ in 0..count {
        midpoint = lower.max(upper.min((lower + upper) * 0.5));
        if midpoint - h * midpoint.cos() - k * midpoint.sin() > 0.0 {
            upper = midpoint;
        } else {
            lower = midpoint;
        }
    }
    let mut x = midpoint;
    for _ in 0..5 {
        let cosx = x.cos();
        let sinx = x.sin();
        let y = x - h * cosx - k * sinx;
        let derivative = h * sinx + 1.0 - k * cosx;
        x -= y / derivative;
    }
    x
}
