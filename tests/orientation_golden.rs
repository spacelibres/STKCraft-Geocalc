//! Orientation representations and angular velocity against CSPICE.
//!
//! The rotation matrices in `orientation_matrices.csv` come from CSPICE
//! `sxform`, so this test exercises the representation algebra alone: no
//! frame names, no calendars, no aberration.
//!
//! Oracles: `m2eul` for all twelve valid axis triples, `m2q` for the
//! quaternion, RAXISA for axis-angle, XF2RAV for angular velocity, and
//! XF2EUL for Euler-angle derivatives. Every value must compare exactly.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use stkcraft_geocalc::{
    angular_velocity_vector, euler_angles_with_rates, matrix_to_axis_angle, matrix_to_euler,
    matrix_to_quaternion, FrameAxis,
};

const MATRICES: &str = include_str!("golden/orientation_matrices.csv");
const VALUES: &str = include_str!("golden/orientation_values.csv");

fn axis(name: &str) -> FrameAxis {
    FrameAxis::parse(name).expect("known axis")
}

fn wrapping(a: f64, b: f64) -> f64 {
    let mut difference = (a - b) % (2.0 * PI);
    if difference > PI {
        difference -= 2.0 * PI;
    }
    if difference < -PI {
        difference += 2.0 * PI;
    }
    difference.abs()
}

#[test]
fn orientation_representations_match_cspice() {
    let mut matrices: BTreeMap<String, ([stkcraft_geocalc::Mat3; 1], ())> = BTreeMap::new();
    matrices.clear();

    let mut rotations: BTreeMap<String, stkcraft_geocalc::Mat3> = BTreeMap::new();
    let mut derivatives: BTreeMap<String, stkcraft_geocalc::Mat3> = BTreeMap::new();

    for line in MATRICES.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 20 {
            continue;
        }
        let case = fields[0].to_string();
        let numbers: Vec<f64> = fields[2..20]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();
        let mut rotation = [[0.0f64; 3]; 3];
        let mut derivative = [[0.0f64; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                rotation[row][column] = numbers[row * 3 + column];
                derivative[row][column] = numbers[9 + row * 3 + column];
            }
        }
        rotations.insert(case.clone(), rotation);
        derivatives.insert(case, derivative);
    }
    assert!(!rotations.is_empty());

    let mut worst_euler = 0.0f64;
    let mut worst_quaternion = 0.0f64;
    let mut worst_axis_angle = 0.0f64;
    let mut worst_omega = 0.0f64;
    let mut worst_rates = 0.0f64;
    let mut worst_rates_label = String::new();
    let mut checked = 0;

    for line in VALUES.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 10 {
            continue;
        }
        let case = fields[0];
        let kind = fields[2];
        let (Some(rotation), Some(derivative)) = (rotations.get(case), derivatives.get(case))
        else {
            continue;
        };
        let expected: Vec<f64> = fields[6..10]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();

        match kind {
            "EULER" => {
                let axes = (axis(fields[3]), axis(fields[4]), axis(fields[5]));
                let angles = matrix_to_euler(rotation, axes).expect("euler");
                for index in 0..3 {
                    worst_euler = worst_euler.max(wrapping(angles[index], expected[index]));
                }
            }
            "QUAT" => {
                let quaternion = matrix_to_quaternion(rotation);
                let direct: f64 = (0..4)
                    .map(|index| (quaternion[index] - expected[index]).abs())
                    .fold(0.0, f64::max);
                worst_quaternion = worst_quaternion.max(direct);
            }
            "AA" => {
                let (angle, unit) = matrix_to_axis_angle(rotation);
                let mut difference = (angle - expected[0]).abs();
                for index in 0..3 {
                    difference = difference.max((unit[index] - expected[1 + index]).abs());
                }
                worst_axis_angle = worst_axis_angle.max(difference);
            }
            "OMEGA1" | "OMEGA2" => {
                let vector = angular_velocity_vector(rotation, derivative, kind == "OMEGA1");
                for index in 0..3 {
                    worst_omega = worst_omega.max((vector[index] - expected[index]).abs());
                }
            }
            "EULER_RATE" => {
                let axes = (axis(fields[3]), axis(fields[4]), axis(fields[5]));
                let (state, _) =
                    euler_angles_with_rates(rotation, derivative, axes).expect("rates");
                let rates = &state[3..];
                for index in 0..3 {
                    let difference = (rates[index] - expected[index]).abs();
                    if difference > worst_rates {
                        worst_rates = difference;
                        worst_rates_label = format!(
                            "{case} {:?}{:?}{:?} comp {index} ours={} cspice={}",
                            axes.0, axes.1, axes.2, rates[index], expected[index]
                        );
                    }
                }
            }
            other => panic!("unknown kind {other}"),
        }
        checked += 1;
    }

    println!("vs CSPICE over {checked} orientation values:");
    println!("  euler angles:        {worst_euler:.3e} rad");
    println!("  quaternion:          {worst_quaternion:.3e}");
    println!("  angle and axis:      {worst_axis_angle:.3e} (rad / unit vector)");
    println!("  angular velocity:    {worst_omega:.3e} rad/s");
    println!("  euler angle rates:   {worst_rates:.3e} rad/s");
    println!("  worst rate: {worst_rates_label}");

    assert!(checked > 100, "battery degenerated");
    assert!(worst_euler == 0.0, "euler angles {worst_euler:e} rad");
    assert!(worst_quaternion == 0.0, "quaternion {worst_quaternion:e}");
    assert!(
        worst_axis_angle == 0.0,
        "angle and axis {worst_axis_angle:e}"
    );
    assert!(worst_omega == 0.0, "angular velocity {worst_omega:e}");
    assert!(worst_rates == 0.0, "euler rates {worst_rates:e}");
}
