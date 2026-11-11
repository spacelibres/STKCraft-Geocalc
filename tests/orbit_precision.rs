use stkcraft_geocalc::{elements_from_state, State};
#[test]
fn oscltx_exact_cspice() {
    let mut mismatches = 0;
    let mut max = 0.0_f64;
    for line in include_str!("golden/orbit_precision.csv").lines() {
        let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
        let state = State {
            position: [v[0], v[1], v[2]],
            velocity: [v[3], v[4], v[5]],
        };
        let a = elements_from_state(&state, v[6]).unwrap();
        let actual = [
            a.perifocal_distance,
            a.eccentricity,
            a.inclination,
            a.ascending_node,
            a.argument_of_periapsis,
            a.mean_anomaly,
            a.true_anomaly,
            a.semi_major_axis,
            a.period,
            a.range,
            a.speed,
            a.gm,
        ];
        for (a, b) in actual.iter().zip(&v[7..]) {
            if a.to_bits() != b.to_bits() {
                mismatches += 1;
            }
            max = max.max((a - b).abs());
        }
    }
    eprintln!("orbital elements {mismatches} differing components max{max}");
    assert_eq!(mismatches, 0);
}
#[test]
fn orbit_observations_exact_cspice() {
    use stkcraft_geocalc::orbit::{osculating_elements_in, GmSource};
    use stkcraft_geocalc::{Aberration, Frames, Pck, SpkKernel};
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/gm_de440.tpc").unwrap();
    let frames = Frames::new();
    let mut mismatches = 0;
    let mut primitive = 0;
    let mut typedfields = [0; 12];
    let mut fields = [0; 12];
    let mut worst = 0.0_f64;
    let mut label = String::new();
    for line in include_str!("golden/orbit_observations_precision.csv").lines() {
        let f: Vec<_> = line.split(',').collect();
        let expected: Vec<f64> = f[7..].iter().map(|x| x.parse().unwrap()).collect();
        let gm = f[6].parse().unwrap();
        let source = GmSource::parse(f[5], Some(gm)).unwrap();
        let a = osculating_elements_in(
            &kernel,
            &pck,
            &frames,
            f[0].parse().unwrap(),
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
            Aberration::parse(f[3]).unwrap(),
            f[4],
            source,
        )
        .unwrap();
        let st = &expected[12..];
        let pure = elements_from_state(
            &State {
                position: [st[0], st[1], st[2]],
                velocity: [st[3], st[4], st[5]],
            },
            gm,
        )
        .unwrap();
        let pure = [
            pure.perifocal_distance,
            pure.eccentricity,
            pure.inclination,
            pure.ascending_node,
            pure.argument_of_periapsis,
            pure.mean_anomaly,
            pure.true_anomaly,
            pure.semi_major_axis,
            pure.period,
            pure.range,
            pure.speed,
            pure.gm,
        ];
        for (i, (a, b)) in pure.iter().zip(&expected).enumerate() {
            if a.to_bits() != b.to_bits() {
                primitive += 1;
                fields[i] += 1;
            }
        }
        let actual = [
            a.perifocal_distance,
            a.eccentricity,
            a.inclination,
            a.ascending_node,
            a.argument_of_periapsis,
            a.mean_anomaly,
            a.true_anomaly,
            a.semi_major_axis,
            a.period,
            a.range,
            a.speed,
            a.gm,
        ];
        for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
            if a.to_bits() != b.to_bits() {
                mismatches += 1;
                typedfields[i] += 1;
            }
            if (a - b).abs() > worst {
                worst = (a - b).abs();
                label = line.to_string();
            }
        }
    }
    eprintln!("primitive{primitive} fields{fields:?}; typed{typedfields:?}; orbital observations {mismatches} differences,max{worst} at{label}");
    assert_eq!(mismatches, 0);
}
