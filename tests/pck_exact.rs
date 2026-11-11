use stkcraft_geocalc::Pck;
#[test]
fn body_rotation_and_derivative_equal_cspice() {
    let pck = Pck::open("target/kernels/pck00011.tpc").expect("required reference PCK");
    let mut failures = Vec::new();
    let mut counts = [0usize; 2];
    for line in include_str!("golden/body_state_rotations.csv").lines() {
        if line.starts_with('#') || line.starts_with("et,") || line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split(',').collect();
        let et = fields[0].parse().unwrap();
        let body = fields[1].parse().unwrap();
        let expected: Vec<f64> = fields[3..].iter().map(|v| v.parse().unwrap()).collect();
        let (r, d) = pck.body_state_rotation(body, et).unwrap();
        for (index, (actual, reference)) in r
            .iter()
            .flatten()
            .chain(d.iter().flatten())
            .zip(expected.iter())
            .enumerate()
        {
            if actual != reference {
                counts[index / 9] += 1;
                failures.push(format!(
                    "{body} et={et} component={index}: {actual:?} != {reference:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Unequal rotation/derivative components {counts:?}:\n{}",
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
