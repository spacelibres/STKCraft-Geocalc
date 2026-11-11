use stkcraft_geocalc::{pointing::*, Aberration, Frames, Pck, RayDirection, SpkKernel};
#[test]
fn native_pointing_dynamic_direction_rotations_exact() {
    let kernel = SpkKernel::open_all(&[
        "target/kernels/de440s.bsp",
        "tests/golden/spk_dynamic_sources.bsp",
    ])
    .unwrap();
    let mut frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    for file in [
        "tests/golden/dynamic_two.tf",
        "tests/golden/dynamic_of_date.tf",
    ] {
        frames
            .furnsh(&std::fs::read_to_string(file).unwrap())
            .unwrap();
    }
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let context = PointingContext {
        kernel: &kernel,
        frames: &frames,
        pck: Some(&pck),
        fov: None,
    };
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/pointing_dynamic.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let direction = if f[0] == "VECTOR" {
            PointingDirection::Vector {
                direction: RayDirection::VectorInFrame {
                    frame: f[1].into(),
                    vector: [123.456789, -987.654321, 17.013],
                    anti: false,
                    stellar_aberration: false,
                },
                observer: f[7].parse().unwrap(),
                aberration: PointingVectorCorrection::parse(f[8]).unwrap(),
                anti: false,
            }
        } else {
            PointingDirection::Velocity {
                target: 91002,
                observer: 399,
                input_frame: f[1].into(),
                aberration: Aberration::None,
                anti: false,
            }
        };
        let request = PointingRequest {
            epoch: f[3].parse().unwrap(),
            direction,
            output_frame: f[2].into(),
            magnitude: PointingMagnitude::PreserveOriginal,
            representation: PointingRepresentation::Rectangular,
        };
        let result = pointing_calculation(&context, &request).map(|r| r[0].columns.vector);
        let expected: [f64; 3] = std::array::from_fn(|i| f[4 + i].parse().unwrap());
        if result.as_ref().ok() != Some(&expected) {
            failures.push(format!("{line}:actual{result:?}"));
        }
        count += 1;
    }
    assert_eq!(count, 360);
    assert!(
        failures.is_empty(),
        "{}/360 mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn native_ray_dynamic_direction_rotations_exact() {
    let kernel = SpkKernel::open_all(&[
        "target/kernels/de440s.bsp",
        "tests/golden/spk_dynamic_sources.bsp",
    ])
    .unwrap();
    let mut frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    for file in [
        "tests/golden/dynamic_two.tf",
        "tests/golden/dynamic_of_date.tf",
    ] {
        frames
            .furnsh(&std::fs::read_to_string(file).unwrap())
            .unwrap();
    }
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/pointing_dynamic.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        if f[0] != "VECTOR" || f[2] != "J2000" || (f[8] == "XS" && f[1] != "J2000") {
            continue;
        }
        let direction = RayDirection::VectorInFrame {
            frame: f[1].into(),
            vector: [123.456789, -987.654321, 17.013],
            anti: false,
            stellar_aberration: f[8] != "NONE",
        };
        let correction = if f[8] == "XS" {
            Aberration::TransmissionLightTime
        } else {
            Aberration::None
        };
        let result = stkcraft_geocalc::resolve_ray_directions_with_pck(
            &kernel,
            None,
            &frames,
            &pck,
            &direction,
            f[7].parse().unwrap(),
            f[3].parse().unwrap(),
            correction,
        )
        .map(|r| r[0]);
        let expected: [f64; 3] = std::array::from_fn(|i| f[4 + i].parse().unwrap());
        if result.as_ref().ok() != Some(&expected) {
            failures.push(format!("{line}:actual{result:?}"));
        }
        count += 1;
    }
    assert_eq!(count, 48);
    assert!(
        failures.is_empty(),
        "{}/48 mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
