// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::needless_range_loop)]
//! Frame orientation representations and angular velocity.
//!
//! WebGeocalc's `FRAME_TRANSFORMATION` reports the rotation between two
//! frames in seven forms and the angular velocity in five. Every form is a
//! pure function of the rotation matrix `r` and its time derivative
//! `r_dot`, so this module takes those two matrices and produces the
//! WebGeocalc column set. Calendars, frames, and aberration corrections
//! stay outside it.
//!
//! Angular velocity sign conventions were fixed against CSPICE `sxform`:
//!
//! ```text
//! omega_in_frame1 = vee(-r^T r_dot)
//! omega_in_frame2 = vee(-r_dot r^T) = r * omega_in_frame1
//! ```
//!
//! with `vee` the inverse of the cross-product matrix. Both reproduce the
//! live WebGeocalc `VECTOR_IN_FRAME1` and `VECTOR_IN_FRAME2` columns.

use crate::error::GeoError;
use crate::frame::Mat3;

/// Angular unit for orientation angles and angular velocity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngularUnits {
    /// Degrees.
    Degrees,
    /// Radians.
    Radians,
    /// Revolutions per minute, WebGeocalc's `RPM`.
    RevolutionsPerMinute,
}

impl AngularUnits {
    /// Parses a WebGeocalc `angularUnits` or `angularVelocityUnits` value.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "deg" | "degrees" => Some(Self::Degrees),
            "rad" | "radians" => Some(Self::Radians),
            "rpm" => Some(Self::RevolutionsPerMinute),
            "unitless" | "unitary" => None,
            _ => None,
        }
    }

    fn factor(self) -> f64 {
        match self {
            Self::Radians => 1.0,
            Self::Degrees => 180.0 / std::f64::consts::PI,
            Self::RevolutionsPerMinute => 60.0 / (2.0 * std::f64::consts::PI),
        }
    }
}

/// Independent units for WebGeocalc angular velocity output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngularVelocityUnits {
    RadiansPerSecond,
    DegreesPerSecond,
    RevolutionsPerMinute,
    /// Unit vector with AV_MAG omitted; valid only for vector output.
    Unitary,
}
impl AngularVelocityUnits {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "RAD/S" | "RADIANS" => Some(Self::RadiansPerSecond),
            "DEG/S" | "DEGREES" => Some(Self::DegreesPerSecond),
            "RPM" => Some(Self::RevolutionsPerMinute),
            "UNITLESS" | "UNITARY" => Some(Self::Unitary),
            _ => None,
        }
    }
    fn factor(self) -> f64 {
        match self {
            Self::RadiansPerSecond | Self::Unitary => 1.0,
            Self::DegreesPerSecond => 180.0 / std::f64::consts::PI,
            Self::RevolutionsPerMinute => 60.0 / std::f64::consts::TAU,
        }
    }
}

/// One of the coordinate axes, for Euler sequences and frame axes.
pub use crate::direction::FrameAxis;

impl FrameAxis {
    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    fn matrix(self, angle: f64) -> Mat3 {
        let (sin, cos) = angle.sin_cos();
        match self {
            Self::X => [[1.0, 0.0, 0.0], [0.0, cos, sin], [0.0, -sin, cos]],
            Self::Y => [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]],
            Self::Z => [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]],
        }
    }
}

/// How WebGeocalc reports the rotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrientationRepresentation {
    /// Three successive rotations, `angle3`, `angle2`, `angle1`.
    EulerAngles {
        /// Axis of the first angle in the output triple (`axis1`).
        axis1: FrameAxis,
        /// Middle axis.
        axis2: FrameAxis,
        /// Axis of `angle3`.
        axis3: FrameAxis,
    },
    /// Rotation angle and the unit axis it is about.
    AngleAndAxis,
    /// Quaternion with the scalar first, as CSPICE `m2q` returns it.
    SpiceQuaternion,
    /// Quaternion with the scalar last and the vector part negated.
    OtherQuaternion,
    /// Three row entries per output row, with an explicit `ROW_NUMBER`.
    MatrixRowByRow,
    /// Three row entries per output row, without `ROW_NUMBER`.
    MatrixFlagged,
    /// All nine entries in one output row.
    MatrixAllOneRow,
}

