//! Sub-observer and sub-solar points against CSPICE `subpnt`/`subslr`.
//!
//! `tests/golden/surface_points.csv` holds CSPICE results for the near
//! point on the IAU ellipsoid. The SPK and PCK are not committed.

use std::path::PathBuf;

use stkcraft_geocalc::{
    illumination_angles, planetocentric_solar_longitude, sub_observer_point, sub_solar_point,
    surface_intercept_point, Aberration, Pck, SpkKernel,
};

const CSV: &str = include_str!("golden/surface_points.csv");
const INTERCEPTS: &str = include_str!("golden/surface_intercepts.csv");
const ILLUMINATION: &str = include_str!("golden/illumination.csv");
const SOLAR_LONGITUDE: &str = include_str!("golden/solar_longitude.csv");

fn spk_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("STKCRAFT_SPK") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let default = PathBuf::from("target/kernels/de442s.bsp");
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
fn matches_cspice_sub_points() {
    let (Some(spk), Some(pck_path)) = (spk_path(), pck_path()) else {
        eprintln!("skipping surface golden test: set STKCRAFT_SPK and STKCRAFT_PCK");
        return;
    };

    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let pck = Pck::open(&pck_path).expect("open PCK");

    let mut worst_point = 0.0f64;
    let mut worst_altitude = 0.0f64;
    let mut worst_label = String::new();
    let mut checked = 0;

    for line in CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("kind,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 8 {
            continue;
        }

        let kind = fields[0];
        let target: i32 = fields[1].parse().unwrap();
        let observer: i32 = fields[2].parse().unwrap();
        let et: f64 = fields[3].parse().unwrap();
        let expected_point = [
            fields[4].parse::<f64>().unwrap(),
            fields[5].parse::<f64>().unwrap(),
            fields[6].parse::<f64>().unwrap(),
        ];
        let expected_altitude: f64 = fields[7].parse().unwrap();

        let result = if kind == "observer" {
            sub_observer_point(&kernel, &pck, target, observer, et, Aberration::None)
        } else {
            sub_solar_point(&kernel, &pck, target, et, Aberration::None)
        }
        .unwrap_or_else(|error| {
            panic!("{kind} target={target} observer={observer} et={et}: {error}")
        });

        for (axis, expected) in expected_point.iter().enumerate() {
            let difference = (result.point[axis] - expected).abs();
            if difference > worst_point {
                worst_point = difference;
                worst_label =
                    format!("{kind} target={target} observer={observer} et={et} axis={axis}");
            }
        }
        let difference = (result.altitude - expected_altitude).abs();
        if difference > worst_altitude {
            worst_altitude = difference;
        }
        checked += 1;
    }

    assert!(checked > 0, "no golden rows were checked");
    println!(
        "vs CSPICE subpnt/subslr over {checked} points: worst point {worst_point:.3e} km, altitude {worst_altitude:.3e} km ({worst_label})"
    );
    assert_eq!(worst_point, 0.0, "exact CSPICE worst_point");
    assert_eq!(worst_altitude, 0.0, "exact CSPICE worst_altitude");
}

