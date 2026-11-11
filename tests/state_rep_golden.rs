//! State representations against direct CSPICE SPKEZR and XFMSTA/DAZLDR.
//! No finite-difference reference or tolerance floor is accepted.

use std::path::PathBuf;

use stkcraft_geocalc::{
    state_columns, state_columns_with_pck, Aberration, FrameSource, Frames, Pck, SpkKernel,
    StateRepresentation,
};

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

struct Row {
    et: f64,
    target: i32,
    observer: i32,
    abcorr: Aberration,
    frame_code: i32,
    representation: StateRepresentation,
    expected: Vec<f64>,
}

fn load() -> Vec<Row> {
    let mut rows = Vec::new();
    for line in include_str!("golden/state_representations.csv").lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("et,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 9 {
            continue;
        }

        let az_ccw = fields[6] == "true";
        let el_plus_z = fields[7] == "true";
        let representation = match fields[5] {
            "AZ_EL" => StateRepresentation::azimuth_elevation(az_ccw, el_plus_z),
            other => StateRepresentation::parse(other).expect("known representation"),
        };

        rows.push(Row {
            et: fields[0].parse().unwrap(),
            target: fields[1].parse().unwrap(),
            observer: fields[2].parse().unwrap(),
            abcorr: Aberration::parse(fields[3]).unwrap(),
            frame_code: fields[4].parse().unwrap(),
            representation,
            expected: fields[8..]
                .iter()
                .map(|value| value.parse().unwrap())
                .collect(),
        });
    }
    rows
}

#[test]
fn matches_cspice_state_representations() {
    let (Some(spk), Some(pck_path)) = (
        find("STKCRAFT_SPK", "target/kernels/de442s.bsp"),
        find("STKCRAFT_PCK", "target/kernels/pck00011.tpc"),
    ) else {
        eprintln!("skipping state-representation golden test: place de442s.bsp and pck00011.tpc in target/kernels");
        return;
    };

    let kernel = SpkKernel::open(&spk).expect("open SPK kernel");
    let pck = Pck::open(&pck_path).expect("open text PCK");
    let frames = Frames::new();
    let rows = load();
    assert!(!rows.is_empty(), "no golden rows were loaded");

    let mut worst_value: f64 = 0.0;
    let mut worst_value_label = String::new();
    let mut worst_rate: f64 = 0.0;
    let mut worst_rate_label = String::new();
    let mut checked = 0;
    let mut failures = Vec::new();

    for row in &rows {
        let ecliptic;
        let source = if row.frame_code == 0 {
            FrameSource::J2000
        } else if row.frame_code == -1 {
            ecliptic = frames
                .pxform("J2000", "ECLIPJ2000", row.et)
                .expect("ECLIPJ2000 rotation");
            FrameSource::Fixed(&ecliptic)
        } else {
            FrameSource::BodyFixed {
                pck: &pck,
                body: row.frame_code,
            }
        };

        let apparent = kernel
            .apparent_state_in(row.target, row.observer, row.et, row.abcorr, &source)
            .expect("apparent state");

        let columns = if row.frame_code > 0 {
            state_columns_with_pck(
                row.et,
                apparent.state,
                row.representation,
                apparent.light_time,
                row.abcorr,
                &pck,
                row.frame_code,
            )
        } else {
            state_columns(
                row.et,
                apparent.state,
                row.representation,
                apparent.light_time,
                row.abcorr,
                None,
                row.frame_code,
            )
        }
        .expect("state columns");

        let ids = row.representation.output_ids();
        let actual = columns.numeric_row();
        assert_eq!(
            actual.len() + 1,
            ids.len(),
            "column count for {:?}",
            row.representation
        );
        assert_eq!(
            actual.len(),
            row.expected.len(),
            "golden column count for {:?}",
            row.representation
        );

        for (index, (ours, expected)) in actual.iter().zip(row.expected.iter()).enumerate() {
            let difference = (ours - expected).abs();
            let scale = expected.abs().max(1.0);
            let scaled = difference / scale;
            let identifier = ids[index + 1];
            let finite_difference_oracle = row.representation != StateRepresentation::Rectangular
                && identifier.ends_with("_DT");
            let label = format!(
                "et={} {:?} frame={} abcorr={:?} {} ours={} cspice={}",
                row.et, row.representation, row.frame_code, row.abcorr, identifier, ours, expected
            );

            if ours.to_bits() != expected.to_bits() {
                failures.push(label.clone());
            }
            if finite_difference_oracle {
                if scaled > worst_rate {
                    worst_rate = scaled;
                    worst_rate_label = label;
                }
            } else if scaled > worst_value {
                worst_value = scaled;
                worst_value_label = label;
            }
            checked += 1;
        }
    }

    println!("vs CSPICE over {checked} state-representation columns:");
    println!("  values: {worst_value:.3e} relative (exact equality required)");
    println!("  rates:  {worst_rate:.3e} relative (exact equality required)");
    println!("  worst value column: {worst_value_label}");
    println!("  worst rate column:  {worst_rate_label}");

    if !failures.is_empty() {
        std::fs::write(
            "target/state-representation-mismatches.txt",
            failures.join("\n"),
        )
        .unwrap();
    }
    assert!(
        failures.is_empty(),
        "{} exact failures; first {:#?}",
        failures.len(),
        &failures[..failures.len().min(10)]
    );
}
