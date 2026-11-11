//! Angular separation with `POINT` and `SPHERE` shapes against CSPICE.
//!
//! `tests/golden/separation_shapes.csv` holds `trgsep` values; a negative
//! value means the apparent disks overlap, which only the `SPHERE` model
//! can express. `tests/golden/separation_shape_events.csv` holds `gfsep`
//! local extrema for the same shapes, so the search is checked with the
//! shaped evaluator too.
//!
//! Kernels are not committed. The test is skipped when they are missing.

use std::collections::BTreeMap;
use std::path::PathBuf;

use stkcraft_geocalc::{
    angular_separation_shaped, search_separation_shaped, Aberration, AngularShape, EventKind, Pck,
    SearchOptions, SpkKernel,
};

const VALUES_CSV: &str = include_str!("golden/separation_shapes.csv");
const EVENTS_CSV: &str = include_str!("golden/separation_shape_events.csv");

const EVENT_STEP: f64 = 3600.0;
const EVENT_TOLERANCE: f64 = 1e-6;

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

fn kernels() -> Option<(SpkKernel, Pck)> {
    let spk = find("STKCRAFT_SPK", "target/kernels/de440s.bsp")?;
    let pck = find("STKCRAFT_PCK", "target/kernels/pck00011.tpc")?;
    Some((
        SpkKernel::open(&spk).expect("open SPK kernel"),
        Pck::open(&pck).expect("open text PCK"),
    ))
}

fn shape(text: &str) -> AngularShape {
    AngularShape::parse(text).expect("known shape")
}

#[test]
fn shaped_separation_matches_trgsep() {
    let Some((kernel, pck)) = kernels() else {
        eprintln!("skipping shaped-separation golden test: place de440s.bsp and pck00011.tpc in target/kernels");
        return;
    };

    let mut worst = 0.0f64;
    let mut worst_label = String::new();
    let mut checked = 0;

    for line in VALUES_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("et,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 8 {
            continue;
        }

        let et: f64 = fields[0].parse().unwrap();
        let observer: i32 = fields[1].parse().unwrap();
        let target1: i32 = fields[2].parse().unwrap();
        let target2: i32 = fields[3].parse().unwrap();
        let shape1 = shape(fields[4]);
        let shape2 = shape(fields[5]);
        let abcorr = Aberration::parse(fields[6]).unwrap();
        let expected: f64 = fields[7].parse().unwrap();

        let ours = angular_separation_shaped(
            &kernel, &pck, observer, target1, target2, et, abcorr, shape1, shape2,
        )
        .expect("shaped separation");

        let difference = (ours - expected).abs();
        if difference > worst {
            worst = difference;
            worst_label = format!(
                "et={et} observer={observer} {target1}/{target2} {shape1:?}/{shape2:?} {} ours={ours} cspice={expected}",
                fields[6]
            );
        }
        checked += 1;
    }

    assert!(checked > 0, "no golden rows were checked");
    println!("vs CSPICE trgsep over {checked} shaped separations: {worst:.3e} rad");
    println!("  worst: {worst_label}");

    assert!(worst < 1e-12, "shaped separation error {worst:e} rad");
}

#[test]
fn shaped_separation_events_match_gfsep() {
    let Some((kernel, pck)) = kernels() else {
        eprintln!("skipping shaped-separation event test: place de440s.bsp and pck00011.tpc in target/kernels");
        return;
    };

    // case key -> (expected minima, expected maxima)
    let mut cases: BTreeMap<String, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
    for line in EVENTS_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("observer,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 10 {
            continue;
        }
        let key = fields[..8].join(",");
        let et: f64 = fields[9].parse().unwrap();
        let entry = cases.entry(key).or_default();
        match fields[8] {
            "LOCMIN" => entry.0.push(et),
            "LOCMAX" => entry.1.push(et),
            other => panic!("unexpected event kind {other}"),
        }
    }
    assert!(!cases.is_empty(), "no golden events were checked");

    let mut worst = 0.0f64;
    let mut worst_label = String::new();
    let mut compared = 0;

    for (key, (mut minima, mut maxima)) in cases {
        let fields: Vec<&str> = key.split(',').collect();
        let observer: i32 = fields[0].parse().unwrap();
        let target1: i32 = fields[1].parse().unwrap();
        let target2: i32 = fields[2].parse().unwrap();
        let shape1 = shape(fields[3]);
        let shape2 = shape(fields[4]);
        let abcorr = Aberration::parse(fields[5]).unwrap();
        let start: f64 = fields[6].parse().unwrap();
        let end: f64 = fields[7].parse().unwrap();

        let found = search_separation_shaped(
            &kernel,
            &pck,
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
            shape1,
            shape2,
        )
        .expect("search shaped separation");

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
            let difference = (actual - expected).abs();
            if difference > worst {
                worst = difference;
                worst_label = format!("LOCMIN {key} ours={actual} cspice={expected}");
            }
            compared += 1;
        }
        for (expected, actual) in maxima.iter().zip(&found_maxima) {
            let difference = (actual - expected).abs();
            if difference > worst {
                worst = difference;
                worst_label = format!("LOCMAX {key} ours={actual} cspice={expected}");
            }
            compared += 1;
        }
    }

    println!("vs CSPICE gfsep over {compared} shaped extrema: {worst:.3e} s worst");
    println!("  worst: {worst_label}");

    assert!(worst < 1e-2, "shaped extremum time error {worst:e} s");
}
