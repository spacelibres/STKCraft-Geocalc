use stkcraft_geocalc::location::state_between_in;
use stkcraft_geocalc::*;
#[test]
fn every_point_kind_locus_frame_and_correction_is_exact() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let frames = Frames::new();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut checked = 0;
    let mut failures = Vec::new();
    for line in include_str!("golden/location_states_exact.csv").lines() {
        if line.starts_with('#') || line.starts_with("kind,") {
            continue;
        }
        let f: Vec<_> = line.split(',').collect();
        assert_eq!(f.len(), 23);
        let n = |i: usize| f[i].parse::<f64>().unwrap();
        let et = n(1);
        let center = f[5].parse().unwrap();
        let point = PointLocation::constant_velocity(
            center,
            f[6],
            [n(7), n(8), n(9)],
            [n(10), n(11), n(12)],
            n(13),
        );
        let (target, observer) = if f[0].ends_with("TARGET") {
            (
                Endpoint::Point(&point),
                Endpoint::Object(f[15].parse().unwrap()),
            )
        } else {
            (
                Endpoint::Object(f[14].parse().unwrap()),
                Endpoint::Point(&point),
            )
        };
        let result = state_between_in(
            &kernel,
            &frames,
            &pck,
            target,
            observer,
            OutputFrame::Spice(f[2]),
            FrameLocus::parse(f[3]).unwrap(),
            et,
            Aberration::parse(f[4]).unwrap(),
        )
        .unwrap();
        for (i, a) in result
            .state
            .position
            .into_iter()
            .chain(result.state.velocity)
            .chain([result.light_time])
            .enumerate()
        {
            let e = n(16 + i);
            if a.to_bits() != e.to_bits() {
                failures.push(format!(
                    "row{checked} {} et{et} {} {} {} col{i} actual{a:?} expected{e:?} raw{:?}",
                    f[0],
                    f[2],
                    f[3],
                    f[4],
                    a - e
                ));
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 1296);
    if !failures.is_empty() {
        std::fs::write("target/location-state-mismatches.txt", failures.join("\n")).unwrap();
    }
    assert!(
        failures.is_empty(),
        "{} exactfailures first{:#?}",
        failures.len(),
        &failures[..failures.len().min(15)]
    );
}
