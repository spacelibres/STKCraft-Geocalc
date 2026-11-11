use stkcraft_geocalc::*;
#[test]
fn state_columns_match_direct_cspice_jacobians_exactly() {
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut checked = 0;
    let mut failures = Vec::new();
    for line in include_str!("golden/state_columns_exact.csv").lines() {
        if line.starts_with('#') || line.starts_with("representation,") {
            continue;
        }
        let f: Vec<_> = line.split(',').collect();
        let nums: Vec<f64> = f[2..].iter().map(|x| x.parse().unwrap()).collect();
        let rep = if f[0].starts_with("AZ_EL_") {
            let bytes = f[0].as_bytes();
            StateRepresentation::azimuth_elevation(bytes[6] == b'T', bytes[7] == b'T')
        } else {
            StateRepresentation::parse(f[0]).unwrap()
        };
        let state = State {
            position: [nums[0], nums[1], nums[2]],
            velocity: [nums[3], nums[4], nums[5]],
        };
        let actual = state_columns_with_pck(12.3, state, rep, 4.5, Aberration::None, &pck, 399)
            .unwrap()
            .numeric_row();
        let expected = &nums[6..];
        assert_eq!(actual.len(), expected.len());
        for (column, (a, e)) in actual.iter().zip(expected).enumerate() {
            if a.to_bits() != e.to_bits() {
                failures.push(format!(
                    "row{checked} {} col{column} actual{a:?} expected{e:?} raw{:?}",
                    f[0],
                    a - e
                ));
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 58);
    if !failures.is_empty() {
        std::fs::write("target/state-column-mismatches.txt", failures.join("\n")).unwrap();
    }
    assert!(
        failures.is_empty(),
        "{} exactfailures first{:#?}",
        failures.len(),
        &failures[..failures.len().min(12)]
    );
}
