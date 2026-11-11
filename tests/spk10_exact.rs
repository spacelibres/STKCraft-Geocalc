use stkcraft_geocalc::{Aberration, FrameSource, Frames, Pck, SpkKernel};
fn kernel() -> SpkKernel {
    let mut k =
        SpkKernel::open_all(&["tests/golden/spk10.bsp", "target/kernels/de440s.bsp"]).unwrap();
    k.attach_lsk(
        stkcraft_time::lsk::parse_lsk(
            &std::fs::read_to_string("target/kernels/naif0012.tls").unwrap(),
        )
        .unwrap(),
    );
    k
}
#[test]
fn spk10_sgp4_j2000_states_equal_cspice() {
    let k = kernel();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/spk10.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let expected: Vec<f64> = f[3..].iter().map(|v| v.parse().unwrap()).collect();
        match k.state_inertial(
            f[0].parse().unwrap(),
            399,
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
        ) {
            Ok(s) => {
                let actual = [s.position, s.velocity].concat();
                if actual != expected {
                    failures.push(format!("{}: {actual:?} != {expected:?}", f[..3].join(",")));
                }
            }
            Err(e) => failures.push(format!("{}: {e}", f[..3].join(","))),
        }
        count += 1;
    }
    assert_eq!(count, 3045);
    assert!(
        failures.is_empty(),
        "{}/{} failed:\n{}",
        failures.len(),
        count,
        failures
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn spk10_corrected_state_position_pairs_equal_cspice() {
    let k = kernel();
    let frames = Frames::new();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/spk10_corrected.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let t = f[0].parse().unwrap();
        let o = f[1].parse().unwrap();
        let et = f[2].parse().unwrap();
        let correction = Aberration::parse(f[4]).unwrap();
        let s = k
            .apparent_state_in(
                t,
                o,
                et,
                correction,
                &FrameSource::Named {
                    frames: &frames,
                    pck: &pck,
                    frame_name: f[3],
                },
            )
            .unwrap();
        let (p, lt) = k
            .position_corrected_in_frame(t, o, et, correction, &frames, Some(&pck), f[3])
            .unwrap();
        let actual = [
            s.state.position.to_vec(),
            s.state.velocity.to_vec(),
            vec![s.light_time],
            p.to_vec(),
            vec![lt],
        ]
        .concat();
        let expected: Vec<f64> = f[5..].iter().map(|v| v.parse().unwrap()).collect();
        if actual != expected {
            failures.push(format!("{}: {actual:?} != {expected:?}", f[..5].join(",")));
        }
        count += 1;
    }
    assert_eq!(count, 270);
    assert!(
        failures.is_empty(),
        "{}/{} failed:\n{}",
        failures.len(),
        count,
        failures
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
