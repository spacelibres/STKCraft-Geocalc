use std::collections::HashMap;
use stkcraft_geocalc::{
    ellipsoid_in_fov, ellipsoid_in_fov_with_frame, Aberration, Fov, Frames, Pck, SpkKernel,
};
#[test]
fn selectable_ellipsoid_frames_match_cspice() {
    let text = include_str!("golden/fov_target_frames.tf");
    let frames = Frames::from_text(text).unwrap();
    let mut pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    pck.furnsh(text).unwrap();
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let mut cache = HashMap::new();
    let mut failures = Vec::new();
    let mut visible = 0;
    let mut count = 0;
    let mut changed = 0;
    for line in include_str!("golden/fov_target_frames.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let id = f[0].parse().unwrap();
        let fov = cache
            .entry(id)
            .or_insert_with(|| Fov::from_text(text, id).unwrap());
        let actual = ellipsoid_in_fov_with_frame(
            &kernel,
            &pck,
            fov,
            399,
            301,
            f[1].parse().unwrap(),
            Aberration::parse(f[2]).unwrap(),
            &frames,
            f[3],
        )
        .unwrap();
        if actual != (f[4] == "1") {
            failures.push(line.to_string());
        }
        if f[3] == "ROTATED_EARTH" {
            let default = ellipsoid_in_fov(
                &kernel,
                &pck,
                fov,
                399,
                301,
                f[1].parse().unwrap(),
                Aberration::parse(f[2]).unwrap(),
                &frames,
            )
            .unwrap();
            changed += usize::from(default != actual);
        }
        count += 1;
        visible += usize::from(actual);
    }
    assert_eq!(count, 864);
    assert!(visible > 100 && visible < count);
    assert!(
        changed > 0,
        "test must distinguish the selected target frame from the default frame"
    );
    assert!(
        failures.is_empty(),
        "{} visibility mismatches: {:?}",
        failures.len(),
        failures.iter().take(5).collect::<Vec<_>>()
    );
    let fov = cache.values().next().unwrap();
    assert!(ellipsoid_in_fov_with_frame(
        &kernel,
        &pck,
        fov,
        399,
        301,
        0.,
        Aberration::None,
        &frames,
        "IAU_MOON"
    )
    .is_err());
}
