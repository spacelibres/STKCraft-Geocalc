use stkcraft_geocalc::surface::*;
use stkcraft_geocalc::tangent::*;
use stkcraft_geocalc::{Aberration, Frames, Pck, SpkKernel, SubPointMethod};
#[test]
fn explicit_target_frames_match_all_cspice_bits() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let native = Pck::open_all(&[
        "target/kernels/pck00011.tpc",
        "target/kernels/moon_pa_de440_200625.bpc",
    ])
    .unwrap();
    let mut triaxial = native.clone();
    triaxial
        .furnsh("KPL/PCK\n\\begindata\nBODY301_RADII=(1738.1 1736.7 1734.2)\n\\begintext")
        .unwrap();
    let mut frames = Frames::new();
    for file in [
        "target/kernels/moon_de440_250416.tf",
        "target/kernels/moon_assoc_me.tf",
        "tests/golden/surface_target_frames.tf",
    ] {
        frames
            .furnsh(&std::fs::read_to_string(file).unwrap())
            .unwrap();
    }
    let (mut checked, mut failures) = (0, 0);
    for line in include_str!("golden/surface_target_frames.csv")
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
                        &kernel, pck, &frames, f[2], 301, 399, et, corr, method,
                    )
                } else {
                    sub_solar_geometry_in(&kernel, pck, &frames, f[2], 301, 399, et, corr, method)
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
                    &kernel, pck, &frames, f[2], 301, 399, et, corr, input,
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
                    &kernel, pck, &frames, f[2], 301, 10, 399, et, corr, input,
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
                    301,
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
    assert_eq!(checked, 17460);
    assert_eq!(failures, 0, "exact component mismatches");
}
#[test]
fn target_frame_must_be_non_inertial_and_target_centered() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::new();
    for frame in ["J2000", "IAU_EARTH"] {
        assert!(sub_observer_geometry_in(
            &kernel,
            &pck,
            &frames,
            frame,
            301,
            399,
            0.,
            Aberration::None,
            SubPointMethod::NearPoint
        )
        .is_err());
        assert!(sub_solar_geometry_in(
            &kernel,
            &pck,
            &frames,
            frame,
            301,
            399,
            0.,
            Aberration::None,
            SubPointMethod::NearPoint
        )
        .is_err());
        assert!(surface_intercept_geometry_in(
            &kernel,
            &pck,
            &frames,
            frame,
            301,
            399,
            0.,
            Aberration::None,
            [1., 0., 0.]
        )
        .is_err());
        assert!(illumination_geometry_in(
            &kernel,
            &pck,
            &frames,
            frame,
            301,
            10,
            399,
            0.,
            Aberration::None,
            [1737.4, 0., 0.]
        )
        .is_err());
        assert!(tangent_point_in(
            &kernel,
            &pck,
            &frames,
            frame,
            301,
            399,
            0.,
            Aberration::None,
            [1., 0., 0.],
            CorrectionLocus::TangentPoint
        )
        .is_err());
    }
}
