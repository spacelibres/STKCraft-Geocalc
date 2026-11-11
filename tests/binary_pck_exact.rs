use stkcraft_geocalc::{Frames, Pck};
#[test]
fn binary_pck_moon_and_synthetic_frames_exact() {
    let pck = Pck::open_all(&[
        "target/kernels/pck00011.tpc",
        "target/kernels/moon_pa_de440_200625.bpc",
        "tests/golden/binary_pck_type2.bpc",
    ])
    .unwrap();
    let mut frames = Frames::open("target/kernels/moon_de440_250416.tf").unwrap();
    frames
        .furnsh(&std::fs::read_to_string("tests/golden/binary_pck_type2.tf").unwrap())
        .unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/binary_pck_frames.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let expected: Vec<f64> = f[3..].iter().map(|v| v.parse().unwrap()).collect();
        let result = frames
            .pxform_with_derivative(f[0], f[1], f[2].parse().unwrap(), Some(&pck))
            .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat());
        match result {
            Ok(x) if x == expected => {}
            other => errors.push(format!("{} {} {}:actual{other:?}", f[0], f[1], f[2])),
        };
        count += 1;
    }
    assert_eq!(count, 782);
    assert!(
        errors.is_empty(),
        "{}mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn native_cidfrm_body_frame_associations() {
    let mut frames = Frames::open("target/kernels/moon_de440_250416.tf").unwrap();
    for line in include_str!("golden/body_frame_defaults.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        assert_eq!(frames.body_frame(f[0].parse().unwrap()).unwrap(), f[3]);
    }
    frames
        .furnsh(&std::fs::read_to_string("target/kernels/moon_assoc_me.tf").unwrap())
        .unwrap();
    assert_eq!(frames.body_frame(301).unwrap(), "MOON_ME");
    frames
        .furnsh("\\begindata\nOBJECT_301_FRAME = 'MOON_PA'\n")
        .unwrap();
    assert_eq!(frames.body_frame(301).unwrap(), "MOON_PA");
    frames
        .furnsh("\\begindata\nOBJECT_301_FRAME = 31001\n")
        .unwrap();
    assert_eq!(frames.body_frame(301).unwrap(), "MOON_ME");
}

#[test]
fn binary_pck_file_priority_and_coverage_exact() {
    let first = "tests/golden/binary_pck_type2.bpc";
    let second = "tests/golden/binary_pck_layer.bpc";
    let frames = Frames::open("tests/golden/binary_pck_type2.tf").unwrap();
    let ab = Pck::open_all(&[first, second]).unwrap();
    let ba = Pck::open_all(&[second, first]).unwrap();
    let mut count = 0;
    for line in include_str!("golden/binary_pck_layers.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let pck = if f[0] == "ab" { &ab } else { &ba };
        let xf = frames
            .pxform_with_derivative("J2000", "PCK_TEST", f[1].parse().unwrap(), Some(pck))
            .unwrap();
        let actual = [xf.rotation.concat(), xf.rotation_dot.concat()].concat();
        let expected: Vec<f64> = f[2..].iter().map(|v| v.parse().unwrap()).collect();
        assert_eq!(actual, expected, "{line}");
        count += 1;
    }
    assert_eq!(count, 18);
    assert!(frames
        .pxform_with_derivative("J2000", "PCK_TEST", 86400.01, Some(&ab))
        .is_err());
    let only_text = Pck::open("target/kernels/pck00011.tpc").unwrap();
    assert!(frames
        .pxform_with_derivative("J2000", "PCK_TEST", 0.0, Some(&only_text))
        .is_err());
}

#[test]
fn binary_pck_big_endian_exact() {
    let pck = Pck::open("tests/golden/binary_pck_type2_big.bpc").unwrap();
    let frames = Frames::open("tests/golden/binary_pck_type2.tf").unwrap();
    let mut count = 0;
    for line in include_str!("golden/binary_pck_big.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let xf = frames
            .pxform_with_derivative("J2000", "PCK_TEST", f[0].parse().unwrap(), Some(&pck))
            .unwrap();
        let actual = [xf.rotation.concat(), xf.rotation_dot.concat()].concat();
        let expected: Vec<f64> = f[1..].iter().map(|v| v.parse().unwrap()).collect();
        assert_eq!(actual, expected, "{line}");
        count += 1;
    }
    assert_eq!(count, 7);
}

#[test]
fn binary_pck_named_apparent_states_exact() {
    use stkcraft_geocalc::{Aberration, FrameSource, SpkKernel};
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open_all(&[
        "target/kernels/pck00011.tpc",
        "target/kernels/moon_pa_de440_200625.bpc",
        "tests/golden/binary_pck_type2.bpc",
    ])
    .unwrap();
    let mut frames = Frames::open("target/kernels/moon_de440_250416.tf").unwrap();
    frames
        .furnsh(&std::fs::read_to_string("tests/golden/binary_pck_type2.tf").unwrap())
        .unwrap();
    let mut count = 0;
    let mut errors = Vec::new();
    for line in include_str!("golden/binary_pck_apparent.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let frame = FrameSource::Named {
            frames: &frames,
            pck: &pck,
            frame_name: f[2],
        };
        let state = kernel
            .apparent_state_in(
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                f[3].parse().unwrap(),
                Aberration::parse(f[4]).unwrap(),
                &frame,
            )
            .unwrap();
        let actual = [
            state.state.position.to_vec(),
            state.state.velocity.to_vec(),
            vec![state.light_time],
        ]
        .concat();
        let expected: Vec<f64> = f[5..].iter().map(|v| v.parse().unwrap()).collect();
        if actual != expected {
            errors.push(format!("{line}:actual{actual:?}"));
        }
        count += 1;
    }
    assert_eq!(count, 432);
    assert!(
        errors.is_empty(),
        "{}mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn native_pck_3_20_records_and_frames_exact() {
    let pck = stkcraft_geocalc::Pck::open("tests/golden/binary_pck_3_20.bpc").unwrap();
    let frames = stkcraft_geocalc::Frames::open("tests/golden/binary_pck_3_20.tf").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/binary_pck_3_20.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let frame = format!("PCK_MODEL_{}", f[0]);
        let expected: Vec<f64> = f[3..].iter().map(|v| v.parse().unwrap()).collect();
        let actual = frames
            .pxform_with_derivative(f[2], &frame, f[1].parse().unwrap(), Some(&pck))
            .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat());
        match actual {
            Ok(x) if x == expected => {}
            other => errors.push(format!("{} {} {}:actual{other:?}", f[0], f[1], f[2])),
        };
        count += 1;
    }
    assert_eq!(count, 6408);
    assert!(
        errors.is_empty(),
        "{}mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
