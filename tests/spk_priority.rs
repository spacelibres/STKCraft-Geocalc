//! Exact CSPICE values, not a relative-error acceptance test.
use stkcraft_geocalc::SpkKernel;

#[test]
fn segment_priority_and_center_chains_match_cspice_exactly() {
    let kernel = SpkKernel::open(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/spk_priority.bsp"
    ))
    .expect("committed independent reference fixture");
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/spk_priority.csv").lines().skip(1) {
        let fields: Vec<_> = line.split(',').collect();
        let target = fields[0].parse().unwrap();
        let observer = fields[1].parse().unwrap();
        let et = fields[2].parse().unwrap();
        let expected: Vec<f64> = fields[3..].iter().map(|s| s.parse().unwrap()).collect();
        let result = kernel
            .state(target, observer, et)
            .expect("reference coverage");
        let actual = [result.position, result.velocity].concat();
        for (axis, (got, want)) in actual.iter().zip(expected.iter()).enumerate() {
            if got != want {
                failures.push(format!(
                    "{target}/{observer} et={et} axis={axis}: {got:?} != {want:?}"
                ));
            }
        }
        count += 1;
    }
    assert_eq!(count, 34);
    assert!(
        failures.is_empty(),
        "{} exact mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
