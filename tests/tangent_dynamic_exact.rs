use stkcraft_geocalc::{tangent::*, Aberration, Frames, Pck, SpkKernel};
#[test]
fn native_tangent_dynamic_target_frame_exact() {
    let kernel = SpkKernel::open_all(&[
        "target/kernels/de440s.bsp",
        "tests/golden/spk_dynamic_sources.bsp",
    ])
    .unwrap();
    let frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/tangent_dynamic.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let direction = std::array::from_fn(|i| f[6 + i].parse().unwrap());
        let result = tangent_point_in(
            &kernel,
            &pck,
            &frames,
            f[3],
            499,
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
            Aberration::parse(f[4]).unwrap(),
            direction,
            CorrectionLocus::parse(f[5]).unwrap(),
        )
        .map(|p| {
            [
                p.point[0],
                p.point[1],
                p.point[2],
                p.altitude,
                p.range,
                p.surface_point[0],
                p.surface_point[1],
                p.surface_point[2],
                p.epoch,
                p.surface_vector[0],
                p.surface_vector[1],
                p.surface_vector[2],
            ]
        });
        let expected: [f64; 12] = std::array::from_fn(|i| f[9 + i].parse().unwrap());
        if result.as_ref().ok() != Some(&expected) {
            failures.push(format!(
                "{}:{result:?} native{expected:?}",
                f[..6].join(",")
            ));
        }
        count += 1;
    }
    assert_eq!(count, 864);
    assert!(
        failures.is_empty(),
        "{}/{count} mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