impl OrientationRepresentation {
    /// Parses a WebGeocalc `orientationRepresentation` value.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "EULER_ANGLES" => Some(Self::EulerAngles {
                axis1: FrameAxis::X,
                axis2: FrameAxis::Y,
                axis3: FrameAxis::Z,
            }),
            "ANGLE_AND_AXIS" => Some(Self::AngleAndAxis),
            "SPICE_QUATERNION" => Some(Self::SpiceQuaternion),
            "OTHER_QUATERNION" => Some(Self::OtherQuaternion),
            "MATRIX_ROW_BY_ROW" => Some(Self::MatrixRowByRow),
            "MATRIX_FLAGGED" => Some(Self::MatrixFlagged),
            "MATRIX_ALL_ONE_ROW" => Some(Self::MatrixAllOneRow),
            _ => None,
        }
    }
}

/// How WebGeocalc reports the angular velocity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngularVelocityRepresentation {
    /// Omit it.
    NotIncluded,
    /// Vector in the first frame.
    VectorInFrame1,
    /// Vector in the second frame.
    VectorInFrame2,
    /// Time derivatives of the Euler angles.
    EulerAngleDerivatives {
        /// Axis of the first angle in the output triple.
        axis1: FrameAxis,
        /// Middle axis.
        axis2: FrameAxis,
        /// Axis of `angle3`.
        axis3: FrameAxis,
    },
    /// The full 6x6 state transformation matrix.
    Matrix,
}

impl AngularVelocityRepresentation {
    /// Parses a WebGeocalc `angularVelocityRepresentation` value.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "NOT_INCLUDED" => Some(Self::NotIncluded),
            "VECTOR_IN_FRAME1" => Some(Self::VectorInFrame1),
            "VECTOR_IN_FRAME2" => Some(Self::VectorInFrame2),
            "EULER_ANGLE_DERIVATIVES" => Some(Self::EulerAngleDerivatives {
                axis1: FrameAxis::X,
                axis2: FrameAxis::Y,
                axis3: FrameAxis::Z,
            }),
            "MATRIX" => Some(Self::Matrix),
            _ => None,
        }
    }
}

/// A WebGeocalc-shaped orientation table: column identifiers plus rows.
#[derive(Clone, Debug, PartialEq)]
pub struct OrientationColumns {
    /// Column identifiers, excluding `DATE`.
    pub ids: Vec<&'static str>,
    /// One row, or three for the row-by-row matrix forms.
    pub rows: Vec<Vec<f64>>,
}

/// Angular velocity in the form the output columns need.
enum AngularVelocity {
    /// A rotation vector, in radians per second.
    Vector([f64; 3]),
    /// Time derivatives of the three Euler angles, in radians per second.
    Rates([f64; 3]),
}

/// Builds the WebGeocalc column set for one rotation.
///
/// `rotation` maps vectors from frame 1 into frame 2, matching
/// `pxform(frame1, frame2, et)`. `rotation_dot` is its time derivative,
/// which is zero for two time-independent frames.
pub fn describe(
    rotation: &Mat3,
    rotation_dot: &Mat3,
    orientation: OrientationRepresentation,
    angular_velocity: AngularVelocityRepresentation,
    units: AngularUnits,
) -> Result<OrientationColumns, GeoError> {
    let velocity_units = match units {
        AngularUnits::Radians => AngularVelocityUnits::RadiansPerSecond,
        AngularUnits::Degrees => AngularVelocityUnits::DegreesPerSecond,
        AngularUnits::RevolutionsPerMinute => AngularVelocityUnits::RevolutionsPerMinute,
    };
    describe_with_units(
        rotation,
        rotation_dot,
        orientation,
        angular_velocity,
        units,
        velocity_units,
    )
}

