use std::collections::HashMap;
use stkcraft_geocalc::Fov;

#[test]
fn fov_boundary_predicates_equal_cspice() {
    let mut cache = HashMap::new();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/fov_point_exact.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let key = (f[0].to_string(), f[1].parse::<i32>().unwrap());
        let fov = cache.entry(key.clone()).or_insert_with(|| {
            Fov::from_text(
                &std::fs::read_to_string(format!("tests/golden/{}", key.0)).unwrap(),
                key.1,
            )
            .unwrap()
        });
        let vector = [
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
            f[4].parse().unwrap(),
        ];
        let actual = fov.contains_direction(vector);
        let expected = f[5] == "1";
        if actual != expected {
            failures.push(format!(
                "{} {} {vector:?}: {actual} != {expected}",
                key.0, key.1
            ));
        }
        count += 1;
    }
    assert!(count > 1000);
    std::fs::write("target/fov-point-mismatches.txt", failures.join("\n")).unwrap();
    assert!(
        failures.is_empty(),
        "{} of {count} FOV point predicates differ:\n{}",
        failures.len(),
        failures
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
