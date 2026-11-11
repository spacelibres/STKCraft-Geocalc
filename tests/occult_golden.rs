//! Occultation state and search windows against CSPICE.
//!
//! `occult_states.csv` holds `occult` codes; `occult_events.csv` holds
//! `gfoclt` windows for the Moon/Sun (partial), Earth/Sun (full), and
//! Venus/Sun (annular) geometries.

use std::collections::BTreeMap;

use std::path::PathBuf;
use stkcraft_geocalc::{
    find_occultation, occultation_state, Aberration, FinderOptions, Interval, OccultShape,
    OccultationState, OccultationType, Pck, SpkKernel,
};

const STATES_CSV: &str = include_str!("golden/occult_states.csv");
const EVENTS_CSV: &str = include_str!("golden/occult_events.csv");
const STEP: f64 = 600.0;
const DAY: f64 = 86400.0;

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

fn pck_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("STKCRAFT_PCK") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let default = PathBuf::from("target/kernels/pck00011.tpc");
    default.exists().then_some(default)
}

fn geometry(family: &str) -> (i32, i32, i32, f64, f64) {
    match family {
        "moon_sun" => (301, 10, 399, 0.0, 1200.0 * DAY),
        "earth_sun" => (399, 10, 301, 0.0, 2.0e6),
        "venus_sun" => (299, 10, 399, 1.39e8, 1.41e8),
        other => panic!("unexpected occultation family {other}"),
    }
}

fn body_id(name: &str) -> i32 {
    match name {
        "MOON" => 301,
        "SUN" => 10,
        "EARTH" => 399,
        "VENUS" => 299,
        other => panic!("unexpected body name {other}"),
    }
}

fn state_from_code(code: i32) -> OccultationState {
    match code.abs() {
        0 => OccultationState::None,
        1 => OccultationState::Partial,
        2 => OccultationState::Annular,
        3 => OccultationState::Full,
        other => panic!("unexpected occultation code {other}"),
    }
}

#[test]
fn matches_cspice_occult_states() {
    let (Some(spk), Some(pck_path)) = (kernel_path(), pck_path()) else {
        eprintln!("skipping occultation test: kernels are not present");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let pck = Pck::open(&pck_path).expect("open PCK");

    let mut checked = 0;
    for line in STATES_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("family,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 6 {
            continue;
        }
        let family = fields[0];
        let front = body_id(fields[1]);
        let back = body_id(fields[2]);
        let observer = body_id(fields[3]);
        let et: f64 = fields[4].parse().unwrap();
        let expected = state_from_code(fields[5].parse().unwrap());

        let actual = occultation_state(
            &kernel,
            &pck,
            front,
            OccultShape::Ellipsoid,
            back,
            OccultShape::Ellipsoid,
            observer,
            et,
            Aberration::LightTime,
        )
        .unwrap_or_else(|error| panic!("occultation_state {family} {et}: {error}"));

        assert_eq!(
            expected, actual,
            "occultation state mismatch for {family} at {et} (front {front}, back {back}, observer {observer})"
        );
        checked += 1;
    }
    assert!(checked > 0, "no occultation states");
    println!("vs CSPICE occult over {checked} states: exact");
}

#[test]
fn matches_cspice_point_shapes() {
    let (Some(spk), Some(pck_path)) = (kernel_path(), pck_path()) else {
        eprintln!("skipping occultation test: kernels are not present");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let pck = Pck::open(&pck_path).expect("open PCK");
    let et = 1_701_873.0;

    // CSPICE occult(EARTH, ELLIPSOID, ..., SUN, POINT, ..., LT, MOON, et)
    // returns 3: the Sun point is inside Earth's silhouette.
    let full = occultation_state(
        &kernel,
        &pck,
        399,
        OccultShape::Ellipsoid,
        10,
        OccultShape::Point,
        301,
        et,
        Aberration::LightTime,
    )
    .expect("point back");
    assert_eq!(full, OccultationState::Full);

    // CSPICE occult(EARTH, POINT, ..., SUN, ELLIPSOID, ..., LT, MOON, et)
    // returns 0: a point cannot occult an extended body.
    let none = occultation_state(
        &kernel,
        &pck,
        399,
        OccultShape::Point,
        10,
        OccultShape::Ellipsoid,
        301,
        et,
        Aberration::LightTime,
    )
    .expect("point front");
    assert_eq!(none, OccultationState::None);

    println!("vs CSPICE occult point shapes: exact");
}

#[test]
fn matches_cspice_gfoclt() {
    let (Some(spk), Some(pck_path)) = (kernel_path(), pck_path()) else {
        eprintln!("skipping occultation test: kernels are not present");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let pck = Pck::open(&pck_path).expect("open PCK");

    let mut groups: BTreeMap<(String, String), Vec<Interval>> = BTreeMap::new();
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
            .entry((fields[0].to_string(), fields[1].to_string()))
            .or_default()
            .push(Interval {
                start: fields[2].parse().unwrap(),
                end: fields[3].parse().unwrap(),
            });
    }

    let options = FinderOptions {
        step: STEP,
        tolerance: 1e-6,
        adjustment: 0.0,
    };

    let mut compared = 0;
    for ((family, kind), mut expected) in groups {
        let (front, back, observer, start, end) = geometry(&family);
        let occultation_type = OccultationType::parse(&kind).expect("occultation type");
        let mut found = find_occultation(
            &kernel,
            &pck,
            front,
            OccultShape::Ellipsoid,
            back,
            OccultShape::Ellipsoid,
            observer,
            Aberration::LightTime,
            occultation_type,
            &[Interval { start, end }],
            options,
        )
        .unwrap_or_else(|error| panic!("find_occultation {family} {kind}: {error}"));

        expected.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        found.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        assert_eq!(
            expected.len(),
            found.len(),
            "window count mismatch for {family} {kind}"
        );
        for (expected, actual) in expected.iter().zip(&found) {
            let difference = (actual.start - expected.start)
                .abs()
                .max((actual.end - expected.end).abs());
            assert!(
                difference < 1e-2,
                "{family} {kind}: ours [{}, {}] cspice [{}, {}]",
                actual.start,
                actual.end,
                expected.start,
                expected.end
            );
            compared += 1;
        }
    }
    assert!(compared > 0, "no occultation windows");
    println!("vs CSPICE gfoclt over {compared} windows: endpoints within 1e-2 s");
}