/// Orientation and angular velocity with independently selected units.
pub fn describe_with_units(
    rotation: &Mat3,
    rotation_dot: &Mat3,
    orientation: OrientationRepresentation,
    angular_velocity: AngularVelocityRepresentation,
    units: AngularUnits,
    velocity_units: AngularVelocityUnits,
) -> Result<OrientationColumns, GeoError> {
    let mut ids: Vec<&'static str> = Vec::new();
    let mut rows: Vec<Vec<f64>> = Vec::new();

    let euler_axes = |axis1: FrameAxis, axis2: FrameAxis, axis3: FrameAxis| {
        if axis1 == axis2 || axis2 == axis3 {
            Err(GeoError::BadSearch("consecutive Euler axes must differ"))
        } else {
            Ok((axis1, axis2, axis3))
        }
    };

    match orientation {
        OrientationRepresentation::EulerAngles {
            axis1,
            axis2,
            axis3,
        } => {
            let axes = euler_axes(axis1, axis2, axis3)?;
            let angles = matrix_to_euler(rotation, axes)?;
            let scale = units.factor();
            ids.extend(["ANGLE3", "ANGLE2", "ANGLE1"]);
            rows.push(vec![
                angles[0] * scale,
                angles[1] * scale,
                angles[2] * scale,
            ]);
        }
        OrientationRepresentation::AngleAndAxis => {
            let (angle, axis) = matrix_to_axis_angle(rotation);
            ids.extend(["ANGLE", "AXIS_X", "AXIS_Y", "AXIS_Z"]);
            rows.push(vec![angle * units.factor(), axis[0], axis[1], axis[2]]);
        }
        OrientationRepresentation::SpiceQuaternion => {
            let quaternion = matrix_to_quaternion(rotation);
            ids.extend(["QCOS", "QSIN1", "QSIN2", "QSIN3"]);
            rows.push(vec![
                quaternion[0],
                quaternion[1],
                quaternion[2],
                quaternion[3],
            ]);
        }
        OrientationRepresentation::OtherQuaternion => {
            let quaternion = matrix_to_quaternion(rotation);
            ids.extend(["QSIN1", "QSIN2", "QSIN3", "QCOS"]);
            rows.push(vec![
                -quaternion[1],
                -quaternion[2],
                -quaternion[3],
                quaternion[0],
            ]);
        }
        OrientationRepresentation::MatrixRowByRow => {
            ids.push("ROW_NUMBER");
            ids.extend(["M11", "M12", "M13"]);
            for (index, line) in rotation.iter().enumerate() {
                rows.push(vec![(index + 1) as f64, line[0], line[1], line[2]]);
            }
        }
        OrientationRepresentation::MatrixFlagged => {
            ids.extend(["M11", "M12", "M13"]);
            for line in rotation.iter() {
                rows.push(vec![line[0], line[1], line[2]]);
            }
        }
        OrientationRepresentation::MatrixAllOneRow => {
            ids.extend([
                "M11", "M12", "M13", "M21", "M22", "M23", "M31", "M32", "M33",
            ]);
            let mut row = Vec::with_capacity(9);
            for line in rotation {
                row.extend_from_slice(line);
            }
            rows.push(row);
        }
    }

    // The 6x6 state transformation is only emitted for the three matrix
    // orientations; WebGeocalc drops it for the angle and quaternion forms.
    // It replaces the orientation block, so it is built here and returned.
    if matches!(angular_velocity, AngularVelocityRepresentation::Matrix) {
        if let Some(layout) = MatrixLayout::of(orientation) {
            return Ok(state_matrix_columns(rotation, rotation_dot, layout));
        }
    }

    let velocity = match angular_velocity {
        AngularVelocityRepresentation::NotIncluded | AngularVelocityRepresentation::Matrix => None,
        AngularVelocityRepresentation::VectorInFrame1 => Some(AngularVelocity::Vector(
            angular_velocity_vector(rotation, rotation_dot, true),
        )),
        AngularVelocityRepresentation::VectorInFrame2 => Some(AngularVelocity::Vector(
            angular_velocity_vector(rotation, rotation_dot, false),
        )),
        AngularVelocityRepresentation::EulerAngleDerivatives {
            axis1,
            axis2,
            axis3,
        } => {
            // WebGeocalc only accepts Euler angle rates together with
            // `EULER_ANGLES`, and derives them from the same axis sequence.
            let OrientationRepresentation::EulerAngles {
                axis1: orientation1,
                axis2: orientation2,
                axis3: orientation3,
            } = orientation
            else {
                return Err(GeoError::Unsupported(
                    "EULER_ANGLE_DERIVATIVES needs EULER_ANGLES".to_string(),
                ));
            };
            if (orientation1, orientation2, orientation3) != (axis1, axis2, axis3) {
                return Err(GeoError::Unsupported(
                    "EULER_ANGLE_DERIVATIVES needs the orientation axes".to_string(),
                ));
            }
            let axes = euler_axes(axis1, axis2, axis3)?;
            let (state, _) = euler_angles_with_rates(rotation, rotation_dot, axes)?;
            Some(AngularVelocity::Rates([state[3], state[4], state[5]]))
        }
    };

    match velocity {
        None => {}
        Some(AngularVelocity::Rates(rates)) => {
            if velocity_units == AngularVelocityUnits::Unitary {
                return Err(GeoError::BadSearch(
                    "unitary angular velocity requires vector output",
                ));
            }
            let scale = velocity_units.factor();
            ids.extend(["ANGLE3_RATE", "ANGLE2_RATE", "ANGLE1_RATE"]);
            let values = [rates[0] * scale, rates[1] * scale, rates[2] * scale];
            for row in &mut rows {
                row.extend(values);
            }
        }
        Some(AngularVelocity::Vector(mut vector)) => {
            if velocity_units == AngularVelocityUnits::Unitary {
                vector = crate::surface::unit(vector);
                ids.extend(["AV_X", "AV_Y", "AV_Z"]);
                for row in &mut rows {
                    row.extend(vector);
                }
                return Ok(OrientationColumns { ids, rows });
            }
            let scale = velocity_units.factor();
            let magnitude = crate::surface::norm(vector);
            ids.extend(["AV_X", "AV_Y", "AV_Z", "AV_MAG"]);
            let values = [
                vector[0] * scale,
                vector[1] * scale,
                vector[2] * scale,
                magnitude * scale,
            ];
            // WebGeocalc repeats the angular velocity on every row of the
            // row-by-row and flagged matrix forms.
            for row in &mut rows {
                row.extend(values);
            }
        }
    }

    Ok(OrientationColumns { ids, rows })
}

