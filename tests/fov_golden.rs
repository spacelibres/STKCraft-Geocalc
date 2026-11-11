//! Instrument FOV booleans and interval searches against CSPICE.
//!
//! Uses the synthetic FK `tests/golden/fov.tf`.
//! - `fov_ray.csv`: `fovray`
//! - `fov_target.csv`: `fovtrg` with POINT targets
//! - `fov_events.csv`: `gftfov` / `gfrfov`

use std::collections::BTreeMap;
use std::path::PathBuf;

use stkcraft_geocalc::{
    find_ray_in_fov, find_target_in_fov, ray_in_fov, target_in_fov, Aberration, FinderOptions, Fov,
    Frames, Interval, SpkKernel,
};

const FK: &str = include_str!("golden/fov.tf");
const RAY_CSV: &str = include_str!("golden/fov_ray.csv");
const TARGET_CSV: &str = include_str!("golden/fov_target.csv");
const EVENTS_CSV: &str = include_str!("golden/fov_events.csv");

const DAY: f64 = 86400.0;
const STEP: f64 = 3600.0;

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

fn load_fov(_frames: &Frames, instrument: i32) -> Fov {
    Fov::from_text(FK, instrument).expect("parse FOV")
}

#[test]
fn matches_cspice_fovray() {
    let Some(spk) = kernel_path() else {
        eprintln!("skipping FOV test: SPK is not present");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let frames = Frames::from_text(FK).expect("parse frames");

    let mut checked = 0;
    for line in RAY_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("instrument,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 6 {
            continue;
        }
        let instrument: i32 = fields[0].parse().unwrap();
        let et: f64 = fields[1].parse().unwrap();
        let direction = [
            fields[2].parse::<f64>().unwrap(),
            fields[3].parse::<f64>().unwrap(),
            fields[4].parse::<f64>().unwrap(),
        ];
        let expected = fields[5] == "1";
        let fov = load_fov(&frames, instrument);
        let actual = ray_in_fov(&kernel, &fov, 399, et, Aberration::None, direction, &frames)
            .unwrap_or_else(|error| panic!("ray_in_fov {instrument} {et}: {error}"));
        assert_eq!(
            expected, actual,
            "fovray mismatch for instrument {instrument} at {et} direction {direction:?}"
        );
        checked += 1;
    }
    assert!(checked > 0, "no FOV ray cases");
    println!("vs CSPICE fovray over {checked} rays: exact");
}

#[test]
fn matches_cspice_fovtrg() {
    let Some(spk) = kernel_path() else {
        eprintln!("skipping FOV test: SPK is not present");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let frames = Frames::from_text(FK).expect("parse frames");

    let mut checked = 0;
    for line in TARGET_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("instrument,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 4 {
            continue;
        }
        let instrument: i32 = fields[0].parse().unwrap();
        let et: f64 = fields[2].parse().unwrap();
        let expected = fields[3] == "1";
        let fov = load_fov(&frames, instrument);
        let actual = target_in_fov(&kernel, &fov, 301, 399, et, Aberration::None, &frames)
            .unwrap_or_else(|error| panic!("target_in_fov {instrument} {et}: {error}"));
        assert_eq!(
            expected, actual,
            "fovtrg mismatch for instrument {instrument} at {et}"
        );
        checked += 1;
    }
    assert!(checked > 0, "no FOV target cases");
    println!("vs CSPICE fovtrg over {checked} targets: exact");
}

#[test]
fn matches_cspice_fov_finders() {
    let Some(spk) = kernel_path() else {
        eprintln!("skipping FOV event test: SPK is not present");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let frames = Frames::from_text(FK).expect("parse frames");

    let mut groups: BTreeMap<(String, i32), Vec<Interval>> = BTreeMap::new();
    for line in EVENTS_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("family,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 4 {
            continue;
        }
        groups
            .entry((fields[0].to_string(), fields[1].parse().unwrap()))
            .or_default()
            .push(Interval {
                start: fields[2].parse().unwrap(),
                end: fields[3].parse().unwrap(),
            });
    }

    let windows = [Interval {
        start: 0.0,
        end: 30.0 * DAY,
    }];
    let options = FinderOptions {
        step: STEP,
        tolerance: 1e-6,
        adjustment: 0.0,
    };

    let mut compared = 0;
    for ((family, instrument), mut expected) in groups {
        let fov = load_fov(&frames, instrument);
        let mut found = if family == "target" {
            find_target_in_fov(
                &kernel,
                &fov,
                301,
                399,
                Aberration::None,
                &frames,
                &windows,
                options,
            )
            .expect("target finder")
        } else {
            find_ray_in_fov(
                &kernel,
                &fov,
                399,
                Aberration::None,
                [0.0, 0.0, 1.0],
                &frames,
                &windows,
                options,
            )
            .expect("ray finder")
        };

        expected.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        found.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        assert_eq!(
            expected.len(),
            found.len(),
            "window count mismatch for {family} instrument {instrument}"
        );
        for (expected, actual) in expected.iter().zip(&found) {
            assert!(
                actual.start == expected.start && actual.end == expected.end,
                "{family} instrument {instrument}: ours [{}, {}] cspice [{}, {}]",
                actual.start,
                actual.end,
                expected.start,
                expected.end
            );
            compared += 1;
        }
    }
    assert!(compared > 0, "no FOV event windows");
    println!("vs CSPICE FOV finders over {compared} windows: endpoints exactly equal");
}
