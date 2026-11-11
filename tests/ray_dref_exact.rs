//! Typed high-level rays against native named-DREF SINCPT/TANGPT.
use stkcraft_geocalc::{
    resolve_ray_directions_with_pck, surface_intercept_points, tangent_points, Aberration, Frames,
    Pck, RayDirection, SpkKernel,
};

#[test]
fn exact_cspice_typed_ray_direction_frame_epochs() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::from_text(include_str!("golden/gf_intercept_dref.tf")).unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    for line in include_str!("golden/ray_dref_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let target = f[0].parse().unwrap();
        let observer = f[1].parse().unwrap();
        let et = f[2].parse().unwrap();
        let corr = Aberration::parse(f[3]).unwrap();
        let direction = RayDirection::VectorInFrame {
            frame: f[4].into(),
            vector: [
                f[5].parse().unwrap(),
                f[6].parse().unwrap(),
                f[7].parse().unwrap(),
            ],
            anti: f[8] == "1",
            stellar_aberration: false,
        };
        let expected: Vec<f64> = f[9..].iter().map(|v| v.parse().unwrap()).collect();
        let resolved = resolve_ray_directions_with_pck(
            &kernel, None, &frames, &pck, &direction, observer, et, corr,
        )
        .unwrap();
        if bits(&resolved[0]) != bits(&expected[..3]) {
            errors.push(format!("resolved {line}:actual{resolved:?}"));
        }
        let surf = surface_intercept_points(
            &kernel, &pck, None, &frames, target, observer, et, corr, &direction,
        );
        if expected[3] == 0. {
            if surf.is_ok() {
                errors.push(format!("miss {line}:actual{surf:?}"));
            }
        } else {
            match surf {
                Ok(s) => {
                    let actual = [s[0].point[0], s[0].point[1], s[0].point[2], s[0].altitude];
                    if bits(&actual) != bits(&expected[4..8]) {
                        errors.push(format!("surface {line}:actual{actual:?}"));
                    }
                }
                Err(e) => errors.push(format!("surface {line}:error{e}")),
            }
        }
        let t = tangent_points(
            &kernel, &pck, None, &frames, target, observer, et, corr, &direction,
        )
        .unwrap()[0];
        let actual = [
            t.point[0],
            t.point[1],
            t.point[2],
            t.altitude,
            t.range,
            t.surface_point[0],
            t.surface_point[1],
            t.surface_point[2],
            t.epoch,
            t.surface_vector[0],
            t.surface_vector[1],
            t.surface_vector[2],
        ];
        if bits(&actual) != bits(&expected[8..]) {
            errors.push(format!("tangent {line}:actual{actual:?}"));
        }
        count += 1;
    }
    assert!(
        errors.is_empty(),
        "{} differing outputs/{count} cases:\n{}",
        errors.len(),
        errors
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{count} exact native typed DREF ray cases");
}