/// Which row layout a matrix orientation uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatrixLayout {
    RowByRow,
    Flagged,
    AllOneRow,
}

impl MatrixLayout {
    fn of(orientation: OrientationRepresentation) -> Option<Self> {
        match orientation {
            OrientationRepresentation::MatrixRowByRow => Some(Self::RowByRow),
            OrientationRepresentation::MatrixFlagged => Some(Self::Flagged),
            OrientationRepresentation::MatrixAllOneRow => Some(Self::AllOneRow),
            _ => None,
        }
    }
}

/// The 6x6 state transformation columns, laid out as WebGeocalc reports them.
fn state_matrix_columns(
    rotation: &Mat3,
    rotation_dot: &Mat3,
    layout: MatrixLayout,
) -> OrientationColumns {
    let mut ids: Vec<&'static str> = Vec::new();
    let mut rows: Vec<Vec<f64>> = Vec::new();

    match layout {
        MatrixLayout::AllOneRow => {
            for row in 0..6 {
                for column in 0..6 {
                    ids.push(state_matrix_id(row, column));
                }
            }
            let mut values = Vec::with_capacity(36);
            for row in 0..6 {
                for column in 0..6 {
                    values.push(state_matrix_value(rotation, rotation_dot, row, column));
                }
            }
            rows.push(values);
        }
        MatrixLayout::RowByRow | MatrixLayout::Flagged => {
            if layout == MatrixLayout::RowByRow {
                ids.push("ROW_NUMBER");
            }
            for column in 0..6 {
                ids.push(state_matrix_id(0, column));
            }
            for row in 0..6 {
                let mut values = Vec::with_capacity(7);
                if layout == MatrixLayout::RowByRow {
                    values.push((row + 1) as f64);
                }
                for column in 0..6 {
                    values.push(state_matrix_value(rotation, rotation_dot, row, column));
                }
                rows.push(values);
            }
        }
    }

    OrientationColumns { ids, rows }
}

