use stkcraft_geocalc::surface::*;
use stkcraft_geocalc::{Aberration, Pck, SpkKernel, SubPointMethod};
#[test]
fn surface_precision_cspice() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut worst = [0.0_f64; 5];
    let mut labels = [
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];
    let mut different = [0usize; 5];
    let mut values = [0usize; 5];
    for line in include_str!("golden/surface_precision.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let target = f[1].parse().unwrap();
        let obs = f[2].parse().unwrap();
        let et = f[3].parse().unwrap();
        let corr = Aberration::parse(f[4]).unwrap();
        let expected: Vec<f64> = f[6..].iter().map(|x| x.parse().unwrap()).collect();
        let (actual, which) = if f[0] == "illum" {
            let point = [expected[0], expected[1], expected[2]];
            let a = illumination_geometry(
                &kernel,
                &pck,
                target,
                f[5].parse().unwrap(),
                obs,
                et,
                corr,
                point,
            )
            .unwrap();
            (
                vec![
                    a.epoch,
                    a.surface_vector[0],
                    a.surface_vector[1],
                    a.surface_vector[2],
                    a.angles.phase,
                    a.angles.solar,
                    a.angles.emission,
                ],
                vec![2, 3, 3, 3, 4, 4, 4],
            )
        } else {
            let method = SubPointMethod::parse(f[5]).unwrap();
            let p = if f[0] == "observer" {
                sub_observer_geometry(&kernel, &pck, target, obs, et, corr, method)
            } else {
                sub_solar_geometry(&kernel, &pck, target, obs, et, corr, method)
            }
            .unwrap();
            (
                vec![
                    p.point[0],
                    p.point[1],
                    p.point[2],
                    p.epoch,
                    p.surface_vector[0],
                    p.surface_vector[1],
                    p.surface_vector[2],
                ],
                vec![0, 0, 0, 1, 3, 3, 3],
            )
        };
        let exp = if f[0] == "illum" {
            &expected[3..]
        } else {
            &expected[..7]
        };
        for ((a, b), k) in actual.iter().zip(exp).zip(which) {
            let d = (a - b).abs();
            values[k] += 1;
            if a.to_bits() != b.to_bits() {
                different[k] += 1;
                eprintln!("mismatch metric{k} actual{a:?} expected{b:?} {line}");
            }
            if d > worst[k] {
                worst[k] = d;
                labels[k] = format!("{} {} {} {} {} {}", f[0], f[1], f[2], f[3], f[4], f[5]);
            }
        }
    }
    for k in 0..5 {
        eprintln!(
            "metric{k}: {}/{} bit mismatches; max={} at {}",
            different[k], values[k], worst[k], labels[k]
        );
    }
    assert_eq!(
        different, [0; 5],
        "Exact CSPICE parity remains unresolved; diagnostics above preserve the raw residuals"
    );
}
#[test]
fn surface_ray_precision_cspice() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut different = [0usize; 3];
    let mut worst = [0.0_f64; 3];
    for line in include_str!("golden/surface_ray_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let target = f[0].parse().unwrap();
        let obs = f[1].parse().unwrap();
        let et = f[2].parse().unwrap();
        let corr = Aberration::parse(f[3]).unwrap();
        let v: Vec<f64> = f[4..].iter().map(|x| x.parse().unwrap()).collect();
        let p =
            surface_intercept_geometry(&kernel, &pck, target, obs, et, corr, [v[0], v[1], v[2]])
                .unwrap();
        for (i, a) in p
            .point
            .iter()
            .chain([p.epoch].iter())
            .chain(p.surface_vector.iter())
            .enumerate()
        {
            let k = if i < 3 {
                0
            } else if i == 3 {
                1
            } else {
                2
            };
            let b = v[3 + i];
            if a.to_bits() != b.to_bits() {
                different[k] += 1;
            }
            worst[k] = worst[k].max((a - b).abs());
        }
    }
    eprintln!("ray point/epoch/vector mismatches {different:?}; max{worst:?}");
    assert_eq!(different, [0; 3]);
}
#[test]
fn nearpt_exact_cspice() {
    let mut mismatch = 0;
    let mut worst = 0.0_f64;
    for line in include_str!("golden/nearpt_precision.csv").lines() {
        let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
        let actual = nearest_point_on_ellipsoid([v[0], v[1], v[2]], [v[3], v[4], v[5]]);
        let a = actual.unwrap();
        {
            for (a, b) in a.point.iter().copied().chain([a.altitude]).zip(&v[6..]) {
                worst = worst.max((a - b).abs());
                if a.to_bits() != b.to_bits() {
                    mismatch += 1;
                }
            }
        }
    }
    eprintln!("nearpt {mismatch} differing components, worst{worst}");
    assert_eq!(mismatch, 0);
}
#[test]
fn solar_longitude_exact_cspice() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut count = 0;
    let mut worst = 0.0_f64;
    for line in include_str!("golden/solar_longitude_precision.csv").lines() {
        let v: Vec<_> = line.split(',').collect();
        let expected = v[3].parse::<f64>().unwrap();
        let actual = planetocentric_solar_longitude(
            &kernel,
            &pck,
            v[0].parse().unwrap(),
            v[1].parse().unwrap(),
            Aberration::parse(v[2]).unwrap(),
        )
        .unwrap();
        if actual.to_bits() != expected.to_bits() {
            count += 1;
        }
        worst = worst.max((actual - expected).abs());
    }
    eprintln!("solar longitude {count} differences, max{worst}");
    assert_eq!(count, 0);
}
#[test]
fn georec_exact_cspice() {
    for line in include_str!("golden/georec_precision.csv").lines() {
        let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
        let actual = planetodetic_to_rectangular(v[1], v[0], v[2], [v[3], v[4], v[5]]).unwrap();
        assert_eq!(actual, [v[6], v[7], v[8]], "{line}");
    }
}
#[test]
fn near_limb_none_exact_cspice() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let v: Vec<f64> = include_str!("golden/surface_limb_precision.csv")
        .trim()
        .split(',')
        .map(|x| x.parse().unwrap())
        .collect();
    let rotation = pck.body_rotation(399, v[0]).unwrap();
    for (a, b) in rotation.iter().flatten().zip(&v[7..]) {
        assert_eq!(a, b, "rotation");
    }
    let frame = stkcraft_geocalc::FrameSource::BodyFixed {
        pck: &pck,
        body: 399,
    };
    let pos = kernel
        .apparent_state_in(399, 301, v[0], Aberration::None, &frame)
        .unwrap()
        .state
        .position;
    assert_eq!(pos, [v[4], v[5], v[6]], "initial position");
    let result = surface_intercept_geometry(
        &kernel,
        &pck,
        399,
        301,
        v[0],
        Aberration::None,
        [v[1], v[2], v[3]],
    )
    .unwrap();
    for (a, b) in result
        .point
        .into_iter()
        .chain([result.epoch])
        .chain(result.surface_vector)
        .zip(&v[16..])
    {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}
