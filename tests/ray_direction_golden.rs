//! Every WebGeocalc ray-direction type, resolved and then evaluated.
//!
//! `tests/golden/ray_directions.csv` records, for each specification, the
//! J2000 direction resolved independently in Python plus the CSPICE
//! intercept/tangent point computed from it. This test resolves the same
//! specification with `RayDirection` and compares both, so a resolver bug
//! and a geometry bug are separable.
//!
//! The synthetic kernel `tests/golden/fov_ellipsoid.tf` supplies a frame
//! whose `+Z` points at the Moon and instruments of all four FOV shapes.
//! Some rows are deliberate misses (an anti-vector, a frame axis pointing
//! away); those must report "no point" here too, so a silent fallback
//! cannot pass.

use std::path::PathBuf;

use stkcraft_geocalc::{
    resolve_ray_directions, surface_intercept_ray, tangent_point, Aberration, Fov, FrameAxis,
    Frames, Pck, RayDirection, SpkKernel,
};

const FOV_KERNEL: &str = include_str!("golden/fov_ellipsoid.tf");
const CSV: &str = include_str!("golden/ray_directions.csv");

const EARTH: i32 = 399;
const MOON: i32 = 301;

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
fn ray_directions_match_cspice() {
    let (Some(spk), Some(pck_path)) = (
        find("STKCRAFT_SPK", "target/kernels/de440s.bsp"),
        find("STKCRAFT_PCK", "target/kernels/pck00011.tpc"),
    ) else {
        eprintln!(
            "skipping ray-direction test: place de440s.bsp and pck00011.tpc in target/kernels"
        );
        return;
    };

    let kernel = SpkKernel::open(&spk).expect("open SPK kernel");
    let pck = Pck::open(&pck_path).expect("open text PCK");
    let frames = Frames::from_text(FOV_KERNEL).expect("parse synthetic FK");

    let mut hits = 0;
    let mut misses = 0;
    let mut worst_direction = 0.0f64;
    let mut worst_direction_label = String::new();
    let mut worst_point = 0.0f64;
    let mut worst_point_label = String::new();
    let mut failures: Vec<String> = Vec::new();

    for line in CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("method,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 17 {
            continue;
        }

        let method = fields[0];
        let et: f64 = fields[1].parse().unwrap();
        let kind = fields[2];
        let s1 = fields[3];
        let s2 = fields[4];
        let s3 = fields[5];
        let vector = [
            fields[6].parse::<f64>().unwrap(),
            fields[7].parse::<f64>().unwrap(),
            fields[8].parse::<f64>().unwrap(),
        ];
        let expected_direction = [
            fields[9].parse::<f64>().unwrap(),
            fields[10].parse::<f64>().unwrap(),
            fields[11].parse::<f64>().unwrap(),
        ];
        let expected_point = [
            fields[12].parse::<f64>().unwrap(),
            fields[13].parse::<f64>().unwrap(),
            fields[14].parse::<f64>().unwrap(),
        ];
        let found = fields[16] == "1";

        let instrument: i32 = match kind {
            "BORESIGHT" | "VECFOV" | "FOVBOUND" => s1.parse().unwrap(),
            _ => 1420001,
        };
        let fov = Fov::from_text(FOV_KERNEL, instrument).expect("parse instrument FOV");

        let direction = match kind {
            "OBJECT" => RayDirection::ToObject {
                object: s1.parse().unwrap(),
                anti: s2 == "true",
            },
            "AXIS" => RayDirection::FrameAxis {
                frame: s1.to_string(),
                axis: FrameAxis::parse(s2).expect("known axis"),
                anti: s3 == "true",
            },
            "VECTOR" => RayDirection::VectorInFrame {
                frame: s1.to_string(),
                vector,
                anti: s2 == "true",
                stellar_aberration: false,
            },
            "BORESIGHT" => RayDirection::InstrumentBoresight,
            "VECFOV" => RayDirection::VectorInFov { vector },
            "FOVBOUND" => RayDirection::InstrumentFovBoundaryVectors,
            other => panic!("unknown specification {other}"),
        };

        let resolved = resolve_ray_directions(
            &kernel,
            Some(&fov),
            &frames,
            &direction,
            EARTH,
            et,
            Aberration::None,
        )
        .expect("resolve direction");

        // FOV boundary rows name one boundary vector by index.
        let chosen = if kind == "FOVBOUND" {
            let index: usize = s2.parse().unwrap();
            resolved.get(index).copied().expect("boundary vector index")
        } else {
            resolved[0]
        };

        for axis in 0..3 {
            assert_eq!(
                chosen[axis].to_bits(),
                expected_direction[axis].to_bits(),
                "resolved {kind} {s1} {s2} {et} axis{axis}"
            );
            let difference = (chosen[axis] - expected_direction[axis]).abs();
            if difference > worst_direction {
                worst_direction = difference;
                worst_direction_label = format!("{kind} {s1} {s2} {et} axis{axis}");
            }
        }

        let point = if method == "sincpt" {
            surface_intercept_ray(&kernel, &pck, MOON, EARTH, et, Aberration::None, chosen)
                .map(|value| value.point)
        } else {
            tangent_point(&kernel, &pck, MOON, EARTH, et, Aberration::None, chosen)
                .map(|value| value.point)
        };

        match (found, point) {
            (true, Ok(actual)) => {
                hits += 1;
                for axis in 0..3 {
                    assert_eq!(
                        actual[axis].to_bits(),
                        expected_point[axis].to_bits(),
                        "{method} {kind} {s1} {s2} {et} axis{axis}"
                    );
                    let difference = (actual[axis] - expected_point[axis]).abs();
                    if difference > worst_point {
                        worst_point = difference;
                        worst_point_label =
                            format!("{method} {kind} s1={s1} s2={s2} et={et} axis {axis}");
                    }
                }
            }
            (false, Err(_)) => misses += 1,
            (true, Err(error)) => failures.push(format!(
                "{method} {kind} s1={s1} s2={s2} et={et}: expected a point, got {error}"
            )),
            (false, Ok(actual)) => failures.push(format!(
                "{method} {kind} s1={s1} s2={s2} et={et}: expected no point, got {actual:?}"
            )),
        }
    }

    println!(
        "vs CSPICE over {} rows ({hits} points, {misses} expected misses):\n  direction: {worst_direction:.3e} raw-vector component\n  point: {worst_point:.3e} km",
        hits + misses
    );
    println!("  direction worst: {worst_direction_label}");
    println!("  point worst:     {worst_point_label}");
    for label in failures.iter().take(10) {
        println!("  {label}");
    }

    assert!(failures.is_empty(), "{} rows disagree", failures.len());
    assert!(hits > 20 && misses > 5, "the battery degenerated");
    assert_eq!(
        worst_direction, 0.0,
        "resolved directions must equal CSPICE exactly"
    );
    assert_eq!(
        worst_point, 0.0,
        "intercept and tangent points must equal CSPICE exactly"
    );
}
