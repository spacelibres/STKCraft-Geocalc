use stkcraft_geocalc::{Aberration, FrameAxis, Frames, Pck, RayDirection, SpkKernel};
#[test]
fn resolved_rays_equal_cspice() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::new();
    let mut failures = Vec::new();
    for line in include_str!("golden/ray_resolver_exact.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let number = |i: usize| f[i].parse::<f64>().unwrap();
        let direction = match f[0] {
            "OBJECT" => RayDirection::ToObject {
                object: f[1].parse().unwrap(),
                anti: f[7] == "1",
            },
            "AXIS" => RayDirection::FrameAxis {
                frame: f[1].into(),
                axis: FrameAxis::parse(f[2]).unwrap(),
                anti: f[7] == "1",
            },
            _ => RayDirection::VectorInFrame {
                frame: f[1].into(),
                vector: [number(3), number(4), number(5)],
                anti: f[7] == "1",
                stellar_aberration: f[6] == "1",
            },
        };
        let result = stkcraft_geocalc::resolve_ray_directions_with_pck(
            &kernel,
            None,
            &frames,
            &pck,
            &direction,
            399,
            number(8),
            Aberration::parse(f[9]).unwrap(),
        );
        let expected = [number(10), number(11), number(12)];
        match result {
            Ok(v) if v == vec![expected] => {}
            result => failures.push(format!("{line}: {result:?} != {expected:?}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
