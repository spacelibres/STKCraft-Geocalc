use stkcraft_geocalc::SpkKernel;
#[test]
fn spk18_packet_interpolation_equal_cspice() {
    let kernel = SpkKernel::open("tests/golden/spk18.bsp").unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/spk18.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let expected: Vec<f64> = f[3..].iter().map(|v| v.parse().unwrap()).collect();
        match kernel.state_inertial(
            f[0].parse().unwrap(),
            0,
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
        ) {
            Ok(state) => {
                let actual = [state.position, state.velocity].concat();
                if actual != expected {
                    failures.push(format!("{}: {actual:?} != {expected:?}", f[..3].join(",")));
                }
            }
            Err(e) => failures.push(format!("{}: {e}", f[..3].join(","))),
        }
        count += 1;
    }
    assert_eq!(count, 6720);
    assert!(
        failures.is_empty(),
        "{}/{} failed:\n{}",
        failures.len(),
        count,
        failures
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
