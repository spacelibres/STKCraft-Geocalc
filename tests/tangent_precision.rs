use stkcraft_geocalc::tangent::{tangent_point_with_locus, CorrectionLocus};
use stkcraft_geocalc::{Aberration, Pck, SpkKernel};
#[test]
fn tangent_exact_cspice() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut count = 0;
    let mut worst = 0.0_f64;
    let mut label = String::new();
    let mut cases = std::collections::BTreeMap::<String, usize>::new();
    for line in include_str!("golden/tangent_precision.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let locus = CorrectionLocus::parse(f[4]).unwrap();
        let v: Vec<f64> = f[5..].iter().map(|x| x.parse().unwrap()).collect();
        let a = tangent_point_with_locus(
            &kernel,
            &pck,
            f[0].parse().unwrap(),
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
            Aberration::parse(f[3]).unwrap(),
            [v[0], v[1], v[2]],
            locus,
        )
        .unwrap();
        for (a, b) in a
            .point
            .iter()
            .copied()
            .chain([a.altitude, a.range])
            .chain(a.surface_point)
            .chain([a.epoch])
            .chain(a.surface_vector)
            .zip(&v[3..])
        {
            if a.to_bits() != b.to_bits() {
                count += 1;
                *cases
                    .entry(format!("{} {} {} {}", f[0], f[2], f[3], f[4]))
                    .or_default() += 1;
            }
            if (a - b).abs() > worst {
                worst = (a - b).abs();
                label = line.to_string();
            }
        }
    }
    eprintln!("tangent {count} mismatches, max{worst} at{label}; groups{cases:?}");
    assert_eq!(count, 0);
}
#[test]
fn npedln_exact_cspice() {
    let mut count = 0;
    let mut worst = 0.0_f64;
    for line in include_str!("golden/npedln_precision.csv").lines() {
        let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
        let (p, h) = stkcraft_geocalc::tangent::nearest_point_on_ellipsoid_to_line(
            [v[0], v[1], v[2]],
            [v[3], v[4], v[5]],
            [v[6], v[7], v[8]],
        )
        .unwrap();
        for (a, b) in p.into_iter().chain([h]).zip(&v[9..]) {
            if a.to_bits() != b.to_bits() {
                count += 1;
            }
            worst = worst.max((a - b).abs());
        }
    }
    eprintln!("npedln {count} differences, max{worst}");
    assert_eq!(count, 0);
}
