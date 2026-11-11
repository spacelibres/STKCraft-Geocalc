//! Column layout and shape of the orientation output.
//!
//! The layouts were measured against the live WebGeocalc API:
//!
//! * a non-matrix orientation with `MATRIX` angular velocity drops the
//!   matrix columns entirely;
//! * a matrix orientation with `MATRIX` emits the 6x6 state transformation
//!   instead of the 3x3 rotation, in three, six, or one row;
//! * the angular velocity columns repeat on every row of the multi-row
//!   matrix forms;
//! * Euler angle rates are only accepted with `EULER_ANGLES`.

use stkcraft_geocalc::{
    describe, AngularUnits, AngularVelocityRepresentation, FrameAxis, Mat3,
    OrientationRepresentation,
};

const GOLDEN: &str = include_str!("golden/frame_derivative.csv");

/// The `mars_j2000` case, whose rotation rate is the largest in the file.
fn mars_case() -> (Mat3, Mat3) {
    for line in GOLDEN.lines() {
        if !line.starts_with("mars_j2000,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        let numbers: Vec<f64> = fields[4..22]
            .iter()
            .map(|value| value.parse().expect("value"))
            .collect();
        let mut rotation = [[0.0f64; 3]; 3];
        let mut derivative = [[0.0f64; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                rotation[row][column] = numbers[row * 3 + column];
                derivative[row][column] = numbers[9 + row * 3 + column];
            }
        }
        return (rotation, derivative);
    }
    panic!("golden case missing");
}

fn euler(z: FrameAxis, y: FrameAxis, x: FrameAxis) -> OrientationRepresentation {
    OrientationRepresentation::EulerAngles {
        axis1: z,
        axis2: y,
        axis3: x,
    }
}

#[test]
fn matrix_velocity_is_dropped_for_angle_orientations() {
    let (rotation, derivative) = mars_case();
    let columns = describe(
        &rotation,
        &derivative,
        euler(FrameAxis::Z, FrameAxis::Y, FrameAxis::X),
        AngularVelocityRepresentation::Matrix,
        AngularUnits::Degrees,
    )
    .expect("describe");

    assert_eq!(columns.ids, ["ANGLE3", "ANGLE2", "ANGLE1"]);
    assert_eq!(columns.rows.len(), 1);
    assert_eq!(columns.rows[0].len(), 3);
}

#[test]
fn angular_velocity_repeats_on_every_matrix_row() {
    let (rotation, derivative) = mars_case();
    let columns = describe(
        &rotation,
        &derivative,
        OrientationRepresentation::MatrixRowByRow,
        AngularVelocityRepresentation::VectorInFrame1,
        AngularUnits::Degrees,
    )
    .expect("describe");

    assert_eq!(
        columns.ids,
        [
            "ROW_NUMBER",
            "M11",
            "M12",
            "M13",
            "AV_X",
            "AV_Y",
            "AV_Z",
            "AV_MAG"
        ]
    );
    assert_eq!(columns.rows.len(), 3);
    for (index, row) in columns.rows.iter().enumerate() {
        assert_eq!(row.len(), 8);
        assert_eq!(row[0], (index + 1) as f64);
        assert_eq!(row[4..], columns.rows[0][4..], "row {index} velocity");
    }
}

#[test]
fn state_matrix_matches_cspice_in_every_layout() {
    let (rotation, derivative) = mars_case();
    let layouts = [
        (OrientationRepresentation::MatrixRowByRow, 6, 7, true),
        (OrientationRepresentation::MatrixFlagged, 6, 6, false),
        (OrientationRepresentation::MatrixAllOneRow, 1, 36, false),
    ];

    for (orientation, row_count, width, numbered) in layouts {
        let columns = describe(
            &rotation,
            &derivative,
            orientation,
            AngularVelocityRepresentation::Matrix,
            AngularUnits::Degrees,
        )
        .expect("describe");
        assert_eq!(columns.rows.len(), row_count, "{orientation:?} rows");
        for row in &columns.rows {
            assert_eq!(row.len(), width, "{orientation:?} width");
        }

        for row in 0..6 {
            for column in 0..6 {
                let expected = match (row, column) {
                    (0..=2, 0..=2) => rotation[row][column],
                    (3..=5, 0..=2) => derivative[row - 3][column],
                    (3..=5, 3..=5) => rotation[row - 3][column - 3],
                    _ => 0.0,
                };
                let actual = match orientation {
                    OrientationRepresentation::MatrixAllOneRow => columns.rows[0][row * 6 + column],
                    _ => {
                        let offset = usize::from(numbered);
                        columns.rows[row][offset + column]
                    }
                };
                assert_eq!(actual, expected, "{orientation:?} at {row},{column}");
            }
        }
    }
}

#[test]
fn euler_rates_require_euler_angles() {
    let (rotation, derivative) = mars_case();
    let rates = AngularVelocityRepresentation::EulerAngleDerivatives {
        axis1: FrameAxis::Z,
        axis2: FrameAxis::Y,
        axis3: FrameAxis::X,
    };

    let error = describe(
        &rotation,
        &derivative,
        OrientationRepresentation::SpiceQuaternion,
        rates,
        AngularUnits::Radians,
    )
    .expect_err("quaternion with Euler rates must be rejected");
    assert!(format!("{error}").contains("EULER_ANGLE_DERIVATIVES"));

    // The axis sequence has to agree with the orientation, as in WebGeocalc.
    let error = describe(
        &rotation,
        &derivative,
        euler(FrameAxis::X, FrameAxis::Y, FrameAxis::Z),
        rates,
        AngularUnits::Radians,
    )
    .expect_err("mismatched axes must be rejected");
    assert!(format!("{error}").contains("EULER_ANGLE_DERIVATIVES"));

    let columns = describe(
        &rotation,
        &derivative,
        euler(FrameAxis::Z, FrameAxis::Y, FrameAxis::X),
        rates,
        AngularUnits::Radians,
    )
    .expect("matching axes");
    assert_eq!(
        columns.ids,
        [
            "ANGLE3",
            "ANGLE2",
            "ANGLE1",
            "ANGLE3_RATE",
            "ANGLE2_RATE",
            "ANGLE1_RATE"
        ]
    );
    assert_eq!(columns.rows[0].len(), 6);
}
