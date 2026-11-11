//! CSPICE ZZGFFVST ellipsoid FOV visibility: artificial ellipsoid for
//! circular/elliptical FOVs, ZZELVUPY limb/pyramid intersection for polygons.
use crate::frame::{mat_mul, transpose, Mat3};
use crate::occult_ellipsoid::{
    angle, classify_semiaxes, edlimb, intersect_ray, plane_spans, Ellipse, Plane,
};
use crate::surface::{norm, perpendicular, project, unit, unit_cross};
use crate::tangent::ellipse_axes;
use crate::{Aberration, Fov, FovShape, Frames, GeoError, Pck, SpkKernel};
use std::f64::consts::{FRAC_PI_2, TAU};
#[allow(clippy::too_many_arguments)]
pub(crate) fn ellipsoid_in_fov(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: &Fov,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: &Frames,
) -> Result<bool, GeoError> {
    calculate(kernel, pck, fov, target, observer, et, abcorr, frames, None)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn ellipsoid_in_frame(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: &Fov,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: &Frames,
    target_frame: &str,
) -> Result<bool, GeoError> {
    if frames.center(target_frame)? != target {
        return Err(GeoError::BadSearch(
            "ellipsoid frame must be centered on target",
        ));
    }
    calculate(
        kernel,
        pck,
        fov,
        target,
        observer,
        et,
        abcorr,
        frames,
        Some(target_frame),
    )
}

#[allow(clippy::too_many_arguments)]
fn calculate(
    kernel: &SpkKernel,
    pck: &Pck,
    fov: &Fov,
    target: i32,
    observer: i32,
    et: f64,
    abcorr: Aberration,
    frames: &Frames,
    target_frame: Option<&str>,
) -> Result<bool, GeoError> {
    fov.validate_visibility()?;
    if target == observer {
        return Err(GeoError::BadSearch(
            "FOV target and observer must be distinct",
        ));
    }
    let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
    let (position, lt) = crate::spkapo::corrected_position(kernel, target, observer, et, abcorr)?;
    let epoch = crate::state_rep::time_at_target(et, lt, abcorr);
    let body = if let Some(frame) = target_frame {
        frames
            .pxform_with_derivative("J2000", frame, epoch, Some(pck))?
            .rotation
    } else {
        pck.body_state_rotation_checked(target, epoch)?.0
    };
    let pos = rotate(&body, position);
    let to_instrument = frames
        .pxform_with_derivative("J2000", &fov.frame, et, Some(pck))?
        .rotation;
    let matrix = mat_mul(&to_instrument, &transpose(&body));
    if matches!(fov.shape, FovShape::Circle | FovShape::Ellipse) {
        let (center, axes) = artificial_ellipsoid(fov)?;
        let target_center = rotate(&matrix, pos);
        let mut target_axes = matrix;
        for row in &mut target_axes {
            for i in 0..3 {
                row[i] *= radii[i];
            }
        }
        Ok(classify_semiaxes([0.0; 3], center, &axes, target_center, &target_axes)? != 0)
    } else {
        let limb = edlimb(radii, neg(pos))?;
        let center = rotate(&matrix, add(pos, limb.center));
        let (major, minor) = ellipse_axes(rotate(&matrix, limb.major), rotate(&matrix, limb.minor));
        ellipse_in_pyramid(
            Ellipse {
                center,
                major,
                minor,
            },
            [0.0; 3],
            fov.axis(),
            &fov.boundary,
        )
    }
}
fn artificial_ellipsoid(fov: &Fov) -> Result<([f64; 3], Mat3), GeoError> {
    let axis = fov.axis();
    let center_direction = unit(axis);
    let (semi, lengths) = if fov.shape == FovShape::Circle {
        let second = rotate_axis(fov.boundary[0], axis, FRAC_PI_2);
        let plane = Plane {
            normal: unit(axis),
            constant: 1.0 / norm(axis),
        };
        let first = intersect_ray([0.0; 3], fov.boundary[0], plane)
            .ok_or(GeoError::BadSearch("invalid circular FOV"))?;
        let second = intersect_ray([0.0; 3], second, plane)
            .ok_or(GeoError::BadSearch("invalid circular FOV"))?;
        let semi = [sub(first, center_direction), sub(second, center_direction)];
        (semi, [norm(semi[0]), norm(semi[1])])
    } else {
        (fov.semi_axes, fov.semi_lengths)
    };
    let extension = lengths[0] * lengths[0];
    let center = scale(center_direction, extension + 1.0);
    let radius = lengths[0] * (lengths[0].powf(2.0) + 1.0).sqrt();
    let third = scale(center_direction, radius);
    let first = scale(unit(semi[0]), radius);
    let second_radius = lengths[1] / lengths[0] * radius;
    let second = scale(unit_cross(third, first), second_radius);
    let scaling = 1e-5 / (norm(center) + radius.max(second_radius));
    let axes = transpose(&[
        scale(first, scaling),
        scale(second, scaling),
        scale(third, scaling),
    ]);
    Ok((scale(center, scaling), axes))
}
fn ellipse_in_pyramid(
    ellipse: Ellipse,
    vertex: [f64; 3],
    axis: [f64; 3],
    bounds: &[[f64; 3]],
) -> Result<bool, GeoError> {
    let scaling = 1.0
        / norm(ellipse.center)
            .max(norm(ellipse.major))
            .max(norm(vertex));
    let center = scale(ellipse.center, scaling);
    let major = scale(ellipse.major, scaling);
    let minor = scale(ellipse.minor, scaling);
    let apex = scale(vertex, scaling);
    let (packed_major, packed_minor) = ellipse_axes(major, minor);
    let packed = Ellipse {
        center,
        major: packed_major,
        minor: packed_minor,
    };
    let a = norm(major);
    let b = norm(minor);
    if b == 0.0 || norm(axis) == 0.0 || bounds.len() < 3 {
        return Err(GeoError::BadSearch("invalid ellipse/pyramid geometry"));
    }
    let center_vector = sub(center, apex);
    let mut pyramid_size = 0.0_f64;
    for (i, &edge) in bounds.iter().enumerate() {
        let separation = angle(axis, edge);
        if separation >= FRAC_PI_2 || unit_cross(edge, bounds[(i + 1) % bounds.len()]) == [0.0; 3] {
            return Err(GeoError::BadSearch("invalid pyramid boundary"));
        }
        pyramid_size = pyramid_size.max(separation);
    }
    let distance = norm(sub(center, apex));
    if a < distance && angle(center_vector, axis) - ((a / distance).asin() + pyramid_size) > 0.0 {
        return Ok(false);
    }
    if dot(axis, center_vector) > 0.0 {
        let plane = plane_point(axis, center_vector);
        let mut vertices = Vec::with_capacity(bounds.len());
        for &edge in bounds {
            vertices.push(
                intersect_ray([0.0; 3], edge, plane)
                    .ok_or(GeoError::BadSearch("pyramid plane intersection failed"))?,
            );
        }
        if winding(plane, &vertices, center_vector) != 0 {
            return Ok(true);
        }
    }
    let ellipse_plane = plane_spans(center, major, minor);
    if let Some(p) = intersect_ray(apex, bounds[0], ellipse_plane) {
        let offset = sub(p, center);
        let x = dot(offset, major) / a;
        let y = dot(offset, minor) / b;
        if (x / a).powf(2.0) + (y / b).powf(2.0) <= 1.0 {
            return Ok(true);
        }
    }
    for i in 0..bounds.len() {
        let next = (i + 1) % bounds.len();
        let plane = plane_spans(apex, bounds[i], bounds[next]);
        if let Some((p1, p2)) = ellipse_plane_intersection(packed, plane) {
            let e1 = unit(bounds[i]);
            let e2 = unit(bounds[next]);
            let eb = add(scale(e1, 0.5), scale(e2, 0.5));
            let half_edge = angle(e1, e2) / 2.0;
            let v1 = unit(sub(p1, apex));
            let v2 = unit(sub(p2, apex));
            let vb = add(scale(v1, 0.5), scale(v2, 0.5));
            let half_sector = angle(v1, v2) / 2.0;
            if angle(v1, eb) <= half_edge
                || angle(v2, eb) <= half_edge
                || angle(vb, eb) <= half_edge
                || angle(eb, vb) <= half_sector
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn ellipse_plane_intersection(ellipse: Ellipse, plane: Plane) -> Option<([f64; 3], [f64; 3])> {
    let point = sub(scale(plane.normal, plane.constant), ellipse.center);
    let transformed = plane_point(plane.normal, point);
    let x = dot(ellipse.major, transformed.normal);
    let y = dot(ellipse.minor, transformed.normal);
    if x == 0.0 && y == 0.0 {
        return None;
    }
    let maximum = x.abs().max(y.abs());
    let sx = x / maximum;
    let sy = y / maximum;
    let length = maximum * (sx * sx + sy * sy).sqrt();
    if length < transformed.constant {
        return None;
    }
    let alpha = (transformed.constant / length).acos();
    let beta = y.atan2(x);
    let one = beta - alpha;
    let two = beta + alpha;
    let evaluate = |theta: f64| {
        add(
            add(ellipse.center, scale(ellipse.major, theta.cos())),
            scale(ellipse.minor, theta.sin()),
        )
    };
    Some((evaluate(one), evaluate(two)))
}
fn winding(plane: Plane, vertices: &[[f64; 3]], point: [f64; 3]) -> i32 {
    let mut normal = plane.normal;
    if dot(normal, vertices[0]) < 0.0 {
        normal = neg(normal);
    }
    let mut previous = perpendicular(sub(vertices[0], point), normal);
    let mut total = 0.0;
    for i in 1..=vertices.len() {
        let next = perpendicular(sub(vertices[i % vertices.len()], point), normal);
        let separation = angle(next, previous);
        let tangent = unit_cross(normal, previous);
        if dot(next, tangent) >= 0.0 {
            total += separation;
        } else {
            total -= separation;
        }
        previous = next;
    }
    (total / TAU).round() as i32
}
fn plane_point(normal: [f64; 3], point: [f64; 3]) -> Plane {
    let mut normal = unit(normal);
    let mut constant = dot(point, normal);
    if constant < 0.0 {
        normal = neg(normal);
        constant = -constant;
    }
    Plane { normal, constant }
}
fn rotate_axis(v: [f64; 3], axis: [f64; 3], theta: f64) -> [f64; 3] {
    let axis = unit(axis);
    let projected = project(v, axis);
    let plane = sub(v, projected);
    let other = cross(axis, plane);
    add(
        add(scale(plane, theta.cos()), scale(other, theta.sin())),
        projected,
    )
}
fn rotate(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(v: [f64; 3], s: f64) -> [f64; 3] {
    [s * v[0], s * v[1], s * v[2]]
}
fn neg(v: [f64; 3]) -> [f64; 3] {
    [-v[0], -v[1], -v[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[cfg(test)]
mod tests {
    #[test]
    fn zzelvupy_exact_spicelib() {
        for line in include_str!("../../tests/golden/fov_pyramid_precision.csv").lines() {
            let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
            let ellipse = super::Ellipse {
                center: [v[0], v[1], v[2]],
                major: [v[3], v[4], v[5]],
                minor: [v[6], v[7], v[8]],
            };
            let bounds = [
                [v[15], v[16], v[17]],
                [v[18], v[19], v[20]],
                [v[21], v[22], v[23]],
                [v[24], v[25], v[26]],
            ];
            let actual = super::ellipse_in_pyramid(
                ellipse,
                [v[9], v[10], v[11]],
                [v[12], v[13], v[14]],
                &bounds,
            )
            .unwrap();
            assert_eq!(actual, v[27] != 0.0, "{line}");
        }
    }
    #[test]
    fn inelpl_exact_spicelib() {
        let mut count = 0;
        for line in include_str!("../../tests/golden/fov_inelpl_precision.csv").lines() {
            let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
            let ellipse = super::Ellipse {
                center: [v[0], v[1], v[2]],
                major: [v[3], v[4], v[5]],
                minor: [v[6], v[7], v[8]],
            };
            let plane = super::Plane {
                normal: [v[9], v[10], v[11]],
                constant: v[12],
            };
            let actual = super::ellipse_plane_intersection(ellipse, plane);
            assert_eq!(actual.is_some(), v[13] > 0.0);
            if let Some((a, b)) = actual {
                for (a, b) in a.into_iter().chain(b).zip(&v[14..]) {
                    if a.to_bits() != b.to_bits() {
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(count, 0);
    }
}
