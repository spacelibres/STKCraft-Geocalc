//! N0067 SPKAPO position-only light-time and stellar-aberration pipeline.
use crate::{
    error::GeoError,
    frame::Mat3,
    spk::{Aberration, SpkKernel, CLIGHT},
};

pub(crate) fn corrected_position(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    et: f64,
    correction: Aberration,
) -> Result<([f64; 3], f64), GeoError> {
    corrected_position_inertial(
        kernel,
        target,
        observer,
        et,
        correction,
        &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    )
}

pub(crate) fn corrected_position_inertial(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    et: f64,
    correction: Aberration,
    matrix: &Mat3,
) -> Result<([f64; 3], f64), GeoError> {
    corrected_position_with_geometry(
        target,
        observer,
        et,
        correction,
        |t, o, epoch| kernel.geometric_in_matrix(t, o, epoch, matrix),
        |t, o, epoch| {
            kernel
                .geometric_in_matrix(t, o, epoch, matrix)
                .map(|state| state.position)
        },
    )
}

fn corrected_position_with_geometry<S, P>(
    target: i32,
    observer: i32,
    et: f64,
    correction: Aberration,
    state_geometry: S,
    position_geometry: P,
) -> Result<([f64; 3], f64), GeoError>
where
    S: Fn(i32, i32, f64) -> Result<crate::spk::State, GeoError>,
    P: Fn(i32, i32, f64) -> Result<[f64; 3], GeoError>,
{
    let (light_time, converged, stellar, transmission) = correction.flags();
    if !light_time {
        let position = position_geometry(target, observer, et)?;
        return Ok((position, crate::surface::norm(position) / CLIGHT));
    }
    let observer_state = state_geometry(observer, 0, et)?;
    let observer_position = observer_state.position;
    let observer_velocity = observer_state.velocity;
    let target_position = position_geometry(target, 0, et)?;
    let mut relative = std::array::from_fn(|i| target_position[i] - observer_position[i]);
    let mut lt = crate::surface::norm(relative) / CLIGHT;
    // Unlike SPKLTC's convergence loop, SPKAPO always executes three
    // position iterations for CN/XCN and one for LT/XLT.
    let sign = if transmission { 1.0 } else { -1.0 };
    for _ in 0..if converged { 3 } else { 1 } {
        let position = position_geometry(target, 0, et + sign * lt)?;
        relative = std::array::from_fn(|i| position[i] - observer_position[i]);
        lt = crate::surface::norm(relative) / CLIGHT;
    }
    if stellar {
        let velocity = if transmission {
            observer_velocity.map(|x| -x)
        } else {
            observer_velocity
        };
        relative = crate::surface::stellar_aberration_position(relative, velocity);
    }
    Ok((relative, lt))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_cspice_position_only_inertial_frames() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let frames = crate::frame::Frames::new();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/gf_position_only.csv")
            .lines()
            .skip(2)
        {
            let f: Vec<_> = line.split(',').collect();
            let et = f[2].parse().unwrap();
            let matrix = frames.pxform("J2000", f[4], et).unwrap();
            let (p, lt) = corrected_position_inertial(
                &kernel,
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                et,
                Aberration::parse(f[3]).unwrap(),
                &matrix,
            )
            .unwrap();
            let expected = [
                f[5].parse::<f64>().unwrap(),
                f[6].parse().unwrap(),
                f[7].parse().unwrap(),
                f[8].parse().unwrap(),
            ];
            let actual = [p[0], p[1], p[2], lt];
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
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// SPKEZP's requested-frame position path, including frame-center light time.
#[allow(clippy::too_many_arguments)]
pub(crate) fn corrected_position_in_frame(
    kernel: &SpkKernel,
    target: i32,
    observer: i32,
    et: f64,
    correction: Aberration,
    frames: &crate::frame::Frames,
    frame: &str,
    pck: Option<&crate::pck::Pck>,
) -> Result<([f64; 3], f64), GeoError> {
    let rotate = |matrix: Mat3, p: [f64; 3]| {
        std::array::from_fn(|i| matrix[i][0] * p[0] + matrix[i][1] * p[1] + matrix[i][2] * p[2])
    };
    if frames.is_inertial(frame)? {
        return corrected_position_with_geometry(
            target,
            observer,
            et,
            correction,
            |t, o, epoch| kernel.state_in_frame(t, o, epoch, frames, pck, frame),
            |t, o, epoch| kernel.position_in_frame(t, o, epoch, frames, pck, frame),
        );
    }
    if correction == Aberration::None {
        let position = kernel.position_in_frame(target, observer, et, frames, pck, frame)?;
        return Ok((position, crate::surface::norm(position) / CLIGHT));
    }
    let corrected = |t, o| {
        corrected_position_with_geometry(
            t,
            o,
            et,
            correction,
            |target, observer, epoch| {
                kernel.state_in_frame(target, observer, epoch, frames, pck, "J2000")
            },
            |target, observer, epoch| {
                kernel.position_in_frame(target, observer, epoch, frames, pck, "J2000")
            },
        )
    };
    let (position, light_time) = corrected(target, observer)?;
    let center = frames.center(frame)?;
    let ltcenter = if center == observer {
        0.0
    } else if center == target {
        light_time
    } else {
        corrected(center, observer)?.1
    };
    let transmission = correction.flags().3;
    let epoch = if transmission {
        et - (-ltcenter)
    } else {
        et - ltcenter
    };
    let r = frames.pxform_in("J2000", frame, epoch, pck)?;
    Ok((rotate(r, position), light_time))
}
#[cfg(test)]
mod dynamic_tests {
    use super::*;
    #[test]
    fn native_spkezp_dynamic_input_output_frames_exact() {
        let kernel = SpkKernel::open_all(&[
            "target/kernels/de440s.bsp",
            "tests/golden/spk_mixed_frames.bsp",
            "tests/golden/spk_dynamic_sources.bsp",
        ])
        .unwrap();
        let mut frames = crate::Frames::open("tests/golden/dynamic_euler.tf").unwrap();
        for path in [
            "tests/golden/dynamic_two.tf",
            "tests/golden/dynamic_of_date.tf",
        ] {
            frames
                .furnsh(&std::fs::read_to_string(path).unwrap())
                .unwrap();
        }
        let pck = crate::Pck::open("target/kernels/pck00011.tpc").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/spkapo_dynamic.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let actual = corrected_position_in_frame(
                &kernel,
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
                Aberration::parse(f[4]).unwrap(),
                &frames,
                f[3],
                Some(&pck),
            )
            .map(|(p, lt)| [p[0], p[1], p[2], lt]);
            let expected: [f64; 4] = std::array::from_fn(|i| f[5 + i].parse().unwrap());
            if actual.as_ref().ok() != Some(&expected) {
                errors.push(format!(
                    "{} {} {} {} {}: {actual:?} expected{expected:?}",
                    f[0], f[1], f[2], f[3], f[4]
                ));
            }
            count += 1;
        }
        assert_eq!(count, 1134);
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
