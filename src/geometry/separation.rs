//! Angular separation of independent WebGeocalc direction specifications.
use crate::pointing::{PointingContext, PointingDirection};
use crate::{AngularShape, GeoError};
/// One TWO_DIRECTIONS input. SPHERE is permitted only for a POSITION
/// direction with antiVectorFlag=false; all other directions are POINT.
#[derive(Clone, Debug, PartialEq)]
pub struct SeparationDirection {
    /// Position, velocity, or explicit vector specification.
    pub direction: PointingDirection,
    /// Point direction or apparent spherical target limb.
    pub shape: AngularShape,
}
/// One angular separation at an observer epoch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionSeparation {
    /// Observer epoch in TDB seconds past J2000.
    pub epoch: f64,
    /// Separation in radians; negative when spherical limbs overlap.
    pub angle: f64,
    /// First direction in J2000, preserving its original magnitude.
    pub first_vector: [f64; 3],
    /// Second direction in J2000, preserving its original magnitude.
    pub second_vector: [f64; 3],
    /// Epoch at which the first source frame is evaluated.
    pub first_frame_epoch: f64,
    /// Epoch at which the second source frame is evaluated.
    pub second_frame_epoch: f64,
}
/// Evaluate two independent directions and their CSPICE VSEP separation.
/// Each context may provide a different instrument FOV. FOV boundary
/// expansion is invalid for TWO_DIRECTIONS, so exactly one result is returned.
pub fn angular_separation_directions(
    first_context: &PointingContext<'_>,
    second_context: &PointingContext<'_>,
    epoch: f64,
    first: &SeparationDirection,
    second: &SeparationDirection,
) -> Result<DirectionSeparation, GeoError> {
    validate_direction(first)?;
    validate_direction(second)?;
    let a = resolve(first_context, epoch, first)?;
    let b = resolve(second_context, epoch, second)?;
    let radius1 = angular_radius(first_context, first, a.columns.vector)?;
    let radius2 = angular_radius(second_context, second, b.columns.vector)?;
    Ok(DirectionSeparation {
        epoch,
        angle: separation(a.columns.vector, b.columns.vector) - radius1 - radius2,
        first_vector: a.columns.vector,
        second_vector: b.columns.vector,
        first_frame_epoch: a.frame_epoch,
        second_frame_epoch: b.frame_epoch,
    })
}
fn validate_direction(specification: &SeparationDirection) -> Result<(), GeoError> {
    use crate::RayDirection;
    if matches!(
        &specification.direction,
        PointingDirection::Vector {
            direction: RayDirection::InstrumentFovBoundaryVectors,
            ..
        }
    ) {
        return Err(GeoError::BadSearch(
            "instrument FOV boundary vectors are not allowed for TWO_DIRECTIONS",
        ));
    }
    if specification.shape == AngularShape::Sphere {
        match &specification.direction {
            PointingDirection::Position { anti: false, .. } => {}
            PointingDirection::Position { anti: true, .. } => {
                return Err(GeoError::BadSearch(
                    "SPHERE target shape cannot use an anti-vector",
                ))
            }
            _ => {
                return Err(GeoError::BadSearch(
                    "SPHERE target shape requires a POSITION direction",
                ))
            }
        }
    }
    Ok(())
}
fn resolve(
    context: &PointingContext<'_>,
    epoch: f64,
    specification: &SeparationDirection,
) -> Result<crate::pointing::PointingResult, GeoError> {
    use crate::pointing::{
        pointing_calculation, PointingMagnitude, PointingRepresentation, PointingRequest,
    };
    let request = PointingRequest {
        epoch,
        direction: specification.direction.clone(),
        output_frame: "J2000".into(),
        magnitude: PointingMagnitude::PreserveOriginal,
        representation: PointingRepresentation::Rectangular,
    };
    let mut vectors = pointing_calculation(context, &request)?;
    if vectors.len() != 1 {
        return Err(GeoError::BadSearch(
            "TWO_DIRECTIONS requires exactly one vector per direction",
        ));
    }
    Ok(vectors.remove(0))
}
fn angular_radius(
    context: &PointingContext<'_>,
    specification: &SeparationDirection,
    vector: [f64; 3],
) -> Result<f64, GeoError> {
    if specification.shape == AngularShape::Point {
        return Ok(0.0);
    }
    let target = match specification.direction {
        PointingDirection::Position { target, .. } => target,
        _ => unreachable!(),
    };
    let pck = context.pck.ok_or(GeoError::MissingPckData(target))?;
    let axes = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let radius = axes[0].max(axes[1]).max(axes[2]);
    let distance = crate::surface::norm(vector);
    if distance <= radius {
        return Err(GeoError::BadSearch(
            "observer is inside the spherical target",
        ));
    }
    Ok((radius / distance).asin())
}
// Source operation order of SPICELIB VSEP, including the zero-vector case.
fn separation(first: [f64; 3], second: [f64; 3]) -> f64 {
    use crate::surface::{norm, unit};
    if norm(first) == 0.0 || norm(second) == 0.0 {
        return 0.0;
    }
    let a = unit(first);
    let b = unit(second);
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    if dot > 0.0 {
        2.0 * (norm([a[0] - b[0], a[1] - b[1], a[2] - b[2]]) * 0.5).asin()
    } else if dot < 0.0 {
        std::f64::consts::PI - 2.0 * (norm([a[0] + b[0], a[1] + b[1], a[2] + b[2]]) * 0.5).asin()
    } else {
        std::f64::consts::FRAC_PI_2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pointing::PointingVectorCorrection;
    use crate::{Aberration, Fov, FrameAxis, Frames, Pck, RayDirection, SpkKernel};
    fn specification(f: &[&str]) -> SeparationDirection {
        let target = f[1].parse().unwrap();
        let observer = f[2].parse().unwrap();
        let anti = f[5] == "1";
        let direction = match f[0] {
            "POSITION" => PointingDirection::Position {
                target,
                observer,
                aberration: Aberration::parse(f[3]).unwrap(),
                anti,
            },
            "VELOCITY" => PointingDirection::Velocity {
                target,
                observer,
                input_frame: f[4].into(),
                aberration: Aberration::parse(f[3]).unwrap(),
                anti,
            },
            other => {
                let vector = [
                    f[8].parse().unwrap(),
                    f[9].parse().unwrap(),
                    f[10].parse().unwrap(),
                ];
                let direction = match other {
                    "AXIS" => RayDirection::FrameAxis {
                        frame: f[4].into(),
                        axis: FrameAxis::Z,
                        anti: false,
                    },
                    "BORESIGHT" => RayDirection::InstrumentBoresight,
                    "VECTOR_FOV" => RayDirection::VectorInFov { vector },
                    _ => RayDirection::VectorInFrame {
                        frame: f[4].into(),
                        vector,
                        anti: false,
                        stellar_aberration: false,
                    },
                };
                PointingDirection::Vector {
                    direction,
                    observer,
                    aberration: PointingVectorCorrection::parse(f[3]).unwrap(),
                    anti,
                }
            }
        };
        SeparationDirection {
            direction,
            shape: AngularShape::parse(f[6]).unwrap(),
        }
    }
    #[test]
    fn exact_cspice_two_directions() {
        let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let ik = include_str!("../../tests/golden/fov_ellipsoid.tf");
        let frames = Frames::from_text(ik).unwrap();
        let fov1 = Fov::from_text(ik, 1420001).unwrap();
        let fov2 = Fov::from_text(ik, 1420025).unwrap();
        let context = |instrument: &str| PointingContext {
            kernel: &kernel,
            pck: Some(&pck),
            frames: &frames,
            fov: Some(if instrument == "1420001" {
                &fov1
            } else {
                &fov2
            }),
        };
        let mut differences = [0; 3];
        let mut worst = [0.0_f64; 3];
        let mut labels = [String::new(), String::new(), String::new()];
        for line in include_str!("../../tests/golden/two_directions_precision.csv").lines() {
            let f: Vec<_> = line.split(',').collect();
            let first = specification(&f[1..12]);
            let second = specification(&f[12..23]);
            let a = angular_separation_directions(
                &context(f[8]),
                &context(f[19]),
                f[0].parse().unwrap(),
                &first,
                &second,
            )
            .unwrap();
            let expected: Vec<f64> = f[23..].iter().map(|x| x.parse().unwrap()).collect();
            for (i, (a, b)) in [a.angle]
                .into_iter()
                .chain(a.first_vector)
                .chain(a.second_vector)
                .chain([a.first_frame_epoch, a.second_frame_epoch])
                .zip(&expected)
                .enumerate()
            {
                let metric = if i == 0 {
                    0
                } else if i < 7 {
                    1
                } else {
                    2
                };
                if a.to_bits() != b.to_bits() {
                    differences[metric] += 1;
                }
                if (a - b).abs() > worst[metric] {
                    worst[metric] = (a - b).abs();
                    labels[metric] = line.into();
                }
            }
        }
        eprintln!("TWO_DIRECTIONS differing angle/vector/epoch components {differences:?}; worst{worst:?} at{labels:?}");
        assert_eq!(differences, [0; 3]);
    }
    #[test]
    fn exact_cspice_for_live_nasa_probe_inputs() {
        let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let frames = Frames::new();
        let context = PointingContext {
            kernel: &kernel,
            pck: Some(&pck),
            frames: &frames,
            fov: None,
        };
        let mut output = String::from("label,angle_degrees\n");
        for line in include_str!("../../tests/golden/two_directions_live_cspice.csv").lines() {
            let f: Vec<_> = line.split(',').collect();
            let first = specification(&f[2..13]);
            let second = specification(&f[13..24]);
            let result = angular_separation_directions(
                &context,
                &context,
                f[1].parse().unwrap(),
                &first,
                &second,
            )
            .unwrap();
            let expected = f[24].parse::<f64>().unwrap();
            assert_eq!(result.angle.to_bits(), expected.to_bits(), "{}", f[0]);
            output.push_str(&format!("{},{:?}\n", f[0], result.angle.to_degrees()));
        }
        std::fs::create_dir_all("target/reference").unwrap();
        std::fs::write("target/reference/two-directions-local.csv", output).unwrap();
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::pointing::PointingVectorCorrection;
    use crate::{Aberration, Frames, RayDirection, SpkKernel};
    fn vector(ray: RayDirection) -> SeparationDirection {
        SeparationDirection {
            direction: PointingDirection::Vector {
                direction: ray,
                observer: 399,
                aberration: PointingVectorCorrection::None,
                anti: false,
            },
            shape: AngularShape::Point,
        }
    }
    #[test]
    fn rejects_instrument_boundary_expansion_on_either_side() {
        let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
        let frames = Frames::new();
        let context = PointingContext {
            kernel: &kernel,
            frames: &frames,
            pck: None,
            fov: None,
        };
        let boundary = vector(RayDirection::InstrumentFovBoundaryVectors);
        let point = vector(RayDirection::VectorInFrame {
            frame: "J2000".into(),
            vector: [1.0, 0.0, 0.0],
            anti: false,
            stellar_aberration: false,
        });
        for (a, b) in [(&boundary, &point), (&point, &boundary)] {
            let error = angular_separation_directions(&context, &context, 0.0, a, b).unwrap_err();
            assert!(error
                .to_string()
                .contains("boundary vectors are not allowed"));
        }
    }
    #[test]
    fn rejects_sphere_anti_and_nonposition_spheres() {
        let invalid = SeparationDirection {
            direction: PointingDirection::Position {
                target: 301,
                observer: 399,
                aberration: Aberration::None,
                anti: true,
            },
            shape: AngularShape::Sphere,
        };
        assert!(validate_direction(&invalid).is_err());
        let invalid = SeparationDirection {
            direction: PointingDirection::Velocity {
                target: 301,
                observer: 399,
                input_frame: "J2000".into(),
                aberration: Aberration::None,
                anti: false,
            },
            shape: AngularShape::Sphere,
        };
        assert!(validate_direction(&invalid).is_err());
    }
    #[test]
    fn exact_cspice_vsep_edge_cases() {
        for line in include_str!("../../tests/golden/two_directions_vsep_precision.csv").lines() {
            let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
            let actual = separation([v[0], v[1], v[2]], [v[3], v[4], v[5]]);
            assert_eq!(actual.to_bits(), v[6].to_bits(), "{line}");
        }
    }
}
