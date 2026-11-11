//! Frame rotations against CSPICE `pxform`.
//!
//! `tests/golden/frames.tf` defines 46 `TKFRAME` rotations covering all
//! three specification forms, every angle unit, and multi-level chains.
//! `tests/golden/frame_rotations.csv` holds CSPICE `pxform` results for
//! every ordered pair; both files are committed and regenerated with
//! `tools/gen_frame_golden.py`.
//!
//! Every generated matrix element must equal the independent CSPICE value.

use std::path::Path;

use stkcraft_geocalc::Frames;

const KERNEL: &str = include_str!("golden/frames.tf");
const CSV: &str = include_str!("golden/frame_rotations.csv");

fn compare(frames: &Frames, from: &str, to: &str, expected: &[f64; 9]) -> f64 {
    let matrix = frames
        .pxform(from, to, 0.0)
        .unwrap_or_else(|error| panic!("pxform({from}, {to}): {error}"));

    let mut worst = 0.0f64;
    for row in 0..3 {
        for column in 0..3 {
            let difference = (matrix[row][column] - expected[row * 3 + column]).abs();
            worst = worst.max(difference);
        }
    }
    worst
}

#[test]
fn matches_cspice_pxform() {
    let frames = Frames::from_text(KERNEL).expect("parse golden frame kernel");

    let mut worst: f64 = 0.0;
    let mut worst_label = String::new();
    let mut checked = 0;

    for line in CSV.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("from,") {
            continue;
        }

        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 11, "malformed golden row: {line}");

        let from = fields[0];
        let to = fields[1];
        let mut expected = [0.0f64; 9];
        for (index, field) in fields[2..11].iter().enumerate() {
            expected[index] = field.parse().unwrap();
        }

        let difference = compare(&frames, from, to, &expected);
        if difference > worst {
            worst = difference;
            worst_label = format!("{from} -> {to}");
        }
        checked += 1;
    }

    assert_eq!(checked, 2256, "incomplete frame reference matrix");
    println!(
        "vs CSPICE pxform over {checked} transforms: worst element {worst:.3e} ({worst_label})"
    );

    assert!(
        worst == 0.0,
        "frame rotation differs from CSPICE by {worst:e} ({worst_label})"
    );
}

/// A real NAIF frame kernel, when available, exercises the exact data
/// shapes NAIF ships. `moon_080317.tf` defines `TKFRAME_31007_SPEC`
/// as `ANGLES` in arcseconds relative to the body-fixed frame
/// `MOON_PA_DE421`, and CSPICE can evaluate that one-hop rotation
/// without loading a binary PCK.
#[test]
#[allow(clippy::excessive_precision)]
fn matches_cspice_on_real_naif_kernel() {
    let path = Path::new("target/kernels/moon_080317.tf");
    if !path.exists() {
        eprintln!(
            "skipping real NAIF FK test: place moon_080317.tf at target/kernels/ \
             (https://naif.jpl.nasa.gov/pub/naif/generic_kernels/fk/satellites/)"
        );
        return;
    }

    let text = std::fs::read_to_string(path).expect("read moon FK");
    let frames = Frames::from_text(&text).expect("parse moon FK");

    // CSPICE pxform('MOON_ME_DE421', 'MOON_PA_DE421', 0.0).
    let expected = [
        0.999999873254714,
        0.000329286000210947,
        -0.000380869119096078,
        -0.00032928542237557117,
        0.9999999457843058,
        1.5798557868269077e-06,
        0.0003808696186713873,
        -1.4544409378362703e-06,
        0.9999999274681064,
    ];

    let worst = compare(&frames, "MOON_ME_DE421", "MOON_PA_DE421", &expected);
    println!("vs CSPICE on moon_080317.tf: worst element {worst:.3e}");
    assert!(
        worst == 0.0,
        "real NAIF frame differs from CSPICE by {worst:e}"
    );
}

#[test]
fn exact_cspice_body_and_fixed_frame_state_chains() {
    let mut frames = Frames::from_text(KERNEL).unwrap();
    frames
        .furnsh(include_str!("golden/frame_body_children.tf"))
        .unwrap();
    let pck = stkcraft_geocalc::Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/frame_state_rotations.csv")
        .lines()
        .skip(2)
    {
        let f: Vec<_> = line.split(',').collect();
        let actual = frames
            .pxform_with_derivative(f[0], f[1], f[2].parse().unwrap(), Some(&pck))
            .unwrap();
        for i in 0..6 {
            for j in 0..6 {
                let expected = f[3 + i * 6 + j].parse::<f64>().unwrap();
                let value = if i < 3 && j < 3 {
                    actual.rotation[i][j]
                } else if i >= 3 && j >= 3 {
                    actual.rotation[i - 3][j - 3]
                } else if i >= 3 {
                    actual.rotation_dot[i - 3][j]
                } else {
                    0.0
                };
                if value != expected {
                    errors.push(format!(
                        "{} {} {} [{i},{j}] {value:?}!={expected:?}",
                        f[0], f[1], f[2]
                    ));
                }
            }
        }
        count += 1;
    }
    assert!(
        errors.is_empty(),
        "{} component mismatches/{count} states:\n{}",
        errors.len(),
        errors
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
