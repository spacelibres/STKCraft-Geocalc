//! Ellipsoid-target FOV visibility against CSPICE `fovtrg`.
//!
//! `tests/golden/fov_ellipsoid.tf` holds synthetic instruments whose
//! boresights point at the target, with the half-angle swept across the
//! target's apparent angular radius. That produces cases well outside,
//! well inside, and straddling the visibility boundary for all four FOV
//! shapes.

use std::path::PathBuf;

use stkcraft_geocalc::{ellipsoid_in_fov, Aberration, Fov, Frames, Pck, SpkKernel};

const KERNEL: &str = include_str!("golden/fov_ellipsoid.tf");
const CSV: &str = include_str!("golden/fov_ellipsoid.csv");

fn find(variable: &str, default: &str) -> Option<PathBuf> {
    if let Ok(value) = std::env::var(variable) {
        let path = PathBuf::from(value);
        if path.exists() {
            return Some(path);
        }
    }
    let path = PathBuf::from(default);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

#[test]
fn ellipsoid_visibility_matches_fovtrg() {
    let (Some(spk), Some(pck_path)) = (
        find("STKCRAFT_SPK", "target/kernels/de440s.bsp"),
        find("STKCRAFT_PCK", "target/kernels/pck00011.tpc"),
    ) else {
        eprintln!(
            "skipping ellipsoid FOV test: place de440s.bsp and pck00011.tpc in target/kernels"
        );
        return;
    };

    let kernel = SpkKernel::open(&spk).expect("open SPK kernel");
    let pck = Pck::open(&pck_path).expect("open text PCK");
    let frames = Frames::from_text(KERNEL).expect("parse synthetic FK");

    let mut checked = 0;
    let mut visible = 0;
    let mut mismatches = Vec::new();

    for line in CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("instrument,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 7 {
            continue;
        }

        let instrument: i32 = fields[0].parse().unwrap();
        let et: f64 = fields[1].parse().unwrap();
        let observer: i32 = fields[2].parse().unwrap();
        let target: i32 = fields[3].parse().unwrap();
        let abcorr = Aberration::parse(fields[4]).unwrap();
        let expected = fields[6] == "1";

        let fov = Fov::from_text(KERNEL, instrument).expect("parse instrument FOV");
        let ours = ellipsoid_in_fov(&kernel, &pck, &fov, target, observer, et, abcorr, &frames)
            .expect("ellipsoid visibility");

        if expected {
            visible += 1;
        }
        if ours != expected {
            mismatches.push(format!(
                "instrument {instrument} et={et} obs={observer} tgt={target} {} ours={ours} cspice={expected}",
                fields[4]
            ));
        }
        checked += 1;
    }

    assert!(checked > 0, "no golden rows were checked");
    println!(
        "vs CSPICE fovtrg(ELLIPSOID) over {checked} cases, {visible} visible: {} mismatches",
        mismatches.len()
    );
    for label in mismatches.iter().take(10) {
        println!("  {label}");
    }

    assert!(
        mismatches.is_empty(),
        "{} of {checked} visibility flags disagree with CSPICE",
        mismatches.len()
    );
}
