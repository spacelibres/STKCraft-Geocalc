use stkcraft_geocalc::Fov;
#[test]
fn getfov_validation_and_boundary_precedence_match_cspice() {
    let text = include_str!("golden/fov_validation.ti");
    let mut failures = Vec::new();
    for line in include_str!("golden/fov_validation.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let instrument = f[0].parse().unwrap();
        let actual = Fov::from_text(text, instrument);
        let expected = f[1] == "1";
        if actual.is_ok() != expected {
            failures.push(format!(
                "{instrument}: success {}, reference {expected}",
                actual.is_ok()
            ));
        } else if let Ok(value) = actual {
            if value.boundary[0][0] != f[2].parse::<f64>().unwrap() {
                failures.push(format!("{instrument}: boundary priority differs"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
