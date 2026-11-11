//! Pointing direction against CSPICE `spkezp`.
//!
//! `tests/golden/pointing.csv` holds unit vectors in J2000.

use std::path::PathBuf;

use stkcraft_geocalc::{pointing_direction, Aberration, SpkKernel};

const POINTING_CSV: &str = include_str!("golden/pointing.csv");

fn kernel_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("STKCRAFT_SPK") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let default = PathBuf::from("target/kernels/de440s.bsp");
    default.exists().then_some(default)
}

#[test]
fn matches_cspice_spkezp_direction() {
    let Some(spk) = kernel_path() else {
        eprintln!("skipping pointing golden test: SPK is not present");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let mut worst = 0.0f64;
    let mut worst_label = String::new();
    let mut checked = 0;

    for line in POINTING_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("target,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 7 {
            continue;
        }
        let target: i32 = fields[0].parse().unwrap();
        let observer: i32 = fields[1].parse().unwrap();
        let et: f64 = fields[2].parse().unwrap();
        let abcorr = Aberration::parse(fields[3]).unwrap();
        let expected = [
            fields[4].parse::<f64>().unwrap(),
            fields[5].parse::<f64>().unwrap(),
            fields[6].parse::<f64>().unwrap(),
        ];

        let direction = pointing_direction(&kernel, target, observer, et, abcorr, None, "J2000")
            .unwrap_or_else(|error| panic!("pointing_direction {et} {}: {error}", fields[3]));

        for axis in 0..3 {
            assert_eq!(direction[axis].to_bits(), expected[axis].to_bits(),
                "SPKEZP/VHAT et={et} target={target} observer={observer} correction={} axis {axis}: {:?} vs {:?}", fields[3], direction[axis], expected[axis]);
            let difference = (direction[axis] - expected[axis]).abs();
            if difference > worst {
                worst = difference;
                worst_label = format!(
                    "et={et} target={target} observer={observer} abcorr={} axis {axis}",
                    fields[3]
                );
            }
        }
        checked += 1;
    }

    assert!(checked > 0, "no pointing rows");
    println!("vs CSPICE spkezp over {checked} directions: worst {worst:.3e} ({worst_label})");
    assert_eq!(worst, 0.0, "pointing direction must equal CSPICE exactly");
}
