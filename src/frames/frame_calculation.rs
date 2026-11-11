//! Corrected two-epoch WebGeocalc frame transformations.
use crate::{Aberration, FrameRotation, Frames, GeoError, Pck, SpkKernel};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameTimeLocation {
    Frame1,
    Frame2,
}
impl FrameTimeLocation {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "FRAME1" => Some(Self::Frame1),
            "FRAME2" => Some(Self::Frame2),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameTransformation {
    pub frame1_epoch: f64,
    pub frame2_epoch: f64,
    pub light_time: f64,
    pub transform: FrameRotation,
}
impl FrameTransformation {
    pub fn state_matrix(&self) -> [[f64; 6]; 6] {
        pack(self.transform)
    }
}
fn pack(value: FrameRotation) -> [[f64; 6]; 6] {
    let mut matrix = [[0.; 6]; 6];
    for i in 0..3 {
        for j in 0..3 {
            matrix[i][j] = value.rotation[i][j];
            matrix[i + 3][j] = value.rotation_dot[i][j];
            matrix[i + 3][j + 3] = value.rotation[i][j];
        }
    }
    matrix
}
#[allow(clippy::too_many_arguments)]
pub fn frame_transformation(
    kernel: &SpkKernel,
    frames: &Frames,
    pck: Option<&Pck>,
    one: &str,
    two: &str,
    et: f64,
    correction: Aberration,
    locus: FrameTimeLocation,
) -> Result<FrameTransformation, GeoError> {
    if !et.is_finite() {
        return Err(GeoError::BadSearch("frame epoch must be finite"));
    }
    let (light_time, _, stellar, transmission) = correction.flags();
    if stellar {
        return Err(GeoError::BadSearch(
            "stellar corrections are not defined for frame transformations",
        ));
    }
    let center1 = frames.center(one)?;
    let center2 = frames.center(two)?;
    if !light_time || center1 == center2 {
        return Ok(FrameTransformation {
            frame1_epoch: et,
            frame2_epoch: et,
            light_time: 0.,
            transform: frames.pxform_with_derivative(one, two, et, pck)?,
        });
    }
    let (target, observer) = match locus {
        FrameTimeLocation::Frame1 => (center2, center1),
        FrameTimeLocation::Frame2 => (center1, center2),
    };
    let state = kernel.state_corrected(target, observer, et, correction)?;
    let sign = if transmission { 1.0 } else { -1.0 };
    let shifted = et + sign * state.light_time;
    let factor = 1.0 + sign * state.light_time_rate;
    let (time1, time2) = match locus {
        FrameTimeLocation::Frame1 => (et, shifted),
        FrameTimeLocation::Frame2 => (shifted, et),
    };
    let mut first = pack(frames.pxform_with_derivative(one, "J2000", time1, pck)?);
    let mut second = pack(frames.pxform_with_derivative("J2000", two, time2, pck)?);
    let shifted_transform = match locus {
        FrameTimeLocation::Frame1 => &mut second,
        FrameTimeLocation::Frame2 => &mut first,
    };
    for row in shifted_transform.iter_mut().skip(3) {
        for item in row.iter_mut().take(3) {
            *item *= factor;
        }
    }
    let mut matrix = [[0.; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            for k in 0..6 {
                matrix[i][j] += second[i][k] * first[k][j];
            }
        }
    }
    let mut rotation = [[0.; 3]; 3];
    let mut rotation_dot = rotation;
    for i in 0..3 {
        for j in 0..3 {
            rotation[i][j] = matrix[i][j];
            rotation_dot[i][j] = matrix[i + 3][j];
        }
    }
    Ok(FrameTransformation {
        frame1_epoch: time1,
        frame2_epoch: time2,
        light_time: state.light_time,
        transform: FrameRotation {
            rotation,
            rotation_dot,
        },
    })
}
