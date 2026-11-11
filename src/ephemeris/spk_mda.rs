//! N0067 SPKE01/SPKE21 modified divided-difference evaluation.
use crate::{GeoError, State};
pub(crate) fn evaluate(record: &[f64], dimension: usize, et: f64) -> Result<State, GeoError> {
    if !(1..=25).contains(&dimension) || record.len() != 4 * dimension + 11 {
        return Err(GeoError::BadDaf("invalid difference-line dimension"));
    }
    let maximum = record[4 * dimension + 7] as usize;
    let orders: [usize; 3] = std::array::from_fn(|i| record[4 * dimension + 8 + i] as usize);
    if maximum < 3
        || maximum > dimension + 1
        || orders.iter().any(|q| *q > dimension || *q >= maximum)
    {
        return Err(GeoError::BadDaf(
            "invalid difference-line integration order",
        ));
    }
    let g = &record[1..1 + dimension];
    let delta = et - record[0];
    let mut tp = delta;
    let mut fc = vec![0.0; dimension + 1];
    let mut wc = vec![0.0; dimension];
    let mut w = vec![0.0; dimension + 3];
    for j in 1..=maximum - 2 {
        if g[j - 1] == 0.0 {
            return Err(GeoError::BadDaf("zero difference-line step"));
        }
        fc[j] = tp / g[j - 1];
        wc[j - 1] = delta / g[j - 1];
        tp = delta + g[j - 1];
    }
    for j in 1..=maximum {
        w[j - 1] = 1.0 / j as f64;
    }
    let mut ks = maximum - 1;
    let mut ks1 = ks - 1;
    let mut jx = 0;
    while ks >= 2 {
        jx += 1;
        for j in 1..=jx {
            w[j + ks - 1] = fc[j] * w[j + ks1 - 1] - wc[j - 1] * w[j + ks - 1];
        }
        ks = ks1;
        ks1 -= 1;
    }
    let mut state = State::ZERO;
    for i in 0..3 {
        let mut sum = 0.0;
        for j in (1..=orders[i]).rev() {
            sum += record[dimension + 7 + i * dimension + j - 1] * w[j + ks - 1];
        }
        state.position[i] =
            record[dimension + 1 + i * 2] + delta * (record[dimension + 2 + i * 2] + delta * sum);
    }
    for j in 1..=jx {
        w[j + ks - 1] = fc[j] * w[j + ks1 - 1] - wc[j - 1] * w[j + ks - 1];
    }
    ks -= 1;
    for i in 0..3 {
        let mut sum = 0.0;
        for j in (1..=orders[i]).rev() {
            sum += record[dimension + 7 + i * dimension + j - 1] * w[j + ks - 1];
        }
        state.velocity[i] = record[dimension + 2 + i * 2] + delta * sum;
    }
    Ok(state)
}
