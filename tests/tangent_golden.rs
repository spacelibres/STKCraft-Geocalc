//! Tangent point against CSPICE `tangpt`.
//!
//! `tests/golden/tangent.csv` holds `tangpt` results with the direction
//! expressed in J2000 and the output in `IAU_MOON`. The SPK and PCK are
//! not committed.

use std::path::PathBuf;

use stkcraft_geocalc::{tangent_point, Aberration, Pck, SpkKernel};

const TANGENT_CSV: &str = include_str!("golden/tangent.csv");

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
fn matches_cspice_tangpt() {
    let (Some(spk), Some(pck_path)) = (kernel_path(), pck_path()) else {
        eprintln!("skipping tangent golden test: kernels are not present");
        return;
    };
    let kernel = SpkKernel::open(&spk).expect("open SPK");
    let pck = Pck::open(&pck_path).expect("open PCK");

    let mut worst_point = 0.0f64;
    let mut worst_surface = 0.0f64;
    let mut worst_range = 0.0f64;
    let mut worst_geometric = 0.0f64;
    let mut worst_label = String::new();
    let mut checked = 0;

    for line in TANGENT_CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("target,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 16 {
            continue;
        }
        let target: i32 = fields[0].parse().unwrap();
        let observer: i32 = fields[1].parse().unwrap();
        let et: f64 = fields[2].parse().unwrap();
        let abcorr = Aberration::parse(fields[3]).unwrap();
        let direction = [
            fields[4].parse::<f64>().unwrap(),
            fields[5].parse::<f64>().unwrap(),
            fields[6].parse::<f64>().unwrap(),
        ];
        let expected_point = [
            fields[7].parse::<f64>().unwrap(),
            fields[8].parse::<f64>().unwrap(),
            fields[9].parse::<f64>().unwrap(),
        ];
        let expected_altitude: f64 = fields[10].parse().unwrap();
        let expected_range: f64 = fields[11].parse().unwrap();
        let expected_surface = [
            fields[12].parse::<f64>().unwrap(),
            fields[13].parse::<f64>().unwrap(),
            fields[14].parse::<f64>().unwrap(),
        ];

        let result = tangent_point(&kernel, &pck, target, observer, et, abcorr, direction)
            .unwrap_or_else(|error| panic!("tangent_point {et} {}: {error}", fields[3]));

        assert_eq!(
            result.altitude.to_bits(),
            expected_altitude.to_bits(),
            "altitude {et} {}",
            fields[3]
        );
        assert_eq!(
            result.epoch.to_bits(),
            fields[15].parse::<f64>().unwrap().to_bits(),
            "epoch"
        );
        for axis in 0..3 {
            assert_eq!(
                result.point[axis].to_bits(),
                expected_point[axis].to_bits(),
                "point"
            );
            assert_eq!(
                result.surface_point[axis].to_bits(),
                expected_surface[axis].to_bits(),
                "surface point"
            );
            worst_point = worst_point.max((result.point[axis] - expected_point[axis]).abs());
            worst_surface =
                worst_surface.max((result.surface_point[axis] - expected_surface[axis]).abs());
        }
        let range_difference = (result.range - expected_range).abs();
        if range_difference > worst_range {
            worst_range = range_difference;
        }
        let point_difference = result
            .point
            .iter()
            .zip(&expected_point)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);
        if fields[3] == "NONE" {
            worst_geometric = worst_geometric.max(point_difference);
        }
        if fields[3] != "NONE" && point_difference > 1e-6 {
            println!(
                "  et={et} abcorr={} alt ours={} cspice={} point diff {point_difference:.3e}",
                fields[3], result.altitude, expected_altitude
            );
        }

        worst_label = format!("et={et} abcorr={}", fields[3]);
        checked += 1;
    }

    assert!(checked > 0, "no tangent rows");
    println!(
        "vs CSPICE tangpt over {checked} rays: geometric {worst_geometric:.3e} km, \
         point {worst_point:.3e} km, surface {worst_surface:.3e} km, \
         range {worst_range:.3e} km ({worst_label})"
    );
    assert_eq!(worst_geometric, 0.0, "exact CSPICE worst_geometric");
    assert_eq!(worst_point, 0.0, "exact CSPICE worst_point");
    assert_eq!(worst_surface, 0.0, "exact CSPICE worst_surface");
    assert_eq!(worst_range, 0.0, "exact CSPICE worst_range");
}
