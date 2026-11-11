//! IAU body-fixed rotation matrices and their analytic time derivatives
//! against CSPICE `sxform`.
//!
//! The PCK is not committed. Place `pck00011.tpc` at
//! `target/kernels/pck00011.tpc`. When it is absent the test reports that
//! it was skipped; the golden CSV itself is committed.
//!
//! The derivative block of an `sxform` matrix is what turns a J2000
//! velocity into a body-fixed velocity, so comparing it exercises the
//! whole IAU orientation model, nutation terms included.
//!
//! Every matrix and derivative entry must equal the CSPICE reference.

use std::path::PathBuf;

use stkcraft_geocalc::Pck;

fn pck_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("STKCRAFT_PCK") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let default = PathBuf::from("target/kernels/pck00011.tpc");
    if default.exists() {
        Some(default)
    } else {
        None
    }
}

#[test]
fn matches_cspice_sxform() {
    let Some(path) = pck_path() else {
        eprintln!("skipping body-state golden test: place pck00011.tpc in target/kernels");
        return;
    };

    let pck = Pck::open(&path).expect("open text PCK");
    let csv = include_str!("golden/body_state_rotations.csv");

    let mut worst_rotation: f64 = 0.0;
    let mut worst_derivative: f64 = 0.0;
    let mut worst_label = String::new();
    let mut checked = 0;

    for line in csv.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("et,") {
            continue;
        }

        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 21 {
            continue;
        }

        let et: f64 = fields[0].parse().unwrap();
        let body: i32 = fields[1].parse().unwrap();
        let expected: Vec<f64> = fields[3..21]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();

        let (rotation, derivative) = pck
            .body_state_rotation(body, et)
            .expect("body has an IAU orientation model");

        for row in 0..3 {
            for column in 0..3 {
                let index = row * 3 + column;
                let rotation_error = (rotation[row][column] - expected[index]).abs();
                if rotation_error > worst_rotation {
                    worst_rotation = rotation_error;
                    worst_label = format!(
                        "et={et} body={body} m[{row}][{column}] ours={} cspice={}",
                        rotation[row][column], expected[index]
                    );
                }
                let derivative_error = (derivative[row][column] - expected[9 + index]).abs();
                worst_derivative = worst_derivative.max(derivative_error);
            }
        }
        checked += 1;
    }

    assert!(checked > 0, "no golden rows were checked");

    println!(
        "vs CSPICE sxform over {checked} body-fixed frames:\n  rotation: {worst_rotation:.3e} absolute\n  derivative: {worst_derivative:.3e} absolute (rad/s)"
    );
    println!("  worst rotation element: {worst_label}");

    assert!(
        worst_rotation == 0.0,
        "rotation error {worst_rotation:e} is too large"
    );
    assert!(
        worst_derivative == 0.0,
        "rotation derivative error {worst_derivative:e} is too large"
    );
}
