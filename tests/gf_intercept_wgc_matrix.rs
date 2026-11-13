//! Independent CSPICE GFSNTC windows for WGC ray and correction cases.
use stkcraft_geocalc::finder::find_surface_intercept_ray_coordinate_with_frame;
use stkcraft_geocalc::{
    Aberration, Condition, CoordinateSystem, FinderOptions, Frames, Interval, Pck, SpkKernel,
};

#[test]
fn native_wgc_intercept_longitude_windows_match_bits() {
    let kernel = SpkKernel::open("target/kernels/de442.bsp").unwrap();
    let pck = Pck::open_all(&[
        "target/kernels/pck00011.tpc",
        "target/kernels/mars_iau2000_v1.tpc",
        "target/kernels/gm_de431.tpc",
        "target/kernels/moon_pa_de440_200625.bpc",
    ])
    .unwrap();
    let mut frames = Frames::new();
    for path in [
        "target/kernels/moon_de440_250416.tf",
        "target/kernels/moon_assoc_me.tf",
    ] {
        frames
            .furnsh(&std::fs::read_to_string(path).unwrap())
            .unwrap();
    }
    let direction = [-291608.3847334454, -266716.83330418577, -76102.48704883647];
    let mut count = 0;
    for line in include_str!("golden/gf_intercept_wgc_matrix_exact.csv")
        .lines()
        .skip(2)
    {
        let fields: Vec<_> = line.split(',').collect();
        let reference: f64 = fields[2].parse().unwrap();
        let windows = find_surface_intercept_ray_coordinate_with_frame(
            &kernel,
            &pck,
            &frames,
            "MOON_ME",
            301,
            399,
            Aberration::parse(fields[0]).unwrap(),
            direction,
            "J2000",
            CoordinateSystem::Latitudinal,
            1,
            &[Interval {
                start: -10000.0,
                end: 10000.0,
            }],
            Condition::parse(fields[1]).unwrap(),
            reference.to_radians(),
            FinderOptions {
                step: 60.0,
                ..FinderOptions::default()
            },
        )
        .unwrap();
        assert_eq!(windows.len(), 1, "{line}");
        let values = [
            windows[0].start,
            windows[0].end,
            windows[0].end - windows[0].start,
        ];
        for (value, expected) in values.iter().zip(&fields[3..]) {
            assert_eq!(
                value.to_bits(),
                u64::from_str_radix(expected, 16).unwrap(),
                "{line}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 9);
}
