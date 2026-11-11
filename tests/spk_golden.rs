//! SPK state vectors against CSPICE `spkezr`.
//!
//! The kernel is not committed. Provide it with `STKCRAFT_SPK`, or place
//! it at `target/kernels/de440s.bsp` using `tools/fetch_spk.py`. When the
//! kernel is absent the test reports that it was skipped; the golden CSV
//! itself is committed.
//!
//! Differences are judged on **relative** error. At Pluto's distance one
//! `f64` ULP is about 1e-6 km, so an absolute tolerance of a nanometre
//! would be meaningless.

use std::path::PathBuf;

use stkcraft_geocalc::SpkKernel;

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

#[test]
fn matches_cspice_spkezr() {
    let Some(path) = kernel_path() else {
        eprintln!("skipping SPK golden test: set STKCRAFT_SPK or run tools/fetch_spk.py");
        return;
    };

    let kernel = SpkKernel::open(&path).expect("open SPK kernel");
    let csv = include_str!("golden/spk_states.csv");

    let mut worst_position_abs: f64 = 0.0;
    let mut worst_position_rel: f64 = 0.0;
    let mut worst_velocity_abs: f64 = 0.0;
    let mut worst_velocity_rel: f64 = 0.0;
    let mut worst_label = String::new();
    let mut checked = 0;

    for line in csv.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("et,") {
            continue;
        }

        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 9 {
            continue;
        }

        let et: f64 = fields[0].parse().unwrap();
        let target: i32 = fields[1].parse().unwrap();
        let observer: i32 = fields[2].parse().unwrap();
        let expected: Vec<f64> = fields[3..9]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();

        let state = kernel.state(target, observer, et).expect("state");

        for axis in 0..3 {
            let position_error = (state.position[axis] - expected[axis]).abs();
            let position_scale = expected[axis].abs().max(1.0);
            let position_relative = position_error / position_scale;

            if position_relative > worst_position_rel {
                worst_position_rel = position_relative;
                worst_label = format!(
                    "et={et} target={target} observer={observer} axis={axis} ours={} cspice={}",
                    state.position[axis], expected[axis]
                );
            }
            worst_position_abs = worst_position_abs.max(position_error);

            let velocity_error = (state.velocity[axis] - expected[3 + axis]).abs();
            let velocity_scale = expected[3 + axis].abs().max(1.0);
            worst_velocity_rel = worst_velocity_rel.max(velocity_error / velocity_scale);
            worst_velocity_abs = worst_velocity_abs.max(velocity_error);
        }
        checked += 1;
    }

    assert!(checked > 0, "no golden rows were checked");

    println!(
        "vs CSPICE over {checked} states:\n  position: {worst_position_abs:.3e} km absolute, {worst_position_rel:.3e} relative\n  velocity: {worst_velocity_abs:.3e} km/s absolute, {worst_velocity_rel:.3e} relative"
    );
    println!("  worst position row: {worst_label}");

    // An f64 has ~2.2e-16 relative precision. Allow 1e-12, which still
    // catches any real algorithmic error by many orders of magnitude.
    assert!(
        worst_position_rel < 1e-12,
        "position relative error {worst_position_rel:e} is too large"
    );
    assert!(
        worst_velocity_rel < 1e-12,
        "velocity relative error {worst_velocity_rel:e} is too large"
    );
}