fn state_matrix_value(rotation: &Mat3, rotation_dot: &Mat3, row: usize, column: usize) -> f64 {
    match (row, column) {
        (0..=2, 0..=2) => rotation[row][column],
        (3..=5, 0..=2) => rotation_dot[row - 3][column],
        (3..=5, 3..=5) => rotation[row - 3][column - 3],
        _ => 0.0,
    }
}

fn state_matrix_id(row: usize, column: usize) -> &'static str {
    const IDS: [[&str; 6]; 6] = [
        ["M11", "M12", "M13", "M14", "M15", "M16"],
        ["M21", "M22", "M23", "M24", "M25", "M26"],
        ["M31", "M32", "M33", "M34", "M35", "M36"],
        ["M41", "M42", "M43", "M44", "M45", "M46"],
        ["M51", "M52", "M53", "M54", "M55", "M56"],
        ["M61", "M62", "M63", "M64", "M65", "M66"],
    ];
    IDS[row][column]
}

/// `vee` of `-r^T r_dot` (frame 1) or `-r_dot r^T` (frame 2).
pub fn angular_velocity_vector(rotation: &Mat3, rotation_dot: &Mat3, frame_one: bool) -> [f64; 3] {
    // XF2RAV: D^T R, not an algebraically substituted -R^T D.
    let omega = crate::frame::mat_mul(&transpose(rotation_dot), rotation);
    let vector = [omega[2][1], omega[0][2], omega[1][0]];
    if frame_one {
        vector
    } else {
        apply(rotation, vector)
    }
}

/// Native RAXISA angle and axis, including exact zero and half-turn cases.
pub fn matrix_to_axis_angle(rotation: &Mat3) -> (f64, [f64; 3]) {
    let q = matrix_to_quaternion(rotation);
    let vector = [q[1], q[2], q[3]];
    if vector == [0.0; 3] {
        return (0.0, [0.0, 0.0, 1.0]);
    }
    if q[0] == 0.0 {
        return (std::f64::consts::PI, vector);
    }
    let length = crate::surface::norm(vector);
    (
        length.atan2(q[0]) * 2.0,
        [vector[0] / length, vector[1] / length, vector[2] / length],
    )
}

