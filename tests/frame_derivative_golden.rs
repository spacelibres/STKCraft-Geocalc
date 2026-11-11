//! Frame rotations and their analytic derivatives against CSPICE.
//!
//! `tests/golden/frame_derivative.csv` holds `sxform` results for fourteen
//! frame pairs, including body-fixed frames whose orientation rate comes
//! from `BODY<id>_POLE_*` and `_NUT_PREC_*` terms. The derivative is
//! analytic on both sides, so the comparison constrains the product rule
//! through the frame chain and the body orientation rate model, not a
//! difference quotient.
//!
//! # Residual level
//!
//! The static frames agree to `1e-16`, and the Moon cases to about
//! `2e-13`. Mars reaches `5e-12` in the rotation and `4e-16` per second in
//! the derivative at `et = 3e8`. That is not a model difference: the
//! prime-meridian angle of a fast rotator reaches `2e4` radians within a
//! decade of J2000, so a few units in the last place of its sine and
//! cosine are already `1e-11` in the rotation. Those elements are exactly
//! the ones that differ, while the pole-only column matches bit for bit.
//! The bounds below sit just above that measured floor.

use stkcraft_geocalc::{Frames, Mat3, Pck};

const GOLDEN: &str = include_str!("golden/frame_derivative.csv");

const PCK_PATHS: [&str; 2] = ["target/kernels/pck00011.tpc", "target/kernels/gm_de431.tpc"];

/// Maps a CSPICE frame name to the name this crate understands.
fn ours(name: &str) -> String {
    match name {
        "IAU_MOON" => "IAU_301".to_string(),
        "IAU_EARTH" => "IAU_399".to_string(),
        "IAU_MARS" => "IAU_499".to_string(),
        "IAU_SATURN" => "IAU_699".to_string(),
        other => other.to_string(),
    }
}

/// Largest absolute difference between a matrix and row-major golden values.
fn residual(matrix: &Mat3, expected: &[f64]) -> f64 {
    let mut worst: f64 = 0.0;
    for row in 0..3 {
        for column in 0..3 {
            worst = worst.max((matrix[row][column] - expected[row * 3 + column]).abs());
        }
    }
    worst
}

#[test]
fn frame_derivatives_match_cspice() {
    let mut pck = Pck::default();
    for path in PCK_PATHS {
        pck.furnsh(&std::fs::read_to_string(path).expect("kernel readable"))
            .expect("kernel parsed");
    }
    let frames = Frames::new();

    let mut checked = 0;
    let mut worst_rotation: f64 = 0.0;
    let mut worst_derivative: f64 = 0.0;

    for line in GOLDEN.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 22, "malformed golden row: {line}");

        let frame1 = fields[1];
        let frame2 = fields[2];
        let et: f64 = fields[3].parse().expect("epoch");
        let numbers: Vec<f64> = fields[4..22]
            .iter()
            .map(|value| value.parse().expect("value"))
            .collect();

        let rotation = frames
            .pxform_with_derivative(&ours(frame1), &ours(frame2), et, Some(&pck))
            .unwrap_or_else(|error| panic!("{frame1}->{frame2} at {et}: {error}"));

        worst_rotation = worst_rotation.max(residual(&rotation.rotation, &numbers[0..9]));
        worst_derivative = worst_derivative.max(residual(&rotation.rotation_dot, &numbers[9..18]));
        checked += 1;
    }

    assert_eq!(checked, 14, "every golden case must be exercised");
    assert!(
        worst_rotation < 1e-11,
        "rotation residual {worst_rotation:e} exceeds 1e-11"
    );
    assert!(
        worst_derivative < 1e-15,
        "derivative residual {worst_derivative:e} exceeds 1e-15 rad/s"
    );
}
