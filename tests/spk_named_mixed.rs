use stkcraft_geocalc::{Aberration, FrameSource, Frames, Pck, SpkKernel};
#[test]
fn named_mixed_segment_frames_equal_cspice() {
    let k = SpkKernel::open_all(&[
        "target/kernels/de440s.bsp",
        "tests/golden/spk_mixed_frames.bsp",
    ])
    .unwrap();
    let p = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::open("tests/golden/spk_named_mixed.tf").unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/spk_named_mixed.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let t = f[0].parse().unwrap();
        let o = f[1].parse().unwrap();
        let et = f[2].parse().unwrap();
        let result = k.apparent_state_in(
            t,
            o,
            et,
            Aberration::parse(f[4]).unwrap(),
            &FrameSource::Named {
                frames: &frames,
                pck: &p,
                frame_name: f[3],
            },
        );
        let expected: Vec<f64> = f[5..12].iter().map(|v| v.parse().unwrap()).collect();
        match result {
            Ok(result) => {
                let actual = [
                    result.state.position[0],
                    result.state.position[1],
                    result.state.position[2],
                    result.state.velocity[0],
                    result.state.velocity[1],
                    result.state.velocity[2],
                    result.light_time,
                ];
                if actual.as_slice() != expected {
                    failures.push(format!("{}: {actual:?}", f[..5].join(",")));
                }
            }
            Err(e) => failures.push(format!("{}: {e}", f[..5].join(","))),
        }
        if let Some(body) = f[3].strip_prefix("IAU_").and_then(|name| match name {
            "MOON" => Some(301),
            "EARTH" => Some(399),
            _ => None,
        }) {
            match k.apparent_state_in(
                t,
                o,
                et,
                Aberration::parse(f[4]).unwrap(),
                &FrameSource::BodyFixed { pck: &p, body },
            ) {
                Ok(result) => {
                    let actual = [
                        result.state.position[0],
                        result.state.position[1],
                        result.state.position[2],
                        result.state.velocity[0],
                        result.state.velocity[1],
                        result.state.velocity[2],
                        result.light_time,
                    ];
                    if actual.as_slice() != expected {
                        failures.push(format!("bodyfixed {}: {actual:?}", f[..5].join(",")));
                    }
                }
                Err(e) => failures.push(format!("bodyfixed {}: {e}", f[..5].join(","))),
            }
        }
        let expected_p: Vec<f64> = f[12..].iter().map(|v| v.parse().unwrap()).collect();
        match k.position_corrected_in_frame(
            t,
            o,
            et,
            Aberration::parse(f[4]).unwrap(),
            &frames,
            Some(&p),
            f[3],
        ) {
            Ok((p, lt)) => {
                if [p[0], p[1], p[2], lt].as_slice() != expected_p {
                    failures.push(format!("position {}: {p:?} {lt}", f[..5].join(",")));
                }
            }
            Err(e) => failures.push(format!("position {}: {e}", f[..5].join(","))),
        }
        count += 1;
    }
    assert!(
        failures.is_empty(),
        "{}/{} failed:\n{}",
        failures.len(),
        count,
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