/// CSPICE M2Q's branch selection, operation order and normalization.
pub fn matrix_to_quaternion(r: &Mat3) -> [f64; 4] {
    let trace = r[0][0] + r[1][1] + r[2][2];
    let mtrace = 1.0 - trace;
    let cc4 = trace + 1.0;
    let s114 = mtrace + r[0][0] * 2.0;
    let s224 = mtrace + r[1][1] * 2.0;
    let s334 = mtrace + r[2][2] * 2.0;
    let mut q = [0.0; 4];
    if cc4 >= 1.0 {
        q[0] = (cc4 * 0.25).sqrt();
        let f = 1.0 / (q[0] * 4.0);
        q[1] = (r[2][1] - r[1][2]) * f;
        q[2] = (r[0][2] - r[2][0]) * f;
        q[3] = (r[1][0] - r[0][1]) * f;
    } else if s114 >= 1.0 {
        q[1] = (s114 * 0.25).sqrt();
        let f = 1.0 / (q[1] * 4.0);
        q[0] = (r[2][1] - r[1][2]) * f;
        q[2] = (r[0][1] + r[1][0]) * f;
        q[3] = (r[0][2] + r[2][0]) * f;
    } else if s224 >= 1.0 {
        q[2] = (s224 * 0.25).sqrt();
        let f = 1.0 / (q[2] * 4.0);
        q[0] = (r[0][2] - r[2][0]) * f;
        q[1] = (r[0][1] + r[1][0]) * f;
        q[3] = (r[1][2] + r[2][1]) * f;
    } else {
        q[3] = (s334 * 0.25).sqrt();
        let f = 1.0 / (q[3] * 4.0);
        q[0] = (r[1][0] - r[0][1]) * f;
        q[1] = (r[0][2] + r[2][0]) * f;
        q[2] = (r[1][2] + r[2][1]) * f;
    }
    let squared = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
    if squared != 1.0 {
        let polish = 1.0 / squared.sqrt();
        for x in &mut q {
            *x *= polish;
        }
    }
    if q[0] <= 0.0 {
        for x in &mut q {
            *x = -*x;
        }
    }
    q
}

/// Euler angles `(angle3, angle2, angle1)` of `rotation`, matching CSPICE
/// `m2eul(rotation, axis3, axis2, axis1)`.
///
/// The rotation is `[angle3]_axis3 * [angle2]_axis2 * [angle1]_axis1` using
/// SPICE's passive rotation matrices. The sign factor is the parity of the
/// first two axes: `+1` when `axis2` is the next axis after `axis1`, `-1`
/// otherwise. Both branches below were confirmed against `m2eul` over all
/// twelve valid triples.
pub fn matrix_to_euler(
    rotation: &Mat3,
    axes: (FrameAxis, FrameAxis, FrameAxis),
) -> Result<[f64; 3], GeoError> {
    let (i, j, k) = (axes.0.index(), axes.1.index(), axes.2.index());
    if i == j || j == k {
        return Err(GeoError::BadSearch("consecutive Euler axes must differ"));
    }
    let mut r = *rotation;
    for column in 0..3 {
        let v = crate::surface::unit([r[0][column], r[1][column], r[2][column]]);
        for row in 0..3 {
            r[row][column] = v[row];
        }
    }
    let sign = if j == (k + 1) % 3 { 1.0 } else { -1.0 };
    let mut change = [[0.0; 3]; 3];
    if i == k {
        change[k][2] = 1.0;
        change[j][0] = 1.0;
        change[3 - k - j][1] = sign;
    } else {
        change[k][0] = 1.0;
        change[j][1] = 1.0;
        change[i][2] = sign;
    }
    let t = crate::frame::mat_mul(&transpose(&change), &crate::frame::mat_mul(&r, &change));
    if i == k {
        let degenerate = (t[0][2] == 0.0 && t[1][2] == 0.0)
            || (t[2][0] == 0.0 && t[2][1] == 0.0)
            || t[2][2].abs() == 1.0;
        if degenerate {
            Ok([0.0, t[2][2].acos(), t[0][1].atan2(t[0][0])])
        } else {
            Ok([
                t[0][2].atan2(t[1][2]),
                t[2][2].acos(),
                t[2][0].atan2(-t[2][1]),
            ])
        }
    } else {
        let degenerate = (t[0][0] == 0.0 && t[0][1] == 0.0)
            || (t[1][2] == 0.0 && t[2][2] == 0.0)
            || t[0][2].abs() == 1.0;
        if degenerate {
            Ok([0.0, (-t[0][2]).asin(), sign * (-t[1][0]).atan2(t[1][1])])
        } else {
            Ok([
                t[1][2].atan2(t[2][2]),
                (-t[0][2]).asin(),
                sign * t[0][1].atan2(t[0][0]),
            ])
        }
    }
}

