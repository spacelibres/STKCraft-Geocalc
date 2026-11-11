use stkcraft_geocalc::*;
#[test]
fn pointing_mixed_source_inertial_frames_exact() {
    let kernel = SpkKernel::open("tests/golden/spk_mixed_frames.bsp").unwrap();
    let frames = Frames::new();
    let context = PointingContext {
        kernel: &kernel,
        frames: &frames,
        pck: None,
        fov: None,
    };
    let (mut checked, mut different) = (0, 0);
    for line in include_str!("golden/spk_mixed_corrected.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let target = f[0].parse().unwrap();
        let observer = f[1].parse().unwrap();
        let epoch = f[2].parse().unwrap();
        let aberration = Aberration::parse(f[4]).unwrap();
        for velocity in [false, true] {
            let direction = if velocity {
                PointingDirection::Velocity {
                    target,
                    observer,
                    input_frame: f[3].into(),
                    aberration,
                    anti: false,
                }
            } else {
                PointingDirection::Position {
                    target,
                    observer,
                    aberration,
                    anti: false,
                }
            };
            let request = PointingRequest {
                epoch,
                direction,
                output_frame: f[3].into(),
                magnitude: PointingMagnitude::PreserveOriginal,
                representation: PointingRepresentation::Rectangular,
            };
            let actual = pointing_calculation(&context, &request).unwrap()[0]
                .columns
                .vector;
            let start = if velocity { 8 } else { 12 };
            for i in 0..3 {
                let expected: f64 = f[start + i].parse().unwrap();
                if actual[i].to_bits() != expected.to_bits() {
                    if different < 4 {
                        eprintln!("velocity={velocity} frame={} correction={} actual={:?} expected={expected:?}",f[3],f[4],actual[i]);
                    }
                    different += 1;
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 9072);
    assert_eq!(different, 0);
}

#[test]
fn pointing_named_and_rotating_source_frames_exact() {
    let kernel = SpkKernel::open_all(&[
        "target/kernels/de440s.bsp",
        "tests/golden/spk_mixed_frames.bsp",
    ])
    .unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::open("tests/golden/spk_named_mixed.tf").unwrap();
    let context = PointingContext {
        kernel: &kernel,
        frames: &frames,
        pck: Some(&pck),
        fov: None,
    };
    let (mut checked, mut different) = (0, 0);
    for line in include_str!("golden/spk_named_mixed.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let target = f[0].parse().unwrap();
        let observer = f[1].parse().unwrap();
        let epoch = f[2].parse().unwrap();
        let aberration = Aberration::parse(f[4]).unwrap();
        for velocity in [false, true] {
            let direction = if velocity {
                PointingDirection::Velocity {
                    target,
                    observer,
                    input_frame: f[3].into(),
                    aberration,
                    anti: false,
                }
            } else {
                PointingDirection::Position {
                    target,
                    observer,
                    aberration,
                    anti: false,
                }
            };
            let request = PointingRequest {
                epoch,
                direction,
                output_frame: f[3].into(),
                magnitude: PointingMagnitude::PreserveOriginal,
                representation: PointingRepresentation::Rectangular,
            };
            let actual = pointing_calculation(&context, &request).unwrap()[0]
                .columns
                .vector;
            let start = if velocity { 8 } else { 12 };
            for i in 0..3 {
                let expected: f64 = f[start + i].parse().unwrap();
                if actual[i].to_bits() != expected.to_bits() {
                    if different < 4 {
                        eprintln!("velocity={velocity} frame={} correction={} actual={:?} expected={expected:?}",f[3],f[4],actual[i]);
                    }
                    different += 1;
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 15552);
    assert_eq!(different, 0);
}
