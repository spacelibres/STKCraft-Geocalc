//! Text and binary planetary constants kernels: body radii, gravitational
//! parameters, IAU body orientation and native binary types 2, 3 and 20.
//!
//! The orientation model follows the IAU report as encoded in a NAIF
//! text PCK:
//!
//! ```text
//! alpha = ra0 + ra1*T + ra2*T^2 + sum(ra_i * sin(theta_i))
//! delta = de0 + de1*T + de2*T^2 + sum(de_i * cos(theta_i))
//! W     = w0  + w1*d + w2*d^2 + sum(w_i  * sin(theta_i))
//! theta_i = angle_i + rate_i*T
//! ```
//!
//! with `T` in Julian centuries and `d` in days past J2000. The matrix
//! mapping J2000 vectors into the body-fixed frame is
//! `[W]_3 [pi/2 - delta]_1 [pi/2 + alpha]_3`, using SPICE's rotation
//! matrices. This was checked against CSPICE `pxform` for the Moon and
//! Earth.
//!
//! NAIF PCK files are data, not code; provide one at run time.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::Path;

use crate::error::GeoError;
use crate::frame::{
    apply_assignments, extract_data_blocks, mat_mul, parse_f64, rot1, rot3, Mat3, Value,
};

/// Body constants and orientation records loaded from text and binary PCKs.
#[derive(Clone, Debug, Default)]
pub struct Pck {
    radii: HashMap<i32, [f64; 3]>,
    gm: HashMap<i32, f64>,
    orientation: HashMap<i32, Orientation>,
    positive_longitude: HashMap<i32, bool>,
    prime_meridian_rates: HashMap<i32, f64>,
    phase_angles: HashMap<i32, Vec<f64>>,
    phase_degree: HashMap<i32, i32>,
    binary: Vec<crate::binary_pck::BinaryPck>,
    pool: HashMap<String, Value>,
}

#[derive(Clone, Debug, Default)]
struct Orientation {
    has_ra: bool,
    has_dec: bool,
    has_pm: bool,
    pole_ra: [f64; 3],
    pole_dec: [f64; 3],
    prime_meridian: [f64; 3],
    nut_ra: Vec<f64>,
    nut_dec: Vec<f64>,
    nut_pm: Vec<f64>,
}

/// IAU orientation angles and their analytic rates.
#[derive(Clone, Copy, Debug)]
struct OrientationAngles {
    alpha: f64,
    delta: f64,
    w: f64,
    alpha_dot: f64,
    delta_dot: f64,
    w_dot: f64,
}