/// CSPICE XF2EUL, returning angle3/2/1 and their rates and uniqueness.
pub fn euler_angles_with_rates(
    rotation: &Mat3,
    derivative: &Mat3,
    axes: (FrameAxis, FrameAxis, FrameAxis),
) -> Result<([f64; 6], bool), GeoError> {
    let angles = matrix_to_euler(rotation, axes)?;
    let (a, b, c) = (axes.2.index(), axes.1.index(), axes.0.index());
    let l = 3 - a - b;
    let d = if b == (a + 1) % 3 { 1.0 } else { -1.0 };
    let skew = crate::frame::mat_mul(derivative, &transpose(rotation));
    let omega = [d * skew[l][b], d * skew[a][l], d * skew[b][a]];
    let ca = angles[0].cos();
    let sa = angles[0].sin();
    let (u, mut v) = if a == c {
        (angles[1].cos(), d * angles[1].sin())
    } else {
        (-d * angles[1].sin(), angles[1].cos())
    };
    if u.abs() == 1.0 {
        v = 0.0;
    }
    let rates = if v == 0.0 {
        [
            0.0,
            if ca.abs() > sa.abs() {
                -omega[1] / ca
            } else {
                d * omega[2] / sa
            },
            -u * omega[0],
        ]
    } else {
        apply(
            &[
                [-1.0, u * d * sa / v, u * ca / v],
                [0.0, -ca, d * sa],
                [0.0, -d * sa / v, -ca / v],
            ],
            omega,
        )
    };
    Ok((
        [
            angles[0], angles[1], angles[2], rates[0], rates[1], rates[2],
        ],
        v != 0.0,
    ))
}

/// Rate of change of `(angle3, angle2, angle1)` given the angular velocity
/// expressed in frame 1.
///
/// With `rotation = [a3]_axis3 [a2]_axis2 [a1]_axis1`, differentiating and
/// using `d[theta]_axis/dtheta = [theta]_axis * A_axis` gives
///
/// ```text
/// omega_frame1 = angle1_dot * e_axis1
///              + angle2_dot * [a1]^T e_axis2
///              + angle3_dot * [a1]^T [a2]^T e_axis3
/// ```
///
/// which is a 3x3 linear system for the three rates.
pub fn euler_angle_rates(
    angles: [f64; 3],
    axes: (FrameAxis, FrameAxis, FrameAxis),
    omega_frame1: [f64; 3],
) -> Result<[f64; 3], GeoError> {
    let (axis1, axis2, axis3) = axes;
    if axis1 == axis2 || axis2 == axis3 {
        return Err(GeoError::BadSearch("consecutive Euler axes must differ"));
    }
    let first = axis1.matrix(angles[2]);
    let middle = axis2.matrix(angles[1]);

    let column1 = basis(axis1);
    let column2 = apply(&transpose(&first), basis(axis2));
    // The outermost rotation is applied first on the left, so this is
    // `[angle1]^T [angle2]^T e_axis3`, not the other order.
    let column3 = apply(&transpose(&first), apply(&transpose(&middle), basis(axis3)));

    let matrix = [
        [column1[0], column2[0], column3[0]],
        [column1[1], column2[1], column3[1]],
        [column1[2], column2[2], column3[2]],
    ];
    let inverse = crate::occult::inverse3(&matrix)?;
    let rates = apply(&inverse, omega_frame1);
    Ok([rates[2], rates[1], rates[0]])
}

fn basis(axis: FrameAxis) -> [f64; 3] {
    match axis {
        FrameAxis::X => [1.0, 0.0, 0.0],
        FrameAxis::Y => [0.0, 1.0, 0.0],
        FrameAxis::Z => [0.0, 0.0, 1.0],
    }
}

