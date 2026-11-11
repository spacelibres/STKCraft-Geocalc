//! Geometry Finder windows against CSPICE.
//!
//! `tests/golden/gf_events.csv` holds `gfdist`, `gfrr`, `gfpa`,
//! `gfilum`, `gfposc`, `gfsubc`, and `gfsntc` result windows. Each row
//! is one interval; local and absolute extrema are zero-width.

use std::collections::BTreeMap;
use std::path::PathBuf;

use stkcraft_geocalc::{
    find_distance, find_illumination_angle, find_phase_angle, find_position_coordinate_with_radii,
    find_range_rate, find_sub_point_coordinate, find_surface_intercept_ray_coordinate,
    planetodetic_to_rectangular, Condition, CoordinateSystem, FinderOptions, Frames,
    IlluminationAngle, Interval, Pck, SpkKernel, SubPointKind,
};

const EVENTS_CSV: &str = include_str!("golden/gf_events.csv");
const START: f64 = 0.0;
const END: f64 = 30.0 * 86400.0;
const STEP: f64 = 3600.0;
const TOLERANCE: f64 = 1.0e-6;

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

#[test]
fn exact_cspice_geometry_finders() {
    let (Some(spk), Some(pck_path)) = (kernel_path(), pck_path()) else {
        eprintln!("skipping Geometry Finder golden test: kernels are not present");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let pck = Pck::open(&pck_path).expect("open PCK");
    let frames = Frames::new();

    // (family, cases, relate, refval, adjust) -> expected windows
    let mut groups: BTreeMap<(String, String, String, String, String), Vec<Interval>> =
        BTreeMap::new();
    for line in EVENTS_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("family,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 7 {
            continue;
        }
        let key = (
            fields[0].to_string(),
            fields[1].to_string(),
            fields[2].to_string(),
            fields[3].to_string(),
            fields[6].to_string(),
        );
        groups.entry(key).or_default().push(Interval {
            start: fields[4].parse().unwrap(),
            end: fields[5].parse().unwrap(),
        });
    }

    let long_window = [Interval {
        start: START,
        end: END,
    }];
    let short_window = [Interval {
        start: 0.0,
        end: 300.0,
    }];
    let options = FinderOptions {
        step: STEP,
        tolerance: TOLERANCE,
        adjustment: 0.0,
    };

    let mut worst = 0.0f64;
    let mut worst_label = String::new();
    let mut compared = 0;
    let mut mismatches = Vec::new();

    for ((family, cases, relate, reference, adjustment), mut expected) in groups {
        let reference: f64 = reference.parse().unwrap();
        let adjustment: f64 = adjustment.parse().unwrap();
        let condition = Condition::parse(&relate).expect("condition");
        let windows: &[Interval] = if family == "intercept" {
            &short_window
        } else {
            &long_window
        };
        let options = if family == "intercept" {
            FinderOptions {
                step: 10.0,
                adjustment,
                ..options
            }
        } else {
            FinderOptions {
                adjustment,
                ..options
            }
        };
        let found = match family.as_str() {
            "distance" => find_distance(
                &kernel,
                301,
                399,
                stkcraft_geocalc::Aberration::None,
                windows,
                condition,
                reference,
                options,
            ),
            "range_rate" => find_range_rate(
                &kernel,
                301,
                399,
                stkcraft_geocalc::Aberration::None,
                windows,
                condition,
                reference,
                options,
            ),
            "phase" => find_phase_angle(
                &kernel,
                301,
                10,
                399,
                stkcraft_geocalc::Aberration::None,
                windows,
                condition,
                reference,
                options,
            ),
            "illum" => {
                let angle = if cases.starts_with("INCIDENCE") {
                    IlluminationAngle::Incidence
                } else if cases.starts_with("EMISSION") {
                    IlluminationAngle::Emission
                } else {
                    IlluminationAngle::Phase
                };
                let radii = pck.radii(301).unwrap();
                let point = planetodetic_to_rectangular(
                    20.0f64.to_radians(),
                    10.0f64.to_radians(),
                    0.0,
                    radii,
                )
                .unwrap();
                find_illumination_angle(
                    &kernel,
                    &pck,
                    301,
                    399,
                    stkcraft_geocalc::Aberration::None,
                    point,
                    angle,
                    windows,
                    condition,
                    reference,
                    options,
                )
            }
            "position" => {
                let system = if cases.contains("RECTANGULAR") {
                    CoordinateSystem::Rectangular
                } else {
                    CoordinateSystem::Latitudinal
                };
                let coordinate = system
                    .coordinate_index(cases.split(' ').next_back().unwrap())
                    .unwrap();
                find_position_coordinate_with_radii(
                    &kernel,
                    &pck,
                    301,
                    399,
                    stkcraft_geocalc::Aberration::None,
                    &frames,
                    "J2000",
                    system,
                    coordinate,
                    windows,
                    condition,
                    reference,
                    options,
                )
            }
            "subpoint" => {
                let coordinate = CoordinateSystem::Planetographic
                    .coordinate_index(cases.split(' ').next_back().unwrap())
                    .unwrap();
                find_sub_point_coordinate(
                    &kernel,
                    &pck,
                    301,
                    399,
                    stkcraft_geocalc::Aberration::None,
                    SubPointKind::Observer,
                    CoordinateSystem::Planetographic,
                    coordinate,
                    windows,
                    condition,
                    reference,
                    options,
                )
            }
            "intercept" => {
                let direction = kernel.state(301, 399, 0.0).unwrap().position;
                let length = direction
                    .iter()
                    .map(|value| value * value)
                    .sum::<f64>()
                    .sqrt();
                let direction = [
                    direction[0] / length,
                    direction[1] / length,
                    direction[2] / length,
                ];
                let coordinate = CoordinateSystem::Planetographic
                    .coordinate_index(cases.split(' ').next_back().unwrap())
                    .unwrap();
                find_surface_intercept_ray_coordinate(
                    &kernel,
                    &pck,
                    301,
                    399,
                    stkcraft_geocalc::Aberration::None,
                    direction,
                    CoordinateSystem::Planetographic,
                    coordinate,
                    windows,
                    condition,
                    reference,
                    options,
                )
            }
            other => panic!("unexpected finder family {other}"),
        }
        .unwrap_or_else(|error| panic!("{family} {cases} {relate}: {error}"));

        expected.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        let mut found = found;
        found.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());

        assert_eq!(
            expected.len(),
            found.len(),
            "window count mismatch for {family} / {cases} / {relate} {reference}"
        );

        for (index, (expected, actual)) in expected.iter().zip(&found).enumerate() {
            let difference = (actual.start - expected.start)
                .abs()
                .max((actual.end - expected.end).abs());
            if difference != 0.0 {
                mismatches.push(format!("{family} / {cases} / {relate} {reference} window {index}: ours [{}, {}] cspice [{}, {}]",actual.start,actual.end,expected.start,expected.end));
            }
            if difference > worst {
                worst = difference;
                worst_label = format!(
                    "{family} / {cases} / {relate} {reference} window {index}: \
                     ours [{}, {}] cspice [{}, {}]",
                    actual.start, actual.end, expected.start, expected.end
                );
            }
            compared += 1;
        }
    }

    assert!(compared > 0, "no finder windows were compared");
    println!(
        "vs CSPICE Geometry Finder over {compared} windows: worst {worst:.3e} s ({worst_label})"
    );
    assert!(
        mismatches.is_empty(),
        "{} strict mismatches of {} windows:\n{}",
        mismatches.len(),
        compared,
        mismatches.join("\n")
    );
}
