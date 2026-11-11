use std::collections::HashMap;
use stkcraft_geocalc::Fov;
#[test]
fn all_getfov_boundaries_equal_cspice() {
    let mut failures = Vec::new();
    let mut cache = HashMap::new();
    for line in include_str!("golden/fov_boundaries_exact.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let key = (f[0].to_string(), f[1].parse::<i32>().unwrap());
        let fov = cache.entry(key.clone()).or_insert_with(|| {
            let text = std::fs::read_to_string(format!("tests/golden/{}", key.0)).unwrap();
            Fov::from_text(&text, key.1).unwrap()
        });
        let n = |i: usize| f[i].parse::<f64>().unwrap();
        let index: usize = f[2].parse().unwrap();
        let actual = [fov.original_boresight(), fov.boundary[index]].concat();
        let expected = vec![n(3), n(4), n(5), n(6), n(7), n(8)];
        if actual != expected {
            failures.push(format!(
                "{} {} boundary{index}: {actual:?} != {expected:?}",
                f[0], f[1]
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} GETFOV mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
