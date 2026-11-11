//! Sub-observer and sub-solar points for both ellipsoid definitions,
//! against CSPICE `subpnt`/`subslr`.
//!
//! The two definitions coincide for a spheroid and differ for a triaxial
//! ellipsoid. The Earth is the load-bearing case; the Moon is the
//! degenerate control that proves the intercept path is not silently
//! reusing the near-point answer.
//!
//! Same-kernel CSPICE points and source VNORM ranges must agree exactly.

use std::path::PathBuf;

use stkcraft_geocalc::{
    sub_observer_point_with_method, sub_solar_point_with_method, Aberration, Pck, SpkKernel,
    SubPointMethod,
};

const CSV: &str = include_str!("golden/subpoint_methods.csv");

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
fn sub_point_methods_match_cspice() {
    let (Some(spk), Some(pck_path)) = (
        find("STKCRAFT_SPK", "target/kernels/de440s.bsp"),
        find("STKCRAFT_PCK", "target/kernels/pck00011.tpc"),
    ) else {
        eprintln!(
            "skipping sub-point method test: place de440s.bsp and pck00011.tpc in target/kernels"
        );
        return;
    };

    let kernel = SpkKernel::open(&spk).expect("open SPK kernel");
    let pck = Pck::open(&pck_path).expect("open text PCK");

    let mut worst_point = 0.0f64;
    let mut worst_uncorrected_point = 0.0f64;
    let mut worst_altitude = 0.0f64;
    let mut worst_altitude_label = String::new();
    let mut per_case: std::collections::BTreeMap<String, (f64, f64)> = Default::default();
    let mut worst_label = String::new();
    let mut separation = 0.0f64;
    let mut checked = 0;

    for line in CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("kind,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 10 {
            continue;
        }

        let kind = fields[0];
        let target: i32 = fields[1].parse().unwrap();
        let observer: i32 = fields[2].parse().unwrap();
        let et: f64 = fields[3].parse().unwrap();
        let abcorr = Aberration::parse(fields[4]).unwrap();
        let method = SubPointMethod::parse(fields[5]).expect("known method");
        let expected = [
            fields[6].parse::<f64>().unwrap(),
            fields[7].parse::<f64>().unwrap(),
            fields[8].parse::<f64>().unwrap(),
        ];
        let expected_altitude: f64 = fields[9].parse().unwrap();

        let point = if kind == "solar" {
            sub_solar_point_with_method(&kernel, &pck, target, et, abcorr, method)
        } else {
            sub_observer_point_with_method(&kernel, &pck, target, observer, et, abcorr, method)
        }
        .expect("sub-point");

        let mut difference = 0.0f64;
        for (axis, expected_axis) in expected.iter().enumerate() {
            difference = difference.max((point.point[axis] - expected_axis).abs());
        }
        let altitude_difference = (point.altitude - expected_altitude).abs();

        if difference > worst_point {
            worst_point = difference;
            worst_label = format!(
                "{kind} target={target} observer={observer} et={et} {} {:?} ours={:?} cspice={expected:?}",
                fields[4], method, point.point
            );
        }
        if altitude_difference > worst_altitude {
            worst_altitude = altitude_difference;
            worst_altitude_label = format!(
                "{kind} target={target} observer={observer} et={et} {} {:?}",
                fields[4], method
            );
        }

        if fields[4] == "NONE" {
            worst_uncorrected_point = worst_uncorrected_point.max(difference);
        }

        let entry = per_case
            .entry(format!("{kind} {}", fields[4]))
            .or_insert((0.0, 0.0));
        entry.0 = entry.0.max(difference);
        entry.1 = entry.1.max(altitude_difference);

        // How far apart the two definitions are for this row: near zero for
        // the Moon, kilometres for the Earth.
        if method == SubPointMethod::NearPoint {
            let other = if kind == "solar" {
                sub_solar_point_with_method(
                    &kernel,
                    &pck,
                    target,
                    et,
                    abcorr,
                    SubPointMethod::Intercept,
                )
            } else {
                sub_observer_point_with_method(
                    &kernel,
                    &pck,
                    target,
                    observer,
                    et,
                    abcorr,
                    SubPointMethod::Intercept,
                )
            }
            .expect("intercept sub-point");
            let mut gap = 0.0f64;
            for axis in 0..3 {
                gap = gap.max((point.point[axis] - other.point[axis]).abs());
            }
            separation = separation.max(gap);
        }

        checked += 1;
    }

    assert!(checked > 0, "no golden rows were checked");
    println!(
        "vs CSPICE over {checked} sub-points:\n  point: {worst_point:.3e} km\n  altitude: {worst_altitude:.3e} km\n  near-vs-intercept spread: {separation:.3} km"
    );
    println!("  worst: {worst_label}");
    println!("  worst altitude: {worst_altitude_label}");
    for (label, (point_error, altitude_error)) in &per_case {
        println!("    {label:<24} point {point_error:.3e} km  altitude {altitude_error:.3e} km");
    }

    assert!(
        separation > 1.0,
        "the two definitions never diverged, so the test proves nothing"
    );
    assert_eq!(
        worst_uncorrected_point, 0.0,
        "exact CSPICE worst_uncorrected_point"
    );
    assert_eq!(worst_point, 0.0, "exact CSPICE worst_point");
    assert_eq!(worst_altitude, 0.0, "exact CSPICE worst_altitude");
}
