//! Angular separation and Geometry Finder events against CSPICE.
//!
//! `tests/golden/separation.csv` holds `trgsep` values;
//! `tests/golden/separation_events.csv` holds `gfsep` local extrema.
//! The SPK is not committed.

use std::collections::BTreeMap;
use std::path::PathBuf;

use stkcraft_geocalc::{
    angular_separation, search_separation, Aberration, EventKind, SearchOptions, SpkKernel,
};

const SEPARATION_CSV: &str = include_str!("golden/separation.csv");
const EVENTS_CSV: &str = include_str!("golden/separation_events.csv");

const EVENT_STEP: f64 = 3600.0;
const EVENT_TOLERANCE: f64 = 1e-6;

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
fn matches_cspice_trgsep() {
    let Some(path) = kernel_path() else {
        eprintln!("skipping separation golden test: set STKCRAFT_SPK or run tools/fetch_spk.py");
        return;
    };

    let kernel = SpkKernel::open(&path).expect("open SPK kernel");
    let mut worst = 0.0f64;
    let mut worst_label = String::new();
    let mut checked = 0;

    for line in SEPARATION_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("et,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 6 {
            continue;
        }

        let et: f64 = fields[0].parse().unwrap();
        let observer: i32 = fields[1].parse().unwrap();
        let target1: i32 = fields[2].parse().unwrap();
        let target2: i32 = fields[3].parse().unwrap();
        let abcorr = Aberration::parse(fields[4]).unwrap();
        let expected: f64 = fields[5].parse().unwrap();

        let value = angular_separation(&kernel, observer, target1, target2, et, abcorr)
            .unwrap_or_else(|error| {
                panic!("angular_separation({observer}, {target1}, {target2}, {et}): {error}")
            });

        let difference = (value - expected).abs();
        assert_eq!(value.to_bits(), expected.to_bits(), "{line}");
        if difference > worst {
            worst = difference;
            worst_label = format!(
                "et={et} observer={observer} targets={target1},{target2} abcorr={}",
                fields[4]
            );
        }
        checked += 1;
    }

    assert!(checked > 0, "no golden rows were checked");
    println!("vs CSPICE trgsep over {checked} separations: worst {worst:.3e} rad ({worst_label})");
    assert_eq!(worst, 0.0, "angular separation differs from CSPICE");
}

#[test]
fn matches_cspice_gfsep_events() {
    let Some(path) = kernel_path() else {
        eprintln!("skipping GF event test: set STKCRAFT_SPK or run tools/fetch_spk.py");
        return;
    };

    let kernel = SpkKernel::open(&path).expect("open SPK kernel");

    // case key -> (expected minima, expected maxima)
    let mut cases: BTreeMap<String, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
    for line in EVENTS_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("observer,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 8 {
            continue;
        }
        let key = fields[..6].join(",");
        let et: f64 = fields[7].parse().unwrap();
        let entry = cases.entry(key).or_default();
        match fields[6] {
            "LOCMIN" => entry.0.push(et),
            "LOCMAX" => entry.1.push(et),
            other => panic!("unexpected event kind {other}"),
        }
    }

    let mut worst = 0.0f64;
    let mut worst_label = String::new();
    let mut compared = 0;

    for (key, (mut minima, mut maxima)) in cases {
        let fields: Vec<&str> = key.split(',').collect();
        let observer: i32 = fields[0].parse().unwrap();
        let target1: i32 = fields[1].parse().unwrap();
        let target2: i32 = fields[2].parse().unwrap();
        let abcorr = Aberration::parse(fields[3]).unwrap();
        let start: f64 = fields[4].parse().unwrap();
        let end: f64 = fields[5].parse().unwrap();

        let found = search_separation(
            &kernel,
            observer,
            target1,
            target2,
            abcorr,
            start,
            end,
            SearchOptions {
                step: EVENT_STEP,
                tolerance: EVENT_TOLERANCE,
                threshold: None,
            },
        )
        .expect("search separation");

        let mut found_minima: Vec<f64> = found
            .iter()
            .filter(|event| event.kind == EventKind::LocalMinimum)
            .map(|event| event.et)
            .collect();
        let mut found_maxima: Vec<f64> = found
            .iter()
            .filter(|event| event.kind == EventKind::LocalMaximum)
            .map(|event| event.et)
            .collect();

        minima.sort_by(|a, b| a.partial_cmp(b).unwrap());
        maxima.sort_by(|a, b| a.partial_cmp(b).unwrap());
        found_minima.sort_by(|a, b| a.partial_cmp(b).unwrap());
        found_maxima.sort_by(|a, b| a.partial_cmp(b).unwrap());

        assert_eq!(
            minima.len(),
            found_minima.len(),
            "minimum count mismatch for {key}"
        );
        assert_eq!(
            maxima.len(),
            found_maxima.len(),
            "maximum count mismatch for {key}"
        );

        for (expected, actual) in minima.iter().zip(&found_minima) {
            assert_eq!(actual.to_bits(), expected.to_bits(), "LOCMIN {key}");
            let difference = (actual - expected).abs();
            if difference > worst {
                worst = difference;
                worst_label = format!("LOCMIN {key} ours={actual} cspice={expected}");
            }
            if difference > 1e-3 {
                println!("  LOCMIN {key}: {difference:.3e} s");
            }
            compared += 1;
        }
        for (expected, actual) in maxima.iter().zip(&found_maxima) {
            assert_eq!(actual.to_bits(), expected.to_bits(), "LOCMAX {key}");
            let difference = (actual - expected).abs();
            if difference > worst {
                worst = difference;
                worst_label = format!("LOCMAX {key} ours={actual} cspice={expected}");
            }
            if difference > 1e-3 {
                println!("  LOCMAX {key}: {difference:.3e} s");
            }
            compared += 1;
        }
    }

    assert!(compared > 0, "no events were compared");
    println!("vs CSPICE gfsep over {compared} events: worst {worst:.3e} s ({worst_label})");
    assert_eq!(worst, 0.0, "event time differs from CSPICE ({worst_label})");
}
