use stkcraft_geocalc::{
    angular_velocity_vector, describe_with_units, matrix_to_axis_angle, matrix_to_quaternion,
    AngularUnits, AngularVelocityRepresentation, AngularVelocityUnits, FrameAxis,
    OrientationRepresentation,
};
fn axis(n: &str) -> FrameAxis {
    match n {
        "1" => FrameAxis::X,
        "2" => FrameAxis::Y,
        "3" => FrameAxis::Z,
        _ => panic!("axis"),
    }
}
#[test]
fn orientation_native_source_values_exact() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in include_str!("golden/orientation_exact.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let n = |i: usize| f[i].parse::<f64>().unwrap();
        let mut r = [[0.; 3]; 3];
        let mut d = r;
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = n(5 + i * 3 + j);
                d[i][j] = n(14 + i * 3 + j);
            }
        }
        let expected: Vec<_> = (23..f.len()).map(n).collect();
        let actual: Result<Vec<f64>, _> = match f[1] {
            "QUAT" => Ok(matrix_to_quaternion(&r).to_vec()),
            "AA" => {
                let (angle, axis) = matrix_to_axis_angle(&r);
                Ok([vec![angle], axis.to_vec()].concat())
            }
            "AV1" | "AV2" => Ok(angular_velocity_vector(&r, &d, f[1] == "AV1").to_vec()),
            "UNIT_AV1" | "UNIT_AV2" => describe_with_units(
                &r,
                &d,
                OrientationRepresentation::MatrixAllOneRow,
                if f[1] == "UNIT_AV1" {
                    AngularVelocityRepresentation::VectorInFrame1
                } else {
                    AngularVelocityRepresentation::VectorInFrame2
                },
                AngularUnits::Radians,
                AngularVelocityUnits::Unitary,
            )
            .map(|v| {
                assert!(!v.ids.contains(&"AV_MAG"));
                v.rows[0][9..].to_vec()
            }),
            "EULER" | "EULER_DEG_RAD" | "EULER_RAD_DEG" => {
                let (a1, a2, a3) = (axis(f[2]), axis(f[3]), axis(f[4]));
                describe_with_units(
                    &r,
                    &d,
                    OrientationRepresentation::EulerAngles {
                        axis1: a1,
                        axis2: a2,
                        axis3: a3,
                    },
                    AngularVelocityRepresentation::EulerAngleDerivatives {
                        axis1: a1,
                        axis2: a2,
                        axis3: a3,
                    },
                    if f[1] == "EULER_DEG_RAD" {
                        AngularUnits::Degrees
                    } else {
                        AngularUnits::Radians
                    },
                    if f[1] == "EULER_RAD_DEG" {
                        AngularVelocityUnits::DegreesPerSecond
                    } else {
                        AngularVelocityUnits::RadiansPerSecond
                    },
                )
                .map(|v| v.rows[0].clone())
            }
            _ => panic!("kind"),
        };
        match actual {
            Ok(actual) if actual == expected => {}
            other => failures.push(format!(
                "{} {} axes {:?}: {other:?} != {expected:?}",
                f[0],
                f[1],
                &f[2..5]
            )),
        }
        checked += 1;
    }
    assert!(checked > 1000);
    std::fs::write(
        "target/orientation-exact-mismatches.txt",
        failures.join("\n"),
    )
    .unwrap();
    assert!(
        failures.is_empty(),
        "{} / {checked} orientation rows differ:\n{}",
        failures.len(),
        failures
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
