//! Native type-2 plate membership (ZZPTPL02).
use crate::{dsk::DskSegment, GeoError};
pub(crate) fn plate_at_point(
    segment: &DskSegment,
    point: [f64; 3],
) -> Result<Option<i32>, GeoError> {
    if point.iter().any(|x| !x.is_finite()) {
        return Err(GeoError::BadSearch("DSK point must be finite"));
    }
    let p = segment.integer_parameters;
    let size = segment.voxel_size;
    let coarse = p[6];
    let ext: [i32; 3] = p[3..6].try_into().unwrap();
    if size <= 0. || coarse <= 0 || ext.iter().any(|n| *n < 1 || *n % coarse != 0) {
        return Err(GeoError::BadDas("invalid DSK voxel grid"));
    }
    let (_, radius) = crate::dsk_volume::segment_bounding_sphere(&segment.descriptor)?;
    // N0067 DSKGTL defaults: point/surface membership margin and plate expansion.
    let margin = 1e-7;
    let expansion = 1e-10;
    let limit = margin * radius;
    if !segment.descriptor.contains_excluding(point, margin, 0)? {
        return Ok(None);
    }
    let offset = sub(point, segment.voxel_origin);
    let grid: [i32; 3] = std::array::from_fn(|i| ((offset[i] / size) as i32 + 1).clamp(1, ext[i]));
    let cg = grid.map(|n| (n - 1) / coarse + 1);
    let fine: [i32; 3] = std::array::from_fn(|i| grid[i] - coarse * (cg[i] - 1));
    let ptroff = (fine[2] - 1) * coarse * coarse + (fine[1] - 1) * coarse + fine[0];
    let cgext = ext.map(|n| n / coarse);
    let cgid = cg[0] + cgext[0] * (cg[1] + cg[2] * cgext[1] - (cgext[1] + 1));
    let pointer_base = 10 + 3 * segment.plates.len();
    let list_base = pointer_base + p[7] as usize;
    let coarse_base = list_base + p[8] as usize + segment.vertices.len() + p[9] as usize;
    let item = |base: usize, index: i32| -> Result<i32, GeoError> {
        if index < 1 {
            return Err(GeoError::BadDas("invalid DSK spatial index pointer"));
        }
        segment
            .integer_data
            .get(base + index as usize - 1)
            .copied()
            .ok_or(GeoError::BadDas("DSK spatial index outside segment"))
    };
    let cgptr = item(coarse_base, cgid)?;
    if cgptr < 1 {
        return Ok(None);
    }
    let pltptr = item(pointer_base, cgptr - 1 + ptroff)?;
    if pltptr < 1 {
        return Ok(None);
    }
    let count = item(list_base, pltptr)?;
    if count < 1 {
        return Ok(None);
    }
    let mut minimum = f64::MAX;
    let mut result = None;
    for i in 1..=count {
        let id = item(list_base, pltptr + i)?;
        let ids = segment
            .plates
            .get((id - 1) as usize)
            .ok_or(GeoError::BadDas("DSK plate ID outside segment"))?;
        let mut original = [[0.; 3]; 3];
        for k in 0..3 {
            original[k] = *segment
                .vertices
                .get((ids[k] - 1) as usize)
                .ok_or(GeoError::BadDas("DSK vertex ID outside segment"))?;
        }
        let vertices = expand_plate(original, expansion);
        let normal = crate::surface::unit(cross(
            sub(vertices[1], vertices[0]),
            sub(vertices[2], vertices[1]),
        ));
        let offset = sub(point, vertices[0]);
        let dist = if dot(offset, normal).abs() <= limit {
            plate_nearest(point, vertices).1
        } else {
            f64::MAX
        };
        if dist <= limit && dist < minimum {
            minimum = dist;
            result = Some(id);
        }
    }
    Ok(result)
}
use crate::surface::norm;
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(sub(a, b))
}
fn expand_plate(v: [[f64; 3]; 3], delta: f64) -> [[f64; 3]; 3] {
    let d = delta / 3.;
    let center: [f64; 3] = std::array::from_fn(|i| d * (v[0][i] + v[1][i] + v[2][i]));
    let s = delta + 1.;
    std::array::from_fn(|j| std::array::from_fn(|i| s * v[j][i] - center[i]))
}
fn segment_nearest(a: [f64; 3], b: [f64; 3], point: [f64; 3]) -> ([f64; 3], f64) {
    let seg = sub(b, a);
    if seg == [0.; 3] {
        return (a, distance(a, point));
    }
    let line = add(crate::surface::project(sub(point, a), seg), a);
    let offdot = dot(sub(line, a), seg);
    if offdot < 0. {
        (a, distance(a, point))
    } else if offdot > dot(seg, seg) {
        (b, distance(b, point))
    } else {
        (line, distance(line, point))
    }
}
// PLTNP and NPSGPT retain source edge and tie ordering, including degeneracy.
fn plate_nearest(point: [f64; 3], v: [[f64; 3]; 3]) -> ([f64; 3], f64) {
    let [v1, v2, v3] = v;
    let e1 = sub(v2, v1);
    let e2 = sub(v3, v2);
    let e3 = sub(v1, v3);
    let normal = cross(e1, e2);
    let en1 = cross(e1, normal);
    let en2 = cross(e2, normal);
    let en3 = cross(e3, normal);
    if [normal, en1, en2, en3].contains(&[0.; 3]) {
        let l1 = norm(e1);
        let l2 = norm(e2);
        let l3 = norm(e3);
        if l1 == 0. && l2 == 0. {
            return (v1, distance(v1, point));
        }
        return if l1 > l2.max(l3) {
            segment_nearest(v1, v2, point)
        } else if l2 > l3.max(l1) {
            segment_nearest(v2, v3, point)
        } else {
            segment_nearest(v3, v1, point)
        };
    }
    let perp = crate::surface::perpendicular(sub(point, v1), normal);
    let in1 = dot(perp, en1) <= 0.;
    let in2 = dot(perp, en2) <= dot(e1, en2);
    let in3 = dot(perp, en3) <= 0.;
    if in1 && in2 && in3 {
        let near = add(v1, perp);
        return (near, distance(near, point));
    }
    if !in1 && in2 && in3 {
        return segment_nearest(v1, v2, point);
    }
    if !in2 && in3 && in1 {
        return segment_nearest(v2, v3, point);
    }
    if !in3 && in1 && in2 {
        return segment_nearest(v3, v1, point);
    }
    let p1 = segment_nearest(v1, v2, point);
    let p2 = segment_nearest(v2, v3, point);
    let p3 = segment_nearest(v3, v1, point);
    if p1.1 <= p2.1.min(p3.1) {
        p1
    } else if p2.1 <= p3.1.min(p1.1) {
        p2
    } else {
        p3
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_point_membership_exact() {
        let mut kernels = std::collections::HashMap::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/dsk/point_membership.csv")
            .lines()
            .skip(1)
        {
            let p: Vec<_> = line.split(',').collect();
            let kernel = kernels.entry(p[0]).or_insert_with(|| {
                crate::DskKernel::open(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/golden/dsk")
                        .join(p[0]),
                )
                .unwrap()
            });
            let segment = &kernel.segments()[p[1].parse::<usize>().unwrap()];
            let point = [
                p[2].parse().unwrap(),
                p[3].parse().unwrap(),
                p[4].parse().unwrap(),
            ];
            let actual = plate_at_point(segment, point).unwrap();
            let expected = if p[5] == "1" {
                Some(p[6].parse().unwrap())
            } else {
                None
            };
            assert_eq!(actual, expected, "case {count}: {line}");
            count += 1;
        }
        assert_eq!(count, 11502);
    }
    #[test]
    fn native_plate_nearest_exact() {
        let mut count = 0;
        for line in include_str!("../../tests/golden/dsk/plate_nearest.csv")
            .lines()
            .skip(1)
        {
            let v: Vec<f64> = line.split(',').map(|s| s.parse().unwrap()).collect();
            let (point, distance) = plate_nearest(
                v[9..12].try_into().unwrap(),
                [
                    v[0..3].try_into().unwrap(),
                    v[3..6].try_into().unwrap(),
                    v[6..9].try_into().unwrap(),
                ],
            );
            for (i, actual) in point.into_iter().chain([distance]).enumerate() {
                assert_eq!(
                    actual.to_bits(),
                    v[12 + i].to_bits(),
                    "case {count} field {i}: {actual} vs {}",
                    v[12 + i]
                );
            }
            count += 1;
        }
        assert_eq!(count, 2400);
    }
}
