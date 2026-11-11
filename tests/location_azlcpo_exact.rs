use stkcraft_geocalc::location::state_between_in;
use stkcraft_geocalc::*;
#[test]
fn azlcpo_every_flag_correction_and_pole_is_exact() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let frames = Frames::new();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut checked = 0;
    let mut failures = Vec::new();
    for line in include_str!("golden/location_azlcpo_exact.csv").lines() {
        if line.starts_with('#') || line.starts_with("et,") {
            continue;
        }
        let f: Vec<_> = line.split(',').collect();
        assert_eq!(f.len(), 14);
        let n = |i: usize| f[i].parse::<f64>().unwrap();
        let et = n(0);
        let az = f[4] == "true";
        let el = f[5] == "true";
        let correction = Aberration::parse(f[6]).unwrap();
        let point = PointLocation::fixed(399, "IAU_EARTH", [n(1), n(2), n(3)]);
        let observer = Endpoint::Point(&point);
        let state = state_between_in(
            &kernel,
            &frames,
            &pck,
            Endpoint::Object(301),
            observer,
            OutputFrame::Topocentric(&point),
            FrameLocus::Observer,
            et,
            correction,
        )
        .unwrap();
        let c = state_columns_with_pck(
            et,
            state.state,
            StateRepresentation::azimuth_elevation(az, el),
            state.light_time,
            correction,
            &pck,
            399,
        )
        .unwrap();
        let actual = [
            c.coordinates[2],
            c.coordinates[0],
            c.coordinates[1],
            c.rates[2],
            c.rates[0],
            c.rates[1],
            state.light_time,
        ];
        for (i, a) in actual.into_iter().enumerate() {
            let e = n(7 + i);
            if a.to_bits() != e.to_bits() {
                failures.push(format!(
                    "row{checked} et{et} {} az{az} el{el} col{i} actual{a:?} expected{e:?} raw{:?}",
                    f[6],
                    a - e
                ));
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 324);
    if !failures.is_empty() {
        std::fs::write("target/location-azlcpo-mismatches.txt", failures.join("\n")).unwrap();
    }
    assert!(
        failures.is_empty(),
        "{} exactfailures first{:#?}",
        failures.len(),
        &failures[..failures.len().min(15)]
    );
}
