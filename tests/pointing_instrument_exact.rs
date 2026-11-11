use stkcraft_geocalc::pointing::*;
use stkcraft_geocalc::{Fov, Frames, RayDirection, SpkKernel};
const IK: &str = r#"\begindata
INS-999_FOV_CLASS_SPEC = 'CORNERS'
INS-999_FOV_SHAPE = 'RECTANGLE'
INS-999_FOV_FRAME = 'J2000'
INS-999_BORESIGHT = (0.1,0.2,7.3)
INS-999_FOV_BOUNDARY_CORNERS = (1,1,4,-1,1,4,-1,-1,4,1,-1,4)
\begintext"#;
#[test]
fn getfov_original_magnitude_and_every_corner_are_exact() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let frames = Frames::new();
    let fov = Fov::from_text(IK, -999).unwrap();
    let context = PointingContext {
        kernel: &kernel,
        frames: &frames,
        pck: None,
        fov: Some(&fov),
    };
    let mut checked = 0;
    for line in include_str!("golden/pointing_instrument_exact.csv").lines() {
        if line.starts_with('#') || line.starts_with("kind,") {
            continue;
        }
        let f: Vec<_> = line.split(',').collect();
        assert_eq!(f.len(), 10);
        let index: usize = f[1].parse().unwrap();
        let boundary = f[0] == "BOUNDARY";
        let request = PointingRequest {
            epoch: 0.,
            direction: PointingDirection::Vector {
                direction: if boundary {
                    RayDirection::InstrumentFovBoundaryVectors
                } else {
                    RayDirection::InstrumentBoresight
                },
                observer: 399,
                aberration: PointingVectorCorrection::None,
                anti: false,
            },
            output_frame: f[2].into(),
            magnitude: PointingMagnitude::parse(f[3]).unwrap(),
            representation: PointingRepresentation::Rectangular,
        };
        let results = pointing_calculation(&context, &request).unwrap();
        assert_eq!(results.len(), if boundary { 4 } else { 1 });
        let result = results[if boundary { index - 1 } else { 0 }];
        let raw = if boundary {
            fov.boundary[index - 1]
        } else {
            fov.original_boresight()
        };
        for axis in 0..3 {
            let r: f64 = f[4 + axis].parse().unwrap();
            let e: f64 = f[7 + axis].parse().unwrap();
            assert_eq!(raw[axis].to_bits(), r.to_bits(), "GETFOV raw {line}");
            assert_eq!(
                result.columns.vector[axis].to_bits(),
                e.to_bits(),
                "pointing output {line}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 20);
}
