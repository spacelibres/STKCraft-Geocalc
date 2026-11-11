use stkcraft_geocalc::{Aberration, SpkKernel};

#[test]
fn corrected_states_equal_cspice() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").expect("required reference kernel");
    let mut mismatches = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/abcorr_states.csv").lines() {
        if line.is_empty() || line.starts_with('#') || line.starts_with("et,") {
            continue;
        }
        let fields: Vec<_> = line.split(',').collect();
        let et = fields[0].parse().unwrap();
        let target = fields[1].parse().unwrap();
        let observer = fields[2].parse().unwrap();
        let correction = Aberration::parse(fields[3]).unwrap();
        let expected: Vec<f64> = fields[4..].iter().map(|v| v.parse().unwrap()).collect();
        let result = kernel
            .state_corrected(target, observer, et, correction)
            .unwrap();
        let mut actual = [result.state.position, result.state.velocity].concat();
        actual.push(result.light_time);
        for (index, (got, want)) in actual.iter().zip(expected.iter()).enumerate() {
            if got != want {
                mismatches.push(format!(
                    "{} axis={index} {got:?} != {want:?}",
                    fields[..4].join(",")
                ));
            }
        }
        count += 1;
    }
    assert!(count > 0);
    assert!(
        mismatches.is_empty(),
        "{} exact mismatches over {count} rows:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