impl Pck {
    /// Opens a text or binary PCK file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GeoError> {
        let mut pck = Self::default();
        pck.load(path)?;
        Ok(pck)
    }

    /// Loads a text or binary PCK, preserving file and segment priority.
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<(), GeoError> {
        let bytes = std::fs::read(path)?;
        if bytes.starts_with(b"DAF/PCK") {
            self.binary
                .push(crate::binary_pck::BinaryPck::from_bytes(bytes)?);
            Ok(())
        } else {
            let text = String::from_utf8(bytes)
                .map_err(|_| GeoError::BadDaf("PCK text is not valid UTF-8"))?;
            self.furnsh(&text)
        }
    }

    /// A binary orientation is selected by PCK class ID, not center body ID.
    pub(crate) fn binary_state_rotation(
        &self,
        class_id: i32,
        et: f64,
    ) -> Result<Option<(i32, Mat3, Mat3)>, GeoError> {
        for kernel in self.binary.iter().rev() {
            if let Some(state) = kernel.state_rotation(class_id, et)? {
                return Ok(Some(state));
            }
        }
        Ok(None)
    }

    /// Parses a text PCK from memory.
    pub fn from_text(text: &str) -> Result<Self, GeoError> {
        let mut pck = Self::default();
        pck.furnsh(text)?;
        Ok(pck)
    }

    /// Merges the assignments of another text PCK into this one.
    pub fn furnsh(&mut self, text: &str) -> Result<(), GeoError> {
        let data = extract_data_blocks(text);
        let parse_result = apply_assignments(&data, &mut self.pool);
        // Native loading preserves assignments before an error and removes
        // the failed variable. Rebuild every derived text cache from that pool.
        self.radii.clear();
        self.gm.clear();
        self.orientation.clear();
        self.positive_longitude.clear();
        self.prime_meridian_rates.clear();
        self.phase_angles.clear();
        self.phase_degree.clear();
        let assignments = self.pool.clone();

        for (variable, value) in assignments {
            let Some(rest) = variable.strip_prefix("BODY") else {
                continue;
            };
            let Some((id_text, attribute)) = rest.split_once('_') else {
                continue;
            };
            let Ok(id) = id_text.parse::<i32>() else {
                continue;
            };

            match attribute {
                "RADII" => {
                    if let Some(radii) = first3(&value) {
                        self.radii.insert(id, radii);
                    }
                }
                "GM" => {
                    if let Some(item) = scalar(&value) {
                        self.gm.insert(id, parse_f64(item)?);
                    }
                }
                "POLE_RA" => {
                    if let Some(coefficients) = first3(&value) {
                        let model = self.orientation.entry(id).or_default();
                        model.pole_ra = coefficients;
                        model.has_ra = true;
                    }
                }
                "POLE_DEC" => {
                    if let Some(coefficients) = first3(&value) {
                        let model = self.orientation.entry(id).or_default();
                        model.pole_dec = coefficients;
                        model.has_dec = true;
                    }
                }
                "PM" => {
                    let coefficients = floats(&value).unwrap_or_default();
                    if coefficients.len() >= 2 {
                        self.prime_meridian_rates.insert(id, coefficients[1]);
                    } else {
                        self.prime_meridian_rates.remove(&id);
                    }
                    let model = self.orientation.entry(id).or_default();
                    model.has_pm = !coefficients.is_empty();
                    if model.has_pm {
                        model.prime_meridian = [0.0; 3];
                        for (i, value) in coefficients.iter().take(3).enumerate() {
                            model.prime_meridian[i] = *value;
                        }
                    }
                }
                "NUT_PREC_ANGLES" => {
                    if let Some(coefficients) = floats(&value) {
                        self.phase_angles.insert(id, coefficients);
                    }
                }
                "MAX_PHASE_DEGREE" => {
                    if let Some(item) = scalar(&value) {
                        self.phase_degree
                            .insert(id, parse_f64(item)?.round() as i32);
                    }
                }
                "NUT_PREC_RA" => {
                    if let Some(coefficients) = floats(&value) {
                        self.orientation.entry(id).or_default().nut_ra = coefficients;
                    }
                }
                "NUT_PREC_DEC" => {
                    if let Some(coefficients) = floats(&value) {
                        self.orientation.entry(id).or_default().nut_dec = coefficients;
                    }
                }
                "NUT_PREC_PM" => {
                    if let Some(coefficients) = floats(&value) {
                        self.orientation.entry(id).or_default().nut_pm = coefficients;
                    }
                }
                "PGR_POSITIVE_LON" => {
                    let text = value.as_string().ok_or(GeoError::BadSearch(
                        "PGR_POSITIVE_LON must be the string EAST or WEST",
                    ))?;
                    match text.trim().to_ascii_uppercase().as_str() {
                        "EAST" => {
                            self.positive_longitude.insert(id, true);
                        }
                        "WEST" => {
                            self.positive_longitude.insert(id, false);
                        }
                        _ => {
                            return Err(GeoError::BadSearch(
                                "PGR_POSITIVE_LON must be the string EAST or WEST",
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        parse_result
    }

    /// Opens and merges text and binary PCK files in load order.
    pub fn open_all(paths: &[impl AsRef<Path>]) -> Result<Self, GeoError> {
        let mut pck = Self::default();
        for path in paths {
            pck.load(path)?;
        }
        Ok(pck)
    }

    /// Triaxial radii `(a, b, c)` in kilometres.
    pub fn radii(&self, body: i32) -> Option<[f64; 3]> {
        self.radii.get(&body).copied()
    }

    /// Gravitational parameter in km^3/s^2.
    pub fn gm(&self, body: i32) -> Option<f64> {
        self.gm.get(&body).copied()
    }

    /// First-degree BODY*_PM coefficient, in degrees per day.
    pub fn prime_meridian_rate(&self, body: i32) -> Option<f64> {
        self.prime_meridian_rates.get(&body).copied()
    }

    /// Whether planetographic longitude increases to the east for `body`.
    ///
    /// Follows CSPICE `recpgr`: an explicit `BODY<id>_PGR_POSITIVE_LON`
    /// wins, then the Earth, Moon, and Sun are positive east, and otherwise
    /// the sign of the first-degree prime-meridian coefficient decides
    /// (retrograde spin is positive east). Returns `None` when the PCK has
    /// neither (Earth does not require PM).
    pub fn planetographic_east_positive(&self, body: i32) -> Option<bool> {
        if let Some(explicit) = self.positive_longitude.get(&body) {
            return Some(*explicit);
        }
        // PLNSNS exempts Earth before its PM lookup. RECPGR exempts
        // the Moon and Sun only after PLNSNS has found a numeric rate.
        if body == 399 {
            return Some(true);
        }
        let rate = self.prime_meridian_rate(body)?;
        Some(matches!(body, 10 | 301) || rate < 0.0)
    }

    /// Rotation mapping J2000 vectors into the body-fixed frame, when the
    /// PCK defines an IAU orientation model for `body`.
    ///
    /// Nutation phase angles use the source planetary-system key:
    /// the Moon uses id 3, and extended satellite codes use id / 10000.
    pub fn body_rotation(&self, body: i32, et: f64) -> Option<Mat3> {
        self.body_state_rotation(body, et).map(|(matrix, _)| matrix)
    }

    /// Rotation matrix and its time derivative for the IAU body-fixed
    /// frame of `body` at `et`, matching CSPICE `sxform`.
    ///
    /// Returns `(m, m_dot)` with
    ///
    /// ```text
    /// position_fixed = m * position_j2000
    /// velocity_fixed = m * velocity_j2000 + m_dot * position_j2000
    /// ```
    ///
    /// `m_dot` is analytic. The IAU model composes three rotations,
    /// `m = R3(W) R1(pi/2 - delta) R3(pi/2 + alpha)`, so
    ///
    /// ```text
    /// m_dot = W_dot R3 A3 R1 R3 - delta_dot R3 R1 A1 R3 + alpha_dot R3 R1 R3 A3
    /// ```
    ///
    /// using `dR1/dtheta = R1 A1` and `dR3/dtheta = R3 A3`.
    pub fn body_state_rotation(&self, body: i32, et: f64) -> Option<(Mat3, Mat3)> {
        self.validate_text_orientation(body).ok()?;
        let angles = self.orientation_angles(body, et)?;

        let right_ascension = angles.alpha.to_radians();
        let declination = angles.delta.to_radians();
        let raw_meridian = angles.w.to_radians();
        // f2c D_MOD used by TISBOD performs a rounded division and
        // multiply/subtract, not the language/library fmod operation.
        let prime_meridian = raw_meridian - (2.0 * PI) * (raw_meridian / (2.0 * PI)).trunc();

        let meridian = rot3(prime_meridian);
        let polar = rot1(PI * 0.5 - declination);
        let equinox = rot3(PI * 0.5 + right_ascension);
        let matrix = mat_mul(&meridian, &mat_mul(&polar, &equinox));

        // EUL2XF's 3-1-3 derivative is formed as (dR/dt R^T) R.
        // Keep its exact dot-product order, including zero terms.
        let cosine = prime_meridian.cos();
        let sine = prime_meridian.sin();
        let u = (PI * 0.5 - declination).cos();
        let v = (PI * 0.5 - declination).sin();
        let rates = [angles.w_dot, -angles.delta_dot, angles.alpha_dot];
        let solution = [
            [-1.0, 0.0, -u],
            [0.0, -cosine, -sine * v],
            [0.0, sine, -cosine * v],
        ];
        let mut omega = [0.0; 3];
        for row in 0..3 {
            omega[row] = solution[row][0] * rates[0]
                + solution[row][1] * rates[1]
                + solution[row][2] * rates[2];
        }
        let skew = [
            [0.0, -omega[0], omega[2]],
            [omega[0], 0.0, -omega[1]],
            [-omega[2], omega[1], 0.0],
        ];
        let derivative = mat_mul(&skew, &matrix);
        let (_, reference) = self.constants_settings(body).ok()?;
        if reference == 1 {
            Some((matrix, derivative))
        } else {
            let change = crate::frame::Frames::new()
                .pxform("J2000", &reference.to_string(), et)
                .ok()?;
            Some((mat_mul(&matrix, &change), mat_mul(&derivative, &change)))
        }
    }

    /// Checked text-PCK orientation, preserving source metadata failures.
    pub fn body_state_rotation_checked(
        &self,
        body: i32,
        et: f64,
    ) -> Result<(Mat3, Mat3), GeoError> {
        self.validate_text_orientation(body)?;
        self.body_state_rotation(body, et)
            .ok_or(GeoError::MissingPckData(body))
    }
    pub fn body_rotation_checked(&self, body: i32, et: f64) -> Result<Mat3, GeoError> {
        self.body_state_rotation_checked(body, et).map(|x| x.0)
    }
    fn constants_settings(&self, body: i32) -> Result<(f64, i32), GeoError> {
        let reference = phase_system(body);
        let value =
            |suffix: &str, competing: &'static str, default: f64| -> Result<f64, GeoError> {
                let a = self
                    .pool
                    .get(&format!("BODY{reference}_CONSTANTS_{suffix}"));
                let b = self.pool.get(&format!("BODY{reference}_CONSTS_{suffix}"));
                if a.is_some() && b.is_some() {
                    return Err(GeoError::BadSearch(competing));
                }
                match a.or(b) {
                    None => Ok(default),
                    Some(v) => floats(v)
                        .and_then(|x| x.first().copied())
                        .ok_or(GeoError::BadSearch("SPICE(TYPEMISMATCH)")),
                }
            };
        let epoch = value("JED_EPOCH", "SPICE(COMPETINGEPOCHSPEC)", 2451545.0)?;
        let frame = value("REF_FRAME", "SPICE(COMPETINGFRAMESPEC)", 1.0)?.round() as i32;
        if !(1..=21).contains(&frame) {
            return Err(GeoError::BadSearch("SPICE(IRFNOTREC)"));
        }
        Ok((epoch, frame))
    }
    pub(crate) fn validate_text_orientation(&self, body: i32) -> Result<(), GeoError> {
        if !self.pool.contains_key(&format!("BODY{body}_PM")) {
            return Err(GeoError::BadSearch("SPICE(FRAMEDATANOTFOUND)"));
        }
        self.constants_settings(body)?;
        for key in ["POLE_RA", "POLE_DEC", "PM"] {
            let value = self
                .pool
                .get(&format!("BODY{body}_{key}"))
                .ok_or(GeoError::BadSearch("SPICE(KERNELVARNOTFOUND)"))?;
            let values = floats(value).ok_or(GeoError::BadSearch("SPICE(TYPEMISMATCH)"))?;
            if values.len() > 3 {
                return Err(GeoError::BadSearch("SPICE(ARRAYTOOSMALL)"));
            }
        }
        let model = self
            .orientation
            .get(&body)
            .ok_or(GeoError::MissingPckData(body))?;
        if !model.has_ra || !model.has_dec || !model.has_pm {
            return Err(GeoError::MissingPckData(body));
        }
        let reference = phase_system(body);
        let nphase = if let Some(angles) = self.phase_angles.get(&reference) {
            let degree = self.phase_degree.get(&reference).copied().unwrap_or(1);
            if !(1..=3).contains(&degree) {
                return Err(GeoError::BadSearch("SPICE(DEGREEOUTOFRANGE)"));
            }
            if angles.len() > 800 {
                return Err(GeoError::BadSearch("SPICE(ARRAYTOOSMALL)"));
            }
            angles.len() / (degree as usize + 1)
        } else {
            0
        };
        for key in ["NUT_PREC_RA", "NUT_PREC_DEC", "NUT_PREC_PM"] {
            if let Some(value) = self.pool.get(&format!("BODY{body}_{key}")) {
                let values = floats(value).ok_or(GeoError::BadSearch("SPICE(TYPEMISMATCH)"))?;
                if values.len() > 200 {
                    return Err(GeoError::BadSearch("SPICE(ARRAYTOOSMALL)"));
                }
            }
        }
        let required = model
            .nut_ra
            .len()
            .max(model.nut_dec.len())
            .max(model.nut_pm.len());
        if required > 200 {
            return Err(GeoError::BadSearch("SPICE(ARRAYTOOSMALL)"));
        }
        if required > nphase {
            return Err(GeoError::BadSearch("SPICE(INSUFFICIENTANGLES)"));
        }
        Ok(())
    }

    /// The IAU orientation angles and their analytic time derivatives.
    ///
    /// `alpha`, `delta` and `w` are in degrees; the derivatives are in
    /// radians per second.
    fn orientation_angles(&self, body: i32, et: f64) -> Option<OrientationAngles> {
        let orientation = self.orientation.get(&body)?;
        if !orientation.has_ra || !orientation.has_dec || !orientation.has_pm {
            return None;
        }

        let (reference_epoch, _) = self.constants_settings(body).ok()?;
        let epoch = et - 86_400.0 * (reference_epoch - 2451545.0);
        let centuries = epoch / (86_400.0 * 36_525.0);
        let days = epoch / 86_400.0;
        let degrees = PI / 180.0;

        let mut alpha = polynomial(&orientation.pole_ra, centuries);
        let mut delta = polynomial(&orientation.pole_dec, centuries);
        let mut w = polynomial(&orientation.prime_meridian, days);

        // TISBOD accumulates all angle rates in degrees/second, then
        // converts once. Nutation is a separate dot product added once to
        // the polynomial, rather than a sequence of additions to it.
        let century_seconds = 86_400.0 * 36_525.0;
        let mut alpha_dot =
            (orientation.pole_ra[1] + centuries * 2.0 * orientation.pole_ra[2]) / century_seconds;
        let mut delta_dot =
            (orientation.pole_dec[1] + centuries * 2.0 * orientation.pole_dec[2]) / century_seconds;
        let mut w_dot =
            (orientation.prime_meridian[1] + days * 2.0 * orientation.prime_meridian[2]) / 86_400.0;
        let mut nutation = [0.0; 6];
        if let Some((angles, stride)) = self.nutation_phases(body) {
            for (index, phase) in angles.chunks_exact(stride).enumerate() {
                let (theta, theta_dot) = if stride == 2 {
                    (
                        (phase[0] + centuries * phase[1]) * degrees,
                        phase[1] / century_seconds * degrees,
                    )
                } else {
                    let mut angle = 0.0;
                    for (degree, coefficient) in phase.iter().enumerate() {
                        angle += centuries.powi(degree as i32) * coefficient;
                    }
                    let mut rate = if stride > 1 {
                        phase[1] / century_seconds
                    } else {
                        0.0
                    };
                    for (degree, coefficient) in phase.iter().enumerate().skip(2) {
                        rate += degree as f64 * centuries.powi(degree as i32 - 1) * coefficient
                            / century_seconds.powi(degree as i32 - 1);
                    }
                    (angle * degrees, rate * degrees)
                };
                let sine = theta.sin();
                let cosine = theta.cos();
                if let Some(coefficient) = orientation.nut_ra.get(index) {
                    nutation[0] += coefficient * sine;
                    nutation[3] += coefficient * (cosine * theta_dot);
                }
                if let Some(coefficient) = orientation.nut_dec.get(index) {
                    nutation[1] += coefficient * cosine;
                    nutation[4] += coefficient * (-sine * theta_dot);
                }
                if let Some(coefficient) = orientation.nut_pm.get(index) {
                    nutation[2] += coefficient * sine;
                    nutation[5] += coefficient * (cosine * theta_dot);
                }
            }
        }
        alpha += nutation[0];
        delta += nutation[1];
        w += nutation[2];
        alpha_dot = (alpha_dot + nutation[3]) * degrees;
        delta_dot = (delta_dot + nutation[4]) * degrees;
        w_dot = (w_dot + nutation[5]) * degrees;
        Some(OrientationAngles {
            alpha,
            delta,
            w,
            alpha_dot,
            delta_dot,
            w_dot,
        })
    }

    /// Nutation precession phase coefficients for `body`, with the number
    /// of coefficients per phase.
    ///
    /// NAIF stores phases under the system id selected by ZZBODBRY:
    /// traditional planet/satellite codes use id / 100, extended satellite
    /// codes use id / 10000, and other bodies use their own id.
    /// `BODY<id>_MAX_PHASE_DEGREE` selects a quadratic phase polynomial;
    /// otherwise the coefficients are `(constant, rate)` pairs.
    fn nutation_phases(&self, body: i32) -> Option<(&[f64], usize)> {
        let id = phase_system(body);
        let angles = self.phase_angles.get(&id)?;
        let stride = 1 + self.phase_degree.get(&id).copied().unwrap_or(1) as usize;
        Some((angles.as_slice(), stride))
    }
}

fn phase_system(body: i32) -> i32 {
    if (100..=999).contains(&body) {
        body / 100
    } else if (10000..=99999).contains(&body) {
        body / 10000
    } else {
        body
    }
}

fn scalar(value: &Value) -> Option<&str> {
    value.as_number()
}

fn floats(value: &Value) -> Option<Vec<f64>> {
    value
        .as_list()?
        .iter()
        .map(|item| parse_f64(item).ok())
        .collect()
}

/// The first three numbers, padding missing higher-order terms with zero.
///
/// Keplerian polynomial elements may legally omit the trailing terms:
/// `mars_iau2000_v1.tpc` writes `BODY499_PM = ( 176.630 350.89198226 )`,
/// and SPICE treats the absent quadratic coefficient as zero rather than
/// keeping the value from a lower-priority kernel.
fn first3(value: &Value) -> Option<[f64; 3]> {
    let items = floats(value)?;
    if items.is_empty() {
        return None;
    }
    let mut coefficients = [0.0f64; 3];
    for (index, item) in items.iter().take(3).enumerate() {
        coefficients[index] = *item;
    }
    Some(coefficients)
}

fn polynomial(coefficients: &[f64; 3], x: f64) -> f64 {
    coefficients[0] + x * (coefficients[1] + x * coefficients[2])
}

#[cfg(test)]
mod tests {
    use super::Pck;

    #[test]
    fn parses_radii_and_gm() {
        let text = r#"
\begindata
BODY301_RADII = ( 1737.4, 1737.4, 1737.4 )
BODY301_GM    = 4902.800066
BODY399_RADII = ( 6378.1366, 6378.1366, 6356.7519 )
\begintext
"#;
        let pck = Pck::from_text(text).unwrap();
        assert_eq!(pck.radii(301), Some([1737.4, 1737.4, 1737.4]));
        assert_eq!(pck.radii(399), Some([6378.1366, 6378.1366, 6356.7519]));
        assert!(pck.radii(499).is_none());
        assert!((pck.gm(301).unwrap() - 4902.800066).abs() < 1e-12);
    }

    #[test]
    fn parses_real_pck() {
        let Ok(text) = std::fs::read_to_string("target/kernels/pck00011.tpc") else {
            return;
        };
        let pck = Pck::from_text(&text).unwrap();
        assert_eq!(pck.radii(301), Some([1737.4, 1737.4, 1737.4]));
        assert_eq!(pck.radii(399), Some([6378.1366, 6378.1366, 6356.7519]));
    }

    #[test]
    fn moon_orientation_matches_cspice() {
        let Ok(text) = std::fs::read_to_string("target/kernels/pck00011.tpc") else {
            return;
        };
        let pck = Pck::from_text(&text).unwrap();
        let matrix = pck.body_rotation(301, 0.0).expect("Moon orientation");

        // CSPICE pxform('J2000', 'IAU_MOON', 0.0).
        let expected = [
            [0.7842270520919169, 0.5578471124601639, 0.2716514860755947],
            [-0.6200619152508559, 0.7205566654668131, 0.31035675134719964],
            [
                -0.022608671404182493,
                -0.4118309009426129,
                0.9109797785934293,
            ],
        ];
        for row in 0..3 {
            for column in 0..3 {
                assert!(
                    (matrix[row][column] - expected[row][column]).abs() < 1e-12,
                    "m[{row}][{column}] = {} expected {}",
                    matrix[row][column],
                    expected[row][column]
                );
            }
        }
    }
}

#[cfg(test)]
mod polynomial_lengths {
    use super::*;
    #[test]
    fn native_tisbod_short_orientation_polynomials() {
        let values = [[30., -0.1, 0.001], [70., 0.2, -0.003], [10., 100., 0.0002]];
        let mut count = 0;
        for line in include_str!("../../tests/golden/pck_polynomial_lengths.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let mut text = String::from("\\begindata\n");
            for (i, name) in ["POLE_RA", "POLE_DEC", "PM"].iter().enumerate() {
                let n: usize = f[i].parse().unwrap();
                text += &format!(
                    "BODY299_{name}=({})\n",
                    values[i][..n]
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            let pck = Pck::from_text(&text).unwrap();
            let (r, d) = pck
                .body_state_rotation(299, f[3].parse().unwrap())
                .expect(line);
            let actual = [r.concat(), d.concat()].concat();
            let expected: Vec<f64> = f[4..].iter().map(|v| v.parse().unwrap()).collect();
            assert_eq!(actual, expected, "{line}");
            count += 1;
        }
        assert_eq!(count, 81);
    }
}
#[cfg(test)]
mod phase_metadata {
    use super::*;
    #[test]
    fn native_tisbod_phase_cardinality_and_keys() {
        let mut failures = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/pck_phase_metadata.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let body: i32 = f[0].parse().unwrap();
            let reference: i32 = f[1].parse().unwrap();
            let degree: usize = f[2].parse().unwrap();
            let pc: usize = f[3].parse().unwrap();
            let nc: usize = f[4].parse().unwrap();
            let mut text=format!("\\begindata\nBODY{body}_POLE_RA=(30. -0.1 0.001)\nBODY{body}_POLE_DEC=(70. 0.2 -0.003)\nBODY{body}_PM=(10. 100. 0.0002)\n");
            if pc > 0 {
                text += &format!(
                    "BODY{reference}_NUT_PREC_ANGLES=({})\n",
                    (0..pc * (degree + 1))
                        .map(|i| (17. + i as f64 * 3.1).to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            if reference != body {
                text += &format!("BODY{body}_NUT_PREC_ANGLES=(80. 2. 160. 3. 240. 4. 320. 5.)\n");
            }
            text += &format!("BODY{reference}_MAX_PHASE_DEGREE=({degree})\n");
            for (i, key) in ["RA", "DEC", "PM"].iter().enumerate() {
                if nc > 0 {
                    text += &format!(
                        "BODY{body}_NUT_PREC_{key}=({})\n",
                        (0..nc)
                            .map(|j| {
                                let sign = if (j + i) % 2 == 0 { 1.0 } else { -1.0 };
                                (sign * (i + 1) as f64 * 0.01 * (j + 1) as f64).to_string()
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                }
            }
            let pck = Pck::from_text(&text).unwrap();
            let actual = pck.body_state_rotation(body, 123456789.);
            let checked = pck.body_state_rotation_checked(body, 123456789.);
            if f[5] == "OK" {
                let expected: Vec<f64> = f[6..].iter().map(|v| v.parse().unwrap()).collect();
                if actual
                    .map(|(r, d)| [r.concat(), d.concat()].concat())
                    .as_ref()
                    != Some(&expected)
                {
                    failures.push(format!("{line}: {actual:?}"));
                }
            } else if actual.is_some()
                || !matches!(&checked,Err(GeoError::BadSearch(message)) if *message==f[5])
            {
                failures.push(format!(
                    "{line}: unexpected orientation or error{checked:?}"
                ));
            }
            count += 1;
        }
        assert_eq!(count, 240);
        assert!(
            failures.is_empty(),
            "{} mismatches/{count}: {:?}",
            failures.len(),
            failures.iter().take(4).collect::<Vec<_>>()
        );
    }
}

#[cfg(test)]
mod constants_metadata {
    use super::*;
    #[test]
    fn native_text_pck_constants_frame_and_epoch() {
        let mut failures = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/pck_constants.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let body: i32 = f[0].parse().unwrap();
            let reference: i32 = f[1].parse().unwrap();
            let text=format!("\\begindata\nBODY{body}_POLE_RA=(30. -0.1 0.001)\nBODY{body}_POLE_DEC=(70. 0.2 -0.003)\nBODY{body}_PM=(10. 100. 0.0002)\nBODY{reference}_{}_JED_EPOCH=({})\nBODY{reference}_{}_REF_FRAME=({})\n",f[4],f[2],f[4],f[3]);
            let pck = Pck::from_text(&text).unwrap();
            let actual = pck
                .body_state_rotation_checked(body, f[5].parse().unwrap())
                .map(|(r, d)| [r.concat(), d.concat()].concat());
            let expected: Vec<f64> = f[7..].iter().map(|v| v.parse().unwrap()).collect();
            if actual.as_ref().ok() != Some(&expected) {
                failures.push(format!("{line}: {actual:?}"));
            }
            count += 1;
        }
        assert_eq!(count, 216);
        assert!(
            failures.is_empty(),
            "{} mismatches/{count}: {:?}",
            failures.len(),
            failures.iter().take(2).collect::<Vec<_>>()
        );
    }
}
#[cfg(test)]
mod mission_overlay {
    use super::*;
    #[test]
    fn native_enceladus_incompatible_pck_overlay_rejected() {
        let pck = Pck::open_all(&[
            "target/kernels/cassini/pck00010.tpc",
            "target/kernels/cassini/enceladus_ssd_250728_v2.tpc",
            "target/kernels/pck00011.tpc",
        ])
        .unwrap();
        assert!(matches!(
            pck.body_state_rotation_checked(602, 0.0),
            Err(GeoError::BadSearch("SPICE(INSUFFICIENTANGLES)"))
        ));
        assert!(pck.body_rotation(602, 0.0).is_none());
    }
}
#[cfg(test)]
mod metadata_errors {
    use super::*;
    #[test]
    fn native_tisbod_metadata_errors_and_scalar_spelling() {
        let mut count = 0;
        let mut failures = Vec::new();
        for line in include_str!("../../tests/golden/pck_metadata_errors.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let text = f[2].replace("\\n", "\n");
            let pck = Pck::from_text(&text).unwrap();
            let result = pck.body_state_rotation_checked(2000001, 123456789.);
            if f[1] == "OK" {
                let expected: Vec<f64> = f[3..].iter().map(|v| v.parse().unwrap()).collect();
                if result
                    .as_ref()
                    .map(|(r, d)| [r.concat(), d.concat()].concat())
                    .ok()
                    != Some(expected)
                {
                    failures.push(format!("{}: {result:?}", f[0]));
                }
            } else if !matches!(&result,Err(GeoError::BadSearch(message)) if *message==f[1]) {
                failures.push(format!("{} expected{}: {result:?}", f[0], f[1]));
            }
            count += 1;
        }
        assert_eq!(count, 21);
        assert!(failures.is_empty(), "{failures:?}");
    }
}
