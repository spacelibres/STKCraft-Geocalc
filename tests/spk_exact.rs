//! Independent full-precision CSPICE state corpus; no numerical tolerance.
use stkcraft_geocalc::SpkKernel;

#[test]
fn de440_geometric_states_equal_cspice() {
    let path = std::env::var("STKCRAFT_SPK").unwrap_or_else(|_| "target/kernels/de440s.bsp".into());
    let kernel = SpkKernel::open(path).expect("strict parity requires its reference kernel");
    let mut mismatches = Vec::new();
    let mut rows = 0;
    for line in include_str!("golden/spk_states.csv").lines() {
        if line.starts_with('#') || line.starts_with("et,") || line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split(',').collect();
        let et = fields[0].parse().unwrap();
        let target = fields[1].parse().unwrap();
        let observer = fields[2].parse().unwrap();
        let expected: Vec<f64> = fields[3..]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();
        let state = kernel.state(target, observer, et).unwrap();
        for (index, (actual, reference)) in [state.position, state.velocity]
            .concat()
            .iter()
            .zip(expected.iter())
            .enumerate()
        {
            if actual != reference {
                mismatches.push(format!(
                    "t={target} o={observer} et={et} axis={index}: {actual:?} != {reference:?}"
                ));
            }
        }
        rows += 1;
    }
    assert!(rows > 0);
    assert!(
        mismatches.is_empty(),
        "{} unequal components over {rows} states:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(16)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