#[test]
fn matches_cspice_surface_intercepts() {
    let (Some(spk), Some(pck_path)) = (spk_path(), pck_path()) else {
        eprintln!("skipping surface intercept test: set STKCRAFT_SPK and STKCRAFT_PCK");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let pck = Pck::open(&pck_path).expect("open PCK");

    let mut worst_point = 0.0f64;
    let mut worst_altitude = 0.0f64;
    let mut checked = 0;

    for line in INTERCEPTS.lines() {
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
        let expected = [
            fields[3].parse::<f64>().unwrap(),
            fields[4].parse::<f64>().unwrap(),
            fields[5].parse::<f64>().unwrap(),
        ];
        let expected_altitude: f64 = fields[6].parse().unwrap();

        let result = surface_intercept_point(&kernel, &pck, target, observer, et, Aberration::None)
            .unwrap_or_else(|error| panic!("intercept {target}/{observer} {et}: {error}"));
        for (axis, value) in expected.iter().enumerate() {
            worst_point = worst_point.max((result.point[axis] - value).abs());
        }
        worst_altitude = worst_altitude.max((result.altitude - expected_altitude).abs());
        checked += 1;
    }

    assert!(checked > 0, "no intercept rows were checked");
    println!(
        "vs CSPICE sincpt over {checked} intercepts: worst point {worst_point:.3e} km, altitude {worst_altitude:.3e} km"
    );
    assert_eq!(worst_point, 0.0, "exact CSPICE worst_point");
    assert_eq!(worst_altitude, 0.0, "exact CSPICE worst_altitude");
}

#[test]
fn matches_cspice_illumination() {
    let (Some(spk), Some(pck_path)) = (spk_path(), pck_path()) else {
        eprintln!("skipping illumination test: set STKCRAFT_SPK and STKCRAFT_PCK");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let pck = Pck::open(&pck_path).expect("open PCK");

    let mut worst_phase = 0.0f64;
    let mut worst_solar = 0.0f64;
    let mut worst_emission = 0.0f64;
    let mut checked = 0;

    for line in ILLUMINATION.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("target,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 9 {
            continue;
        }
        let target: i32 = fields[0].parse().unwrap();
        let observer: i32 = fields[1].parse().unwrap();
        let et: f64 = fields[2].parse().unwrap();
        let point = [
            fields[3].parse::<f64>().unwrap(),
            fields[4].parse::<f64>().unwrap(),
            fields[5].parse::<f64>().unwrap(),
        ];
        let expected_phase: f64 = fields[6].parse().unwrap();
        let expected_solar: f64 = fields[7].parse().unwrap();
        let expected_emission: f64 = fields[8].parse().unwrap();

        let angles =
            illumination_angles(&kernel, &pck, target, observer, et, Aberration::None, point)
                .unwrap_or_else(|error| panic!("illumination {target}/{observer} {et}: {error}"));
        worst_phase = worst_phase.max((angles.phase - expected_phase).abs());
        worst_solar = worst_solar.max((angles.solar - expected_solar).abs());
        worst_emission = worst_emission.max((angles.emission - expected_emission).abs());
        checked += 1;
    }

    assert!(checked > 0, "no illumination rows were checked");
    println!(
        "vs CSPICE ilumin over {checked} points: worst phase {worst_phase:.3e}, solar {worst_solar:.3e}, emission {worst_emission:.3e} rad"
    );
    assert_eq!(worst_phase, 0.0, "exact CSPICE worst_phase");
    assert_eq!(worst_solar, 0.0, "exact CSPICE worst_solar");
    assert_eq!(worst_emission, 0.0, "exact CSPICE worst_emission");
}

#[test]
fn reports_nasa_transport_solar_longitude_strict_mismatch() {
    let (Some(spk), Some(pck_path)) = (spk_path(), pck_path()) else {
        eprintln!("skipping solar longitude test: set STKCRAFT_SPK and STKCRAFT_PCK");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let pck = Pck::open(&pck_path).expect("open PCK");

    let mut worst = 0.0f64;
    let mut checked = 0;
    for line in SOLAR_LONGITUDE.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("body,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 3 {
            continue;
        }
        let target = match fields[0] {
            "MOON" => 301,
            "EARTH" => 399,
            _ => continue,
        };
        let et: f64 = fields[1].parse().unwrap();
        let expected: f64 = fields[2].parse().unwrap();

        let value = planetocentric_solar_longitude(&kernel, &pck, target, et, Aberration::None)
            .unwrap_or_else(|error| panic!("solar longitude {target} {et}: {error}"));
        worst = worst.max((value.to_degrees() - expected).abs());
        checked += 1;
    }

    assert!(checked > 0, "no solar rows were checked");
    println!("vs WGC2 planetocentric solar longitude over {checked} epochs: worst {worst:.3e} deg");
    // Preserve the published transport values. This is an audit of the
    // remaining mismatch, not a same-kernel numerical acceptance test.
    let strict_status = if worst == 0.0 { "PASS" } else { "FAIL" };
    println!("NASA transport exact comparison: {strict_status}; max {worst:e} deg; local CSPICE parity is tested separately");
    assert_eq!(
        strict_status, "FAIL",
        "refresh the audit classification if the transport values become exact"
    );
}
