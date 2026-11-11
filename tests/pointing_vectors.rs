use stkcraft_geocalc::pointing::*;
use stkcraft_geocalc::{Aberration, Fov, FrameAxis, Frames, Pck, RayDirection, SpkKernel};
const IK: &str = r#"\begindata
INS-999_FOV_CLASS_SPEC = 'CORNERS'
INS-999_FOV_SHAPE = 'RECTANGLE'
INS-999_FOV_FRAME = 'J2000'
INS-999_BORESIGHT = (0,0,7)
INS-999_FOV_BOUNDARY_CORNERS = (1,1,4,-1,1,4,-1,-1,4,1,-1,4)
\begintext"#;

#[test]
fn raw_boresight_is_preserved_and_boundaries_keep_ik_order() {
    let fov = Fov::from_text(IK, -999).unwrap();
    assert_eq!(fov.original_boresight(), [0., 0., 7.]);
    assert_eq!(fov.boresight, [0., 0., 1.]);
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let frames = Frames::new();
    let context = PointingContext {
        kernel: &kernel,
        frames: &frames,
        pck: None,
        fov: Some(&fov),
    };
    let mut request = PointingRequest {
        epoch: 0.,
        direction: PointingDirection::Vector {
            direction: RayDirection::InstrumentBoresight,
            observer: 399,
            aberration: PointingVectorCorrection::None,
            anti: false,
        },
        output_frame: "J2000".into(),
        magnitude: PointingMagnitude::PreserveOriginal,
        representation: PointingRepresentation::Rectangular,
    };
    let result = pointing_calculation(&context, &request).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].columns.vector, [0., 0., 7.]);
    request.direction = PointingDirection::Vector {
        direction: RayDirection::InstrumentFovBoundaryVectors,
        observer: 399,
        aberration: PointingVectorCorrection::None,
        anti: false,
    };
    let results = pointing_calculation(&context, &request).unwrap();
    assert_eq!(results.len(), 4);
    for (index, result) in results.into_iter().enumerate() {
        assert_eq!(result.boundary_point_number, Some(index + 1));
        assert_eq!(result.columns.vector, fov.boundary[index]);
    }
}

#[test]
fn velocity_direction_does_not_add_output_frame_rate() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let frames = Frames::new();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let context = PointingContext {
        kernel: &kernel,
        frames: &frames,
        pck: Some(&pck),
        fov: None,
    };
    let request = PointingRequest {
        epoch: 0.,
        direction: PointingDirection::Velocity {
            target: 301,
            observer: 399,
            input_frame: "J2000".into(),
            aberration: Aberration::None,
            anti: false,
        },
        output_frame: "IAU_EARTH".into(),
        magnitude: PointingMagnitude::PreserveOriginal,
        representation: PointingRepresentation::Rectangular,
    };
    let result = pointing_calculation(&context, &request).unwrap()[0];
    let state = kernel.state(301, 399, 0.).unwrap();
    let m = frames
        .pxform_with_derivative("J2000", "IAU_EARTH", 0., Some(&pck))
        .unwrap()
        .rotation;
    for (row, coefficients) in m.iter().enumerate() {
        let expected = coefficients[0] * state.velocity[0]
            + coefficients[1] * state.velocity[1]
            + coefficients[2] * state.velocity[2];
        assert_eq!(result.columns.vector[row].to_bits(), expected.to_bits());
    }
}

#[test]
fn body_fixed_frame_axis_works_and_missing_fov_fails() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let frames = Frames::new();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let context = PointingContext {
        kernel: &kernel,
        frames: &frames,
        pck: Some(&pck),
        fov: None,
    };
    let mut request = PointingRequest {
        epoch: 0.,
        direction: PointingDirection::Vector {
            direction: RayDirection::FrameAxis {
                frame: "IAU_MOON".into(),
                axis: FrameAxis::X,
                anti: false,
            },
            observer: 399,
            aberration: PointingVectorCorrection::None,
            anti: false,
        },
        output_frame: "IAU_MOON".into(),
        magnitude: PointingMagnitude::PreserveOriginal,
        representation: PointingRepresentation::Rectangular,
    };
    assert_eq!(
        pointing_calculation(&context, &request).unwrap()[0]
            .columns
            .vector,
        [1., 0., 0.]
    );
    request.direction = PointingDirection::Vector {
        direction: RayDirection::InstrumentBoresight,
        observer: 399,
        aberration: PointingVectorCorrection::None,
        anti: false,
    };
    assert!(pointing_calculation(&context, &request).is_err());
}
