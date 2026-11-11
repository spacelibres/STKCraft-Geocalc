use stkcraft_geocalc::{
    find_ray_in_fov_in, ray_in_fov_in, FinderOptions, Fov, Frames, RayAberration, SpkKernel,
};
#[test]
fn visibility_width_limit_errors_equal_cspice() {
    let text = include_str!("golden/fov_width_limits.ti");
    let frames = Frames::new();
    let kernel = SpkKernel::open("tests/golden/spk_priority.bsp").unwrap();
    let mut failures = Vec::new();
    for line in include_str!("golden/fov_width_limits.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let id = f[0].parse().unwrap();
        let fov = Fov::from_text(text, id).expect("GETFOV allows these definitions");
        let point = ray_in_fov_in(
            &kernel,
            None,
            &fov,
            399,
            0.,
            RayAberration::None,
            [0., 0., 1.],
            "J2000",
            &frames,
        );
        let empty = find_ray_in_fov_in(
            &kernel,
            None,
            &fov,
            399,
            RayAberration::None,
            [0., 0., 1.],
            "J2000",
            &frames,
            &[],
            FinderOptions::default(),
        );
        if point.is_ok() != (f[1] == "1") || empty.is_ok() != (f[2] == "1") {
            failures.push(format!(
                "{id}: ray{point:?}, empty{empty:?}, native {:?}",
                &f[1..]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
