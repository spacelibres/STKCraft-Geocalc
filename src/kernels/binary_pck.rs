// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::assign_op_pattern)]
//! Native binary PCK type-2, type-3 and type-20 orientation records.
//! Class IDs identify orientation models and are independent of body IDs.
use crate::{
    daf::DafFile,
    error::GeoError,
    frame::{mat_mul, rot3, rotate_matrix, Mat3},
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct BinaryPck {
    file: Arc<DafFile>,
}
impl std::fmt::Debug for BinaryPck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinaryPck")
            .field("segments", &self.file.segments().len())
            .finish()
    }
}
impl BinaryPck {
    #[cfg(test)]
    pub(crate) fn open(path: impl AsRef<std::path::Path>) -> Result<Self, GeoError> {
        Self::from_bytes(std::fs::read(path)?)
    }
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Result<Self, GeoError> {
        if !bytes.starts_with(b"DAF/PCK") {
            return Err(GeoError::NotDaf);
        }
        let file = DafFile::from_bytes(bytes)?;
        if file.nd() != 2 || file.ni() != 5 {
            return Err(GeoError::BadDaf(
                "binary PCK summaries require ND=2 and NI=5",
            ));
        }
        Ok(Self {
            file: Arc::new(file),
        })
    }
    /// Reference-to-body rotation and derivative, with its actual reference ID.
    pub(crate) fn state_rotation(
        &self,
        class_id: i32,
        et: f64,
    ) -> Result<Option<(i32, Mat3, Mat3)>, GeoError> {
        if !et.is_finite() {
            return Err(GeoError::BadSearch("epoch must be finite"));
        }
        let Some(segment) = self
            .file
            .segments()
            .iter()
            .rev()
            .find(|s| s.ic[0] == class_id && s.dc[0] <= et && et <= s.dc[1])
        else {
            return Ok(None);
        };
        let begin = usize::try_from(segment.ic[3])
            .map_err(|_| GeoError::BadDaf("negative PCK segment address"))?;
        let end = usize::try_from(segment.ic[4])
            .map_err(|_| GeoError::BadDaf("negative PCK segment address"))?;
        if begin == 0 || end < begin + 3 {
            return Err(GeoError::BadDaf("invalid PCK segment word range"));
        }
        match segment.ic[2] {
            3 => {
                let (r, d) = self.type3(begin, end, et)?;
                return Ok(Some((segment.ic[1], r, d)));
            }
            20 => {
                let (r, d) = self.type20(begin, end, et)?;
                return Ok(Some((segment.ic[1], r, d)));
            }
            2 => {}
            other => return Err(GeoError::Unsupported(format!("binary PCK type {other}"))),
        }
        let directory = self.file.read_doubles(end - 3, 4)?;
        let interval = directory[1];
        let size = directory[2] as usize;
        let count = directory[3] as usize;
        if !interval.is_finite() || interval <= 0.0 || size < 5 || (size - 2) % 3 != 0 || count == 0
        {
            return Err(GeoError::BadDaf("invalid PCK type-2 directory"));
        }
        // PCKR02 truncates the nonnegative interval number and clamps only
        // the final coverage endpoint to the last record.
        let index = (((et - directory[0]) / interval) as usize).min(count - 1);
        let address = begin
            .checked_add(
                index
                    .checked_mul(size)
                    .ok_or(GeoError::BadDaf("PCK record address overflow"))?,
            )
            .ok_or(GeoError::BadDaf("PCK record address overflow"))?;
        if address + size - 1 > end - 4 {
            return Err(GeoError::BadDaf("PCK record exceeds segment data"));
        }
        let record = self.file.read_doubles(address, size)?;
        let radius = record[1];
        if !radius.is_finite() || radius <= 0.0 {
            return Err(GeoError::BadDaf("PCK interval radius must be positive"));
        }
        let n = (size - 2) / 3;
        let tau = (et - record[0]) / radius;
        let mut angles = [0.0; 6];
        for i in 0..3 {
            let (value, rate) = chebyshev(&record[2 + i * n..2 + (i + 1) * n], tau);
            angles[i] = value;
            angles[i + 3] = rate / radius;
        }
        let tau = std::f64::consts::TAU;
        angles[2] -= tau * (angles[2] / tau).trunc();
        let (rotation, derivative) = euler_state_313([
            angles[2], angles[1], angles[0], angles[5], angles[4], angles[3],
        ]);
        Ok(Some((segment.ic[1], rotation, derivative)))
    }
    fn type20(&self, begin: usize, end: usize, et: f64) -> Result<(Mat3, Mat3), GeoError> {
        let c = self.file.read_doubles(end - 6, 7)?;
        let (scale, time, jd, fraction, days) = (c[0], c[1], c[2], c[3], c[4]);
        let size = c[5] as usize;
        let n = c[6] as usize;
        if size < 6 || size % 3 != 0 || n == 0 || days <= 0.0 || time <= 0.0 {
            return Err(GeoError::BadDaf("invalid PCK20 control data"));
        }
        let initial = (jd - 2451545.0 + fraction) * 86400.0;
        let interval = days * 86400.0;
        let number = (((et - initial) / interval) as i64 + 1)
            .max(1)
            .min(n as i64) as usize;
        let record_begin = (jd - 2451545.0 + (number - 1) as f64 * days) * 86400.0;
        let radius = interval / 2.0;
        let midpoint = record_begin + fraction * 86400.0 + radius;
        let record = self.file.read_doubles(begin + (number - 1) * size, size)?;
        let terms = size / 3;
        let factor = scale / time;
        let mut angles = [0.0; 6];
        for axis in 0..3 {
            let cp: Vec<_> = record[axis * terms..(axis + 1) * terms - 1]
                .iter()
                .map(|v| v * factor)
                .collect();
            let (value, integral) = crate::spk::chebyshev_integral(&cp, midpoint, radius, et);
            angles[axis] = record[(axis + 1) * terms - 1] * scale + integral;
            angles[axis + 3] = value;
        }
        let tau = std::f64::consts::TAU;
        angles[2] -= tau * (angles[2] / tau).trunc();
        Ok(euler_state_313([
            angles[2], angles[1], angles[0], angles[5], angles[4], angles[3],
        ]))
    }
    fn type3(&self, begin: usize, end: usize, et: f64) -> Result<(Mat3, Mat3), GeoError> {
        let generic = crate::generic_segment::GenericSegment::new(&self.file, begin, end)?;
        let n = generic.constant(1)? as usize;
        if n == 0 || n > 21 {
            return Err(GeoError::BadDaf("invalid PCK3 coefficient count"));
        }
        let record = generic
            .packet_at(et)?
            .ok_or(GeoError::BadDaf("PCK3 has no packet for covered epoch"))?;
        if record.len() < 2 + 6 * n || record[1] <= 0.0 {
            return Err(GeoError::BadDaf("invalid PCK3 packet"));
        }
        let tau = (et - record[0]) / record[1];
        let mut angles = [0.0; 6];
        let rpd = std::f64::consts::PI / 180.0;
        for i in 0..6 {
            angles[i] = rpd * chebyshev_value(&record[2 + i * n..2 + (i + 1) * n], tau);
        }
        angles[0] = std::f64::consts::FRAC_PI_2 + angles[0];
        angles[1] = std::f64::consts::FRAC_PI_2 - angles[1];
        let rotation = rotate_matrix(rotate_matrix(rot3(angles[0]), angles[1], 0), angles[2], 2);
        let av = [-angles[3], -angles[4], -angles[5]];
        let derivative = std::array::from_fn(|i| {
            std::array::from_fn(|j| match i {
                0 => av[1] * rotation[2][j] - av[2] * rotation[1][j],
                1 => av[2] * rotation[0][j] - av[0] * rotation[2][j],
                _ => av[0] * rotation[1][j] - av[1] * rotation[0][j],
            })
        });
        Ok((rotation, derivative))
    }
}
fn chebyshev(cp: &[f64], s: f64) -> (f64, f64) {
    let s2 = s * 2.0;
    let (mut w0, mut w1, mut dw0, mut dw1) = (0.0, 0.0, 0.0, 0.0);
    for coefficient in cp[1..].iter().rev() {
        let w2 = w1;
        w1 = w0;
        w0 = coefficient + (s2 * w1 - w2);
        let dw2 = dw1;
        dw1 = dw0;
        dw0 = w1 * 2.0 + dw1 * s2 - dw2;
    }
    (cp[0] + (s * w0 - w1), w0 + s * dw0 - dw1)
}
/// Source EUL2XF specialization for PCKMAT's 3-1-3 Euler state.
fn euler_state_313(e: [f64; 6]) -> (Mat3, Mat3) {
    let rotation = rotate_matrix(rotate_matrix(rot3(e[2]), e[1], 0), e[0], 2);
    let ca = e[0].cos();
    let sa = e[0].sin();
    let u = e[1].cos();
    let v = e[1].sin();
    let solution = [[-1.0, 0.0, -u], [0.0, -ca, -sa * v], [0.0, sa, -ca * v]];
    let omega: [f64; 3] = std::array::from_fn(|i| {
        solution[i][0] * e[3] + solution[i][1] * e[4] + solution[i][2] * e[5]
    });
    let skew = [
        [0.0, -omega[0], omega[2]],
        [omega[0], 0.0, -omega[1]],
        [-omega[2], omega[1], 0.0],
    ];
    (rotation, mat_mul(&skew, &rotation))
}
#[cfg(test)]
// Keep each source primitive's independent regression adjacent to its implementation.
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    #[test]
    fn native_pckmat_type2_exact() {
        let moon = BinaryPck::open("target/kernels/moon_pa_de440_200625.bpc").unwrap();
        let synthetic = BinaryPck::open("tests/golden/binary_pck_type2.bpc").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/binary_pck_native.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let class = f[0].parse().unwrap();
            let source = if class == 31008 { &moon } else { &synthetic };
            let (reference, r, d) = source
                .state_rotation(class, f[1].parse().unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(reference, f[2].parse::<i32>().unwrap());
            let expected: Vec<f64> = f[3..].iter().map(|v| v.parse().unwrap()).collect();
            let actual = [r.concat(), d.concat()].concat();
            if actual != expected {
                errors.push(format!("{} {}:actual{actual:?}", f[0], f[1]));
            }
            count += 1;
        }
        assert_eq!(count, 114);
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
        assert!(moon.state_rotation(301, 0.0).unwrap().is_none());
        assert!(synthetic
            .state_rotation(999001, 86400.01)
            .unwrap()
            .is_none());
    }
}

fn chebyshev_value(cp: &[f64], s: f64) -> f64 {
    let (mut w0, mut w1) = (0.0, 0.0);
    let twice = s * 2.0;
    for c in cp[1..].iter().rev() {
        let w2 = w1;
        w1 = w0;
        w0 = c + (twice * w1 - w2);
    }
    s * w0 - w1 + cp[0]
}