pub(crate) fn euler_state(
    mut e: [f64; 6],
    axes: [usize; 3],
) -> Result<crate::FrameRotation, GeoError> {
    use crate::frame::{mat_mul, rot1, rot2, rot3, rotate_matrix};
    if axes.iter().any(|axis| *axis > 2) {
        return Err(GeoError::BadSearch("Euler axis must be1,2or3"));
    }
    let [a, mut b, c] = axes;
    if b == a || b == c {
        let i = if b == a { 0 } else { 2 };
        e[i] += e[1];
        e[1] = 0.0;
        e[i + 3] += e[4];
        e[4] = 0.0;
        b = if c == (a + 1) % 3 {
            (c + 1) % 3
        } else {
            (a + 1) % 3
        };
    }
    let first = match c {
        0 => rot1(e[2]),
        1 => rot2(e[2]),
        _ => rot3(e[2]),
    };
    let rotation = rotate_matrix(rotate_matrix(first, e[1], b), e[0], a);
    let l = 3 - a - b;
    let d = if b == (a + 1) % 3 { 1.0 } else { -1.0 };
    let ca = e[0].cos();
    let sa = e[0].sin();
    let (u, v) = if a == c {
        (e[1].cos(), d * e[1].sin())
    } else {
        (-d * e[1].sin(), e[1].cos())
    };
    let solution = [
        [-d, 0.0, -d * u],
        [0.0, -d * ca, -sa * v],
        [0.0, sa, -d * ca * v],
    ];
    let omega: [f64; 3] = std::array::from_fn(|i| {
        solution[i][0] * e[3] + solution[i][1] * e[4] + solution[i][2] * e[5]
    });
    let mut skew = [[0.0; 3]; 3];
    skew[l][b] = omega[0];
    skew[b][l] = -omega[0];
    skew[a][l] = omega[1];
    skew[l][a] = -omega[1];
    skew[b][a] = omega[2];
    skew[a][b] = -omega[2];
    Ok(crate::FrameRotation {
        rotation,
        rotation_dot: mat_mul(&skew, &rotation),
    })
}

#[cfg(test)]
// Keep each source primitive's independent regression adjacent to its implementation.
#[allow(clippy::items_after_test_module)]
mod construction_tests {
    #[test]
    fn native_eul2xf_all_axis_sequences_exact() {
        let mut failures = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/eul2xf_exact.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<f64> = line.split(',').map(|v| v.parse().unwrap()).collect();
            let axes = [f[0] as usize - 1, f[1] as usize - 1, f[2] as usize - 1];
            let e = [f[3], f[4], f[5], f[6], f[7], f[8]];
            match super::euler_state(e, axes) {
                Ok(x) => {
                    let mut values = Vec::new();
                    for i in 0..6 {
                        for j in 0..6 {
                            values.push(if i < 3 && j < 3 {
                                x.rotation[i][j]
                            } else if i >= 3 && j >= 3 {
                                x.rotation[i - 3][j - 3]
                            } else if i >= 3 {
                                x.rotation_dot[i - 3][j]
                            } else {
                                0.0
                            });
                        }
                    }
                    if values != f[9..] {
                        failures.push(format!("axes{axes:?}: {values:?}"));
                    }
                }
                Err(e) => failures.push(e.to_string()),
            }
            count += 1;
        }
        assert_eq!(count, 540);
        assert!(
            failures.is_empty(),
            "{} EUL2XF failures: {:?}",
            failures.len(),
            failures.iter().take(3).collect::<Vec<_>>()
        );
    }
}

fn apply(matrix: &Mat3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn transpose(matrix: &Mat3) -> Mat3 {
    [
        [matrix[0][0], matrix[1][0], matrix[2][0]],
        [matrix[0][1], matrix[1][1], matrix[2][1]],
        [matrix[0][2], matrix[1][2], matrix[2][2]],
    ]
}
