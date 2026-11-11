use stkcraft_geocalc::Frames;
#[test]
fn builtin_metadata_matches_cspice() {
    let frames = Frames::new();
    for line in include_str!("golden/builtin_frames.csv").lines().skip(1) {
        let fields: Vec<_> = line.split(',').collect();
        let id: i32 = fields[0].parse().unwrap();
        let center: i32 = fields[2].parse().unwrap();
        let class: i32 = fields[3].parse().unwrap();
        assert_eq!(frames.frame_id(fields[1]).unwrap(), id);
        assert_eq!(frames.frame_id(&id.to_string()).unwrap(), id);
        assert_eq!(frames.center(fields[1]).unwrap(), center);
        assert_eq!(frames.is_inertial(fields[1]).unwrap(), class == 1);
    }
    assert_eq!(frames.center("IAU_399").unwrap(), 399);
    assert_eq!(frames.center("IAU_MOON").unwrap(), 301);
    assert_eq!(frames.frame_id("IAU_301").unwrap(), 10020);
}

#[test]
fn equal_body_fixed_frames_have_exact_identity() {
    let frames = Frames::new();
    let pck = stkcraft_geocalc::Pck::open("target/kernels/pck00011.tpc").unwrap();
    let result = frames
        .pxform_with_derivative("IAU_301", "IAU_MOON", 5e8, Some(&pck))
        .unwrap();
    assert_eq!(result.rotation, [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
    assert_eq!(result.rotation_dot, [[0.; 3]; 3]);
}
