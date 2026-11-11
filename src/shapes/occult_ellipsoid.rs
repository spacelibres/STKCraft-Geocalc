//! SPICELIB ZZOCCED ellipsoid classification, preserving its bounding
//! cone tests, scaled limb construction, and ZZASRYEL extrema search.
use crate::surface::{norm, unit, unit_cross};
use crate::tangent::{ellipse_axes, orthonormal_frame};
use crate::{
    error::GeoError,
    frame::{mat_mul, transpose, Mat3},
    occult::OccultationState,
};
use std::f64::consts::{FRAC_PI_2, PI, TAU};
#[derive(Clone, Copy)]
pub(crate) struct Ellipse {
    pub center: [f64; 3],
    pub major: [f64; 3],
    pub minor: [f64; 3],
}
#[derive(Clone, Copy)]
pub(crate) struct Plane {
    pub normal: [f64; 3],
    pub constant: f64,
}

pub(crate) fn classify_ellipsoids(
    front_position: [f64; 3],
    front_radii: [f64; 3],
    front_rotation: &Mat3,
    back_position: [f64; 3],
    back_radii: [f64; 3],
    back_rotation: &Mat3,
) -> Result<OccultationState, GeoError> {
    let code = classify_raw(
        [back_position, front_position],
        [back_radii, front_radii],
        [*back_rotation, *front_rotation],
    )?;
    Ok(match code {
        -3 => OccultationState::Full,
        -2 => OccultationState::Annular,
        -1 => OccultationState::Partial,
        _ => OccultationState::None,
    })
}
fn classify_raw(
    centers: [[f64; 3]; 2],
    input_radii: [[f64; 3]; 2],
    input_rotations: [Mat3; 2],
) -> Result<i32, GeoError> {
    let axis_vectors = [
        [
            scale(input_rotations[0][0], input_radii[0][0]),
            scale(input_rotations[0][1], input_radii[0][1]),
            scale(input_rotations[0][2], input_radii[0][2]),
        ],
        [
            scale(input_rotations[1][0], input_radii[1][0]),
            scale(input_rotations[1][1], input_radii[1][1]),
            scale(input_rotations[1][2], input_radii[1][2]),
        ],
    ];
    classify_axes([0.0; 3], centers, axis_vectors)
}
/// Signed ZZOCCED classification for arbitrary viewing point and semi-axis matrices.
/// Each matrix column is one semi-axis vector in the common reference frame.
pub(crate) fn classify_semiaxes(
    view: [f64; 3],
    center1: [f64; 3],
    axes1: &Mat3,
    center2: [f64; 3],
    axes2: &Mat3,
) -> Result<i32, GeoError> {
    classify_axes(
        view,
        [center1, center2],
        [transpose(axes1), transpose(axes2)],
    )
}
fn classify_axes(
    viewpoint: [f64; 3],
    centers: [[f64; 3]; 2],
    axis_vectors: [Mat3; 2],
) -> Result<i32, GeoError> {
    let mut radii = [[0.0; 3]; 2];
    let mut rotations = [[[0.0; 3]; 3]; 2];
    for b in 0..2 {
        for i in 0..3 {
            let axis = axis_vectors[b][i];
            radii[b][i] = norm(axis);
            rotations[b][i] = unit(axis);
        }
    }
    let positions = [sub(centers[0], viewpoint), sub(centers[1], viewpoint)];
    let distances = [norm(positions[0]), norm(positions[1])];
    let minima = radii.map(|r| r[0].min(r[1]).min(r[2]));
    let maxima = radii.map(|r| r[0].max(r[1]).max(r[2]));
    if minima.iter().any(|&r| r <= 0.0) {
        return Err(GeoError::BadSearch("ellipsoid axes must be positive"));
    }
    if distances.contains(&0.0) || norm(sub(positions[1], positions[0])) <= minima[0] + minima[1] {
        return Err(GeoError::BadSearch(
            "occultation targets or observer are not disjoint",
        ));
    }
    let local = [
        rotate(&rotations[0], positions[0]),
        rotate(&rotations[1], positions[1]),
    ];
    let mut minang = [0.0; 2];
    let mut maxang = [0.0; 2];
    for i in 0..2 {
        let q = [
            local[i][0] / radii[i][0],
            local[i][1] / radii[i][1],
            local[i][2] / radii[i][2],
        ];
        if dot(q, q) < 1.0 {
            return Err(GeoError::BadSearch("observer is inside occultation target"));
        }
        minang[i] = safe_asin(minima[i] / distances[i])?;
        maxang[i] = if distances[i] >= maxima[i] {
            safe_asin(maxima[i] / distances[i])?
        } else {
            let view = neg(local[i]);
            let limb = edlimb(radii[i], view)?;
            maximum_cone(limb, view)
        };
    }
    let separation = angle(positions[0], positions[1]);
    if separation > maxang[0] + maxang[1] {
        return Ok(0);
    }
    let big = if maxang[0] > maxang[1] { 0 } else { 1 };
    let small = 1 - big;
    if minang[0] + minang[1] > separation {
        let angular = angle(neg(positions[0]), sub(positions[1], positions[0]));
        let (front, sign) = if angular < FRAC_PI_2 - minang[0] {
            (1, -1)
        } else {
            (0, 1)
        };
        for i in 0..2 {
            if minang[i] >= maxang[1 - i] + separation {
                return Ok(sign * if front == i { 3 } else { 2 });
            }
        }
        if minang[0] + separation > maxang[1] && minang[1] + separation > maxang[0] {
            return Ok(sign);
        }
    }
    let local_view = rotate(&rotations[small], sub(viewpoint, centers[small]));
    let limb = edlimb(radii[small], local_view)?;
    let reverse = transpose(&rotations[small]);
    let limb = Ellipse {
        center: add(centers[small], rotate(&reverse, limb.center)),
        major: rotate(&reverse, limb.major),
        minor: rotate(&reverse, limb.minor),
    };
    let r = radii[big];
    let scaling = [
        [1.0 / r[0], 0.0, 0.0],
        [0.0, 1.0 / r[1], 0.0],
        [0.0, 0.0, 1.0 / r[2]],
    ];
    let transformation = mat_mul(&scaling, &rotations[big]);
    let view = rotate(&transformation, viewpoint);
    let big_center = rotate(&transformation, centers[big]);
    let center = rotate(&transformation, limb.center);
    let (major, minor) = ellipse_axes(
        rotate(&transformation, limb.major),
        rotate(&transformation, limb.minor),
    );
    // CGV2EL performs SAELGV again even though these axes are orthogonal.
    let (major, minor) = ellipse_axes(major, minor);
    let limb = Ellipse {
        center,
        major,
        minor,
    };
    let ray = sub(big_center, view);
    let (minimum, minpoint) = angular_extremum(true, limb, view, ray)?;
    let radius = safe_asin(1.0 / norm(ray))?;
    if minimum > radius {
        return Ok(0);
    }
    let small_front = angle(sub(minpoint, big_center), neg(ray)) <= FRAC_PI_2 - radius;
    let sign = if (small_front && small == 0) || (!small_front && small == 1) {
        1
    } else {
        -1
    };
    if minimum <= -radius {
        return Ok(sign * if small_front { 3 } else { 2 });
    }
    let from_limb = sub(view, limb.center);
    let distance = norm(from_limb);
    let maxcone = if distance >= norm(limb.major) {
        safe_asin(norm(limb.major) / distance)?
    } else {
        maximum_cone(limb, view)
    };
    let mut tilt = angle(unit_cross(limb.major, limb.minor), from_limb);
    if tilt > FRAC_PI_2 {
        tilt = PI - tilt;
    }
    let minorlength = norm(limb.minor);
    let mincone = (tilt.cos() * minorlength).atan2(tilt.sin() * minorlength + distance);
    let separation = angle(ray, sub(limb.center, view));
    if separation + maxcone <= radius {
        return Ok(sign * if small_front { 2 } else { 3 });
    }
    if separation + mincone > radius {
        return Ok(sign);
    }
    let (maximum, _) = angular_extremum(false, limb, view, ray)?;
    Ok(if maximum.abs() <= radius {
        sign * if small_front { 2 } else { 3 }
    } else {
        sign
    })
}
fn maximum_cone(limb: Ellipse, view: [f64; 3]) -> f64 {
    let plane = plane_spans(limb.center, limb.major, limb.minor);
    let projection = project_plane(view, plane);
    let height = norm(sub(view, projection));
    let upper = norm(sub(projection, limb.center)) + norm(limb.major);
    upper.atan2(height)
}
pub(crate) fn edlimb(radii: [f64; 3], view: [f64; 3]) -> Result<Ellipse, GeoError> {
    let scaling = radii[0].max(radii[1]).max(radii[2]);
    let axes = radii.map(|r| r / scaling);
    let v = scale(view, 1.0 / scaling);
    let square = axes.map(|a| a * a);
    let level = v[0] * v[0] / square[0] + v[1] * v[1] / square[1] + v[2] * v[2] / square[2];
    if level < 1.0 {
        return Err(GeoError::BadSearch("viewing point is inside ellipsoid"));
    }
    let normal = [v[0] / square[0], v[1] / square[1], v[2] / square[2]];
    let limb = intersect_plane(axes, plane_normal(normal, 1.0))?;
    Ok(Ellipse {
        center: scale(limb.center, scaling),
        major: scale(limb.major, scaling),
        minor: scale(limb.minor, scaling),
    })
}
fn intersect_plane(axes: [f64; 3], plane: Plane) -> Result<Ellipse, GeoError> {
    let point = scale(plane.normal, plane.constant);
    let (_, span1, span2) = orthonormal_frame(plane.normal);
    let inverse = axes.map(|a| 1.0 / a);
    let distorted = |v: [f64; 3]| [inverse[0] * v[0], inverse[1] * v[1], inverse[2] * v[2]];
    let dp = plane_spans(distorted(point), distorted(span1), distorted(span2));
    let center = scale(dp.normal, dp.constant);
    let (_, vec1, vec2) = orthonormal_frame(dp.normal);
    let distance = norm(center);
    if distance > 1.0 {
        return Err(GeoError::BadSearch(
            "ellipsoid limb plane has no intersection",
        ));
    }
    let radius = (1.0 - distance * distance).clamp(0.0, 1.0).sqrt();
    let vec1 = scale(vec1, radius);
    let vec2 = scale(vec2, radius);
    let distort_back = |v: [f64; 3]| [axes[0] * v[0], axes[1] * v[1], axes[2] * v[2]];
    let (major, minor) = ellipse_axes(distort_back(vec1), distort_back(vec2));
    Ok(Ellipse {
        center: distort_back(center),
        major,
        minor,
    })
}
pub(crate) fn plane_normal(normal: [f64; 3], constant: f64) -> Plane {
    let length = norm(normal);
    let mut n = unit(normal);
    let mut c = constant / length;
    if c < 0.0 {
        c = -c;
        n = neg(n);
    }
    Plane {
        normal: n,
        constant: c,
    }
}
pub(crate) fn plane_spans(center: [f64; 3], a: [f64; 3], b: [f64; 3]) -> Plane {
    let mut n = unit_cross(a, b);
    let mut c = dot(n, center);
    if c < 0.0 {
        c = -c;
        n = neg(n);
    }
    Plane {
        normal: n,
        constant: c,
    }
}
pub(crate) fn project_plane(v: [f64; 3], p: Plane) -> [f64; 3] {
    add(v, scale(p.normal, p.constant - dot(v, p.normal)))
}
pub(crate) fn intersect_ray(vertex: [f64; 3], dir: [f64; 3], p: Plane) -> Option<[f64; 3]> {
    let direction = unit(dir);
    let scaling = p.constant.max(norm(vertex));
    let v = if scaling != 0.0 {
        scale(vertex, 1.0 / scaling)
    } else {
        vertex
    };
    let c = if scaling != 0.0 {
        p.constant / scaling
    } else {
        p.constant
    };
    let difference = c - dot(v, p.normal);
    let projection = dot(direction, p.normal);
    if difference == 0.0 {
        return Some(vertex);
    }
    if projection == 0.0 || (projection > 0.0) != (difference > 0.0) {
        return None;
    }
    let factor = difference.abs() / projection.abs();
    Some(scale(add(v, scale(direction, factor)), scaling))
}
fn angular_extremum(
    minimum: bool,
    ellipse: Ellipse,
    vertex: [f64; 3],
    dir: [f64; 3],
) -> Result<(f64, [f64; 3]), GeoError> {
    let Ellipse {
        center,
        major,
        minor,
    } = ellipse;
    let a = norm(major);
    let b = norm(minor);
    let plane = plane_spans(center, major, minor);
    if a == 0.0
        || b == 0.0
        || norm(dir) == 0.0
        || norm(sub(vertex, project_plane(vertex, plane))) == 0.0
    {
        return Err(GeoError::BadSearch("degenerate angular ellipse geometry"));
    }
    let mut points = 400usize;
    let mut sign = 1.0;
    if let Some(intersection) = intersect_ray(vertex, dir, plane) {
        let offset = sub(intersection, center);
        let ac = dot(offset, major) / a;
        let bc = dot(offset, minor) / b;
        let level = ac * ac / (a * a) + bc * bc / (b * b);
        if level <= 1.0 {
            sign = -1.0;
        } else if minimum {
            points = 320;
        }
    }
    let v2 = sub(vertex, center);
    let direction = unit(dir);
    let delta = TAU / points as f64;
    let mut extreme = if minimum { 2.0 } else { 0.0 };
    let mut index = 0;
    let vector = |theta: f64| {
        add(
            add(scale(v2, -1.0), scale(major, theta.cos())),
            scale(minor, theta.sin()),
        )
    };
    let mut diff = [0.0; 3];
    for i in 0..points {
        diff = vector(i as f64 * delta);
        let proxy = norm(sub(unit(diff), direction));
        if (minimum && proxy < extreme) || (!minimum && proxy > extreme) {
            extreme = proxy;
            index = i;
        }
    }
    let ratio = (3.0 - 5.0_f64.sqrt()) / 2.0;
    let mut lower = TAU / points as f64 * (index as f64 - 1.0) + TAU;
    let mut upper = TAU / points as f64 * (index as f64 + 1.0) + TAU;
    let mut between = TAU / points as f64 * index as f64 + TAU;
    let mut middle_proxy = extreme;
    for _ in 0..=100 {
        if upper - lower <= 1e-9 {
            break;
        }
        let mut new_point = if between - lower > upper - between {
            lower + ratio * (between - lower)
        } else {
            between + ratio * (upper - between)
        };
        diff = vector(new_point);
        let mut proxy = norm(sub(unit(diff), direction));
        if new_point > between {
            std::mem::swap(&mut between, &mut new_point);
            std::mem::swap(&mut middle_proxy, &mut proxy);
        }
        if (minimum && proxy > middle_proxy) || (!minimum && proxy < middle_proxy) {
            lower = new_point;
        } else {
            upper = between;
            between = new_point;
            middle_proxy = proxy;
        }
    }
    let point = add(center, add(diff, v2));
    Ok((angle(diff, direction) * sign, point))
}
/// Minimum angular radius of the apparent limb about the center ray.
pub(crate) fn limb_angular_radius(
    center: [f64; 3],
    radii: [f64; 3],
    rotation: &Mat3,
) -> Result<f64, GeoError> {
    let view = neg(rotate(rotation, center));
    let limb = edlimb(radii, view)?;
    Ok(angular_extremum(true, limb, view, neg(view))?.0.abs())
}
fn safe_asin(value: f64) -> Result<f64, GeoError> {
    if value.abs() - 1e-12 > 1.0 {
        return Err(GeoError::BadSearch(
            "angular radius exceeds valid asin range",
        ));
    }
    Ok(value.clamp(-1.0, 1.0).asin())
}
pub(crate) fn angle(a: [f64; 3], b: [f64; 3]) -> f64 {
    if norm(a) == 0.0 || norm(b) == 0.0 {
        return 0.0;
    }
    let a = unit(a);
    let b = unit(b);
    let d = dot(a, b);
    if d > 0.0 {
        2.0 * (norm(sub(a, b)) * 0.5).asin()
    } else if d < 0.0 {
        PI - 2.0 * (norm(add(a, b)) * 0.5).asin()
    } else {
        FRAC_PI_2
    }
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
#[cfg(test)]
mod tests {
    #[test]
    fn exact_zzocced_states() {
        for line in include_str!("../../tests/golden/occult_ellipsoid_precision.csv").lines() {
            let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
            let fm = [
                [v[6], v[7], v[8]],
                [v[9], v[10], v[11]],
                [v[12], v[13], v[14]],
            ];
            let bm = [
                [v[21], v[22], v[23]],
                [v[24], v[25], v[26]],
                [v[27], v[28], v[29]],
            ];
            let actual = super::classify_ellipsoids(
                [v[0], v[1], v[2]],
                [v[3], v[4], v[5]],
                &fm,
                [v[15], v[16], v[17]],
                [v[18], v[19], v[20]],
                &bm,
            )
            .unwrap();
            let expected = match v[30] as i32 {
                -3 => crate::occult::OccultationState::Full,
                -2 => crate::occult::OccultationState::Annular,
                -1 => crate::occult::OccultationState::Partial,
                _ => crate::occult::OccultationState::None,
            };
            assert_eq!(actual, expected, "{line}");
        }
    }
}
#[cfg(test)]
mod helper_precision {
    #[test]
    fn limb_exact_spicelib() {
        let mut differences = 0;
        let mut worst = 0.0_f64;
        for line in include_str!("../../tests/golden/occult_limb_precision.csv").lines() {
            let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
            let a = super::edlimb([v[0], v[1], v[2]], [v[3], v[4], v[5]]).unwrap();
            for (a, b) in a
                .center
                .into_iter()
                .chain(a.major)
                .chain(a.minor)
                .zip(&v[6..])
            {
                if a.to_bits() != b.to_bits() {
                    differences += 1;
                }
                worst = worst.max((a - b).abs());
            }
        }
        eprintln!("EDLIMB {differences}differences,max{worst}");
        assert_eq!(differences, 0);
    }
    #[test]
    fn angular_extrema_exact_spicelib() {
        let mut differences = 0;
        let mut worst = 0.0_f64;
        for line in include_str!("../../tests/golden/occult_angle_precision.csv").lines() {
            let f: Vec<_> = line.split(',').collect();
            let v: Vec<f64> = f[1..].iter().map(|x| x.parse().unwrap()).collect();
            let el = super::Ellipse {
                center: [v[0], v[1], v[2]],
                major: [v[3], v[4], v[5]],
                minor: [v[6], v[7], v[8]],
            };
            let (angle, p) = super::angular_extremum(
                f[0] == "MIN",
                el,
                [v[9], v[10], v[11]],
                [v[12], v[13], v[14]],
            )
            .unwrap();
            for (a, b) in [angle].into_iter().chain(p).zip(&v[15..]) {
                if a.to_bits() != b.to_bits() {
                    differences += 1;
                }
                worst = worst.max((a - b).abs());
            }
        }
        eprintln!("ZZASRYEL {differences}differences,max{worst}");
        assert_eq!(differences, 0);
    }
}

/// Both apparent limb angular radii about the observer-to-center ray.
pub(crate) fn limb_angular_extents(
    center: [f64; 3],
    radii: [f64; 3],
    rotation: &Mat3,
) -> Result<[f64; 2], GeoError> {
    let view = neg(rotate(rotation, center));
    let limb = edlimb(radii, view)?;
    Ok([
        angular_extremum(true, limb, view, neg(view))?.0.abs(),
        angular_extremum(false, limb, view, neg(view))?.0.abs(),
    ])
}
