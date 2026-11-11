use stkcraft_geocalc::pointing::*;
use stkcraft_geocalc::{Frames, Pck, RayDirection, SpkKernel};
#[test]
fn vector_sources_match_independent_cspice_exactly() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let frames = Frames::new();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let context = PointingContext {
        kernel: &kernel,
        frames: &frames,
        pck: Some(&pck),
        fov: None,
    };
    let mut checked = 0;
    let mut failures = Vec::new();
    for line in include_str!("golden/pointing_vectors_exact.csv").lines() {
        if line.starts_with('#') || line.starts_with("et,") {
            continue;
        }
        let f: Vec<_> = line.split(',').collect();
        assert_eq!(f.len(), 13);
        let epoch = f[0].parse().unwrap();
        let correction = PointingVectorCorrection::parse(f[1]).unwrap();
        let vector = [
            f[6].parse().unwrap(),
            f[7].parse().unwrap(),
            f[8].parse().unwrap(),
        ];
        let request = PointingRequest {
            epoch,
            direction: PointingDirection::Vector {
                direction: RayDirection::VectorInFrame {
                    frame: f[2].into(),
                    vector,
                    anti: false,
                    stellar_aberration: false,
                },
                observer: 399,
                aberration: correction,
                anti: f[4] == "true",
            },
            output_frame: f[3].into(),
            magnitude: PointingMagnitude::parse(f[5]).unwrap(),
            representation: PointingRepresentation::Rectangular,
        };
        let result = pointing_calculation(&context, &request).unwrap()[0];
        let expected_epoch: f64 = f[9].parse().unwrap();
        assert_eq!(result.frame_epoch.to_bits(), expected_epoch.to_bits());
        for axis in 0..3 {
            let e: f64 = f[10 + axis].parse().unwrap();
            let a = result.columns.vector[axis];
            if a.to_bits() != e.to_bits() {
                failures.push(format!(
                    "{line}: axis{axis} actual={a:?} expected={e:?} raw={:?}",
                    a - e
                ));
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 336);
    if !failures.is_empty() {
        std::fs::write("target/pointing-vector-mismatches.txt", failures.join("\n")).unwrap();
    }
    assert!(
        failures.is_empty(),
        "{} exact failures; first {:#?}",
        failures.len(),
        &failures[..failures.len().min(10)]
    );
}
