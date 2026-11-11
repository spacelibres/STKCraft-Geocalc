//! Aberration-corrected states against CSPICE `spkezr`.
//!
//! `tests/golden/abcorr_states.csv` is produced by `spkezr` for every
//! supported `ABCORR` value over a grid of epochs and target/observer
//! pairs from `de440s.bsp`. The SPK itself is not committed; set
//! `STKCRAFT_SPK` or run `tools/fetch_spk.py`.
//!
//! Differences are judged on relative error. Light time is compared
//! absolutely in seconds.

use std::collections::BTreeMap;
use std::path::PathBuf;

use stkcraft_geocalc::{Aberration, SpkKernel};

const CSV: &str = include_str!("golden/abcorr_states.csv");

fn kernel_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("STKCRAFT_SPK") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let default = PathBuf::from("target/kernels/de440s.bsp");
    if default.exists() {
        Some(default)
    } else {
        None
    }
}

fn relative_error(value: f64, expected: f64) -> f64 {
    (value - expected).abs() / expected.abs().max(1.0)
}

#[test]
fn matches_cspice_spkezr_aberrations() {
    let Some(path) = kernel_path() else {
        eprintln!("skipping aberration golden test: set STKCRAFT_SPK or run tools/fetch_spk.py");
        return;
    };

    let kernel = SpkKernel::open(&path).expect("open SPK kernel");

    let mut position: BTreeMap<String, f64> = BTreeMap::new();
    let mut velocity: BTreeMap<String, f64> = BTreeMap::new();
    let mut position_abs: BTreeMap<String, f64> = BTreeMap::new();
    let mut velocity_abs: BTreeMap<String, f64> = BTreeMap::new();
    let mut light_time: BTreeMap<String, f64> = BTreeMap::new();
    let mut worst_label = String::new();
    let mut checked = 0;

    for line in CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("et,") {
            continue;
        }

        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 11 {
            continue;
        }

        let et: f64 = fields[0].parse().unwrap();
        let target: i32 = fields[1].parse().unwrap();
        let observer: i32 = fields[2].parse().unwrap();
        let abcorr_text = fields[3];
        let expected: Vec<f64> = fields[4..11]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();

        let abcorr = Aberration::parse(abcorr_text)
            .unwrap_or_else(|error| panic!("unknown abcorr {abcorr_text}: {error}"));
        let corrected = kernel
            .state_corrected(target, observer, et, abcorr)
            .unwrap_or_else(|error| {
                panic!("state_corrected({target}, {observer}, {et}, {abcorr_text}): {error}")
            });

        let mut local_worst = 0.0f64;
        for axis in 0..3 {
            let absolute = (corrected.state.position[axis] - expected[axis]).abs();
            let error = relative_error(corrected.state.position[axis], expected[axis]);
            if error > local_worst {
                local_worst = error;
                worst_label = format!(
                    "{abcorr_text} et={et} target={target} observer={observer} axis={axis}"
                );
            }
            position
                .entry(abcorr_text.to_string())
                .and_modify(|current| *current = current.max(error))
                .or_insert(error);
            position_abs
                .entry(abcorr_text.to_string())
                .and_modify(|current| *current = current.max(absolute))
                .or_insert(absolute);

            let absolute = (corrected.state.velocity[axis] - expected[3 + axis]).abs();
            let error = relative_error(corrected.state.velocity[axis], expected[3 + axis]);
            velocity
                .entry(abcorr_text.to_string())
                .and_modify(|current| *current = current.max(error))
                .or_insert(error);
            velocity_abs
                .entry(abcorr_text.to_string())
                .and_modify(|current| *current = current.max(absolute))
                .or_insert(absolute);
        }

        light_time
            .entry(abcorr_text.to_string())
            .and_modify(|current| {
                *current = current.max((corrected.light_time - expected[6]).abs());
            })
            .or_insert((corrected.light_time - expected[6]).abs());

        checked += 1;
    }

    assert!(checked > 0, "no golden rows were checked");

    println!("vs CSPICE spkezr over {checked} corrected states:");
    for (abcorr, error) in &position {
        println!("  {abcorr:8} position {error:.3e} relative");
    }
    for (abcorr, error) in &position_abs {
        println!("  {abcorr:8} position {error:.3e} km absolute");
    }
    for (abcorr, error) in &velocity {
        println!("  {abcorr:8} velocity {error:.3e} relative");
    }
    for (abcorr, error) in &velocity_abs {
        println!("  {abcorr:8} velocity {error:.3e} km/s absolute");
    }
    for (abcorr, error) in &light_time {
        println!("  {abcorr:8} light time {error:.3e} s absolute");
    }
    println!("  worst position row: {worst_label}");

    for (abcorr, error) in &position {
        assert!(
            *error < 1e-11,
            "{abcorr} position relative error {error:e} is too large"
        );
    }
    for (abcorr, error) in &velocity {
        assert!(
            *error < 1e-11,
            "{abcorr} velocity relative error {error:e} is too large"
        );
    }
}
