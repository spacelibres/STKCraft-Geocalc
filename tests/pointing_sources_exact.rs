use stkcraft_geocalc::pointing::{
    pointing_calculation, PointingContext, PointingDirection, PointingMagnitude,
    PointingRepresentation, PointingRequest,
};
use stkcraft_geocalc::{Aberration, Frames, Pck, SpkKernel};

#[test]
fn sources_match_independent_cspice_without_tolerance() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").expect("strict oracle SPK required");
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::new();
    let context = PointingContext {
        kernel: &kernel,
        frames: &frames,
        pck: Some(&pck),
        fov: None,
    };
    let mut checked = 0;
    let mut failures = Vec::new();
    for line in include_str!("golden/pointing_sources_exact.csv").lines() {
        if line.starts_with('#') || line.starts_with("kind,") {
            continue;
        }
        let f: Vec<_> = line.split(',').collect();
        assert_eq!(f.len(), 12);
        let target = f[1].parse().unwrap();
        let observer = f[2].parse().unwrap();
        let epoch = f[3].parse().unwrap();
        let aberration = Aberration::parse(f[4]).unwrap();
        let direction = if f[0] == "POSITION" {
            PointingDirection::Position {
                target,
                observer,
                aberration,
                anti: false,
            }
        } else {
            PointingDirection::Velocity {
                target,
                observer,
                input_frame: f[5].into(),
                aberration,
                anti: false,
            }
        };
        let request = PointingRequest {
            epoch,
            direction,
            output_frame: f[6].into(),
            magnitude: PointingMagnitude::parse(f[7]).unwrap(),
            representation: PointingRepresentation::Rectangular,
        };
        let results = pointing_calculation(&context, &request).unwrap();
        assert_eq!(results.len(), 1);
        let actual = results[0];
        let expected_epoch: f64 = f[8].parse().unwrap();
        if actual.frame_epoch.to_bits() != expected_epoch.to_bits() {
            failures.push(format!(
                "{line}: epoch {:?} != {expected_epoch:?}",
                actual.frame_epoch
            ));
        }
        for axis in 0..3 {
            let expected: f64 = f[9 + axis].parse().unwrap();
            let value = actual.columns.vector[axis];
            if value.to_bits() != expected.to_bits() {
                failures.push(format!(
                    "{line}: axis{axis} actual={value:?} expected={expected:?} raw={:?}",
                    value - expected
                ));
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 540);
    if !failures.is_empty() {
        std::fs::write("target/pointing-source-mismatches.txt", failures.join("\n")).unwrap();
    }
    assert!(
        failures.is_empty(),
        "{} exact failures; first failures: {:#?}",
        failures.len(),
        &failures[..failures.len().min(20)]
    );
}
