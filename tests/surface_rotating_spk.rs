use stkcraft_geocalc::surface::*;
use stkcraft_geocalc::tangent::*;
use stkcraft_geocalc::{Aberration, Frames, Pck, SpkKernel, SubPointMethod};
#[test]
fn rotating_spk_sources_match_cspice_surface_geometry() {
    let kernel = SpkKernel::open_all(&[
        "target/kernels/de440s.bsp",
        "tests/golden/spk_mixed_frames.bsp",
    ])
    .unwrap();
    let mut native = Pck::open("target/kernels/pck00011.tpc").unwrap();
    native
        .furnsh(include_str!("golden/surface_rotating_spk.tf"))
        .unwrap();
    let triaxial = native.clone();
    let frames = Frames::open("tests/golden/surface_rotating_spk.tf").unwrap();
    let (mut checked, mut failures) = (0, 0);
    for line in include_str!("golden/surface_rotating_spk.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let pck = if f[0] == "native" { &native } else { &triaxial };
        let et = f[3].parse().unwrap();
        let corr = Aberration::parse(f[4]).unwrap();
        let input = [
            f[6].parse().unwrap(),
            f[7].parse().unwrap(),
            f[8].parse().unwrap(),
        ];
        let actual = match f[1] {
            "SUBPNT" | "SUBSLR" => {
                let method = SubPointMethod::parse(f[5]).unwrap();
                let a = if f[1] == "SUBPNT" {
                    sub_observer_geometry_in(
                        &kernel, pck, &frames, f[2], 90001, 399, et, corr, method,
                    )
                } else {
                    sub_solar_geometry_in(&kernel, pck, &frames, f[2], 90001, 399, et, corr, method)
                }
                .unwrap();
                vec![
                    a.point[0],
                    a.point[1],
                    a.point[2],
                    a.epoch,
                    a.surface_vector[0],
                    a.surface_vector[1],
                    a.surface_vector[2],
                ]
            }
            "SINCPT" => {
                let a = surface_intercept_geometry_in(
                    &kernel, pck, &frames, f[2], 90001, 399, et, corr, input,
                )
                .unwrap();
                vec![
                    a.point[0],
                    a.point[1],
                    a.point[2],
                    a.epoch,
                    a.surface_vector[0],
                    a.surface_vector[1],
                    a.surface_vector[2],
                ]
            }
            "ILLUMF" => {
                let a = illumination_geometry_in(
                    &kernel, pck, &frames, f[2], 90001, 10, 399, et, corr, input,
                )
                .unwrap();
                assert_eq!(a.visible, f[16] == "true", "{line}");
                assert_eq!(a.illuminated, f[17] == "true", "{line}");
                vec![
                    a.epoch,
                    a.surface_vector[0],
                    a.surface_vector[1],
                    a.surface_vector[2],
                    a.angles.phase,
                    a.angles.solar,
                    a.angles.emission,
                ]
            }
            "TANGPT" => {
                let a = tangent_point_in(
                    &kernel,
                    pck,
                    &frames,
                    f[2],
                    90001,
                    399,
                    et,
                    corr,
                    input,
                    CorrectionLocus::parse(f[5]).unwrap(),
                )
                .unwrap();
                vec![
                    a.point[0],
                    a.point[1],
                    a.point[2],
                    a.altitude,
                    a.range,
                    a.surface_point[0],
                    a.surface_point[1],
                    a.surface_point[2],
                    a.epoch,
                    a.surface_vector[0],
                    a.surface_vector[1],
                    a.surface_vector[2],
                ]
            }
            _ => panic!("unknown fixture"),
        };
        for (i, &a) in actual.iter().enumerate() {
            let e: f64 = f[9 + i].parse().unwrap();
            if a.to_bits() != e.to_bits() {
                if failures < 5 {
                    eprintln!(
                        "{} {} {} {} col{} actual={a:?} expected={e:?}",
                        f[0], f[1], f[2], f[4], i
                    );
                }
                failures += 1;
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 2262);
    assert_eq!(failures, 0, "exact component mismatches");
}
