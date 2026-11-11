//! Source-order DSK coordinate-volume bounds and ray intersections.
use crate::{dsk::DskDescriptor, GeoError};
pub(crate) fn ray_volume_intercept(
    vertex: [f64; 3],
    direction: [f64; 3],
    d: &DskDescriptor,
    margin: f64,
) -> Result<Option<[f64; 3]>, GeoError> {
    if d.coordinate_system == 3 {
        let b = d.coordinate_bounds;
        let l: [f64; 3] = std::array::from_fn(|i| b[2 * i + 1] - b[2 * i]);
        if l.iter().any(|v| *v <= 0.) {
            return Err(GeoError::BadDas("SPICE(BADCOORDBOUNDS)"));
        }
        if d.contains_excluding(vertex, margin * 2., 0)? {
            return Ok(Some(vertex));
        }
        let delta = l.map(|v| margin * v.abs());
        return crate::dsk::ray_box(
            vertex,
            direction,
            std::array::from_fn(|i| b[2 * i] - delta[i]),
            std::array::from_fn(|i| l[i] + delta[i] * 2.),
        );
    }
    ray_curved_volume(vertex, direction, d, margin)
}
pub(crate) fn segment_bounding_sphere(d: &DskDescriptor) -> Result<([f64; 3], f64), GeoError> {
    let b = d.coordinate_bounds;
    if d.coordinate_system == 3 {
        let l: [f64; 3] = std::array::from_fn(|i| b[2 * i + 1] - b[2 * i]);
        if l.iter().any(|x| *x <= 0.) {
            return Err(GeoError::BadDas("SPICE(BOUNDSOUTOFORDER)"));
        }
        return Ok((
            std::array::from_fn(|i| b[2 * i] + l[i] / 2.),
            norm(l.map(|x| x / 2.)),
        ));
    }
    if d.coordinate_system != 1 && d.coordinate_system != 4 {
        return Err(GeoError::BadDas("SPICE(NOTSUPPORTED)"));
    }
    let minlon = b[0];
    let mut maxlon = b[1];
    if maxlon <= minlon {
        maxlon += std::f64::consts::TAU;
    }
    if maxlon <= minlon {
        return Err(GeoError::BadDas("SPICE(BADLONGITUDERANGE)"));
    }
    let hpi = std::f64::consts::FRAC_PI_2;
    if b[2] > b[3] {
        return Err(GeoError::BadDas("SPICE(BADLATITUDEBOUNDS)"));
    }
    if b[2] < -hpi - 1e-12 || b[3] > hpi + 1e-12 {
        return Err(GeoError::BadDas("SPICE(BADLATITUDERANGE)"));
    }
    let minlat = b[2].max(-hpi);
    let maxlat = b[3].min(hpi);
    let (outrad, inrad, minz, maxz) = if d.coordinate_system == 1 {
        let (outer, inner) = if minlat >= 0. {
            (minlat.cos() * b[5], maxlat.cos() * b[4])
        } else if maxlat <= 0. {
            (maxlat.cos() * b[5], minlat.cos() * b[4])
        } else {
            (b[5], maxlat.abs().max(minlat.abs()).cos() * b[4])
        };
        let (lo, hi) = if minlat >= 0. {
            (b[4] * minlat.sin(), b[5] * maxlat.sin())
        } else if maxlat <= 0. {
            (b[5] * minlat.sin(), b[4] * maxlat.sin())
        } else {
            (b[5] * minlat.sin(), b[5] * maxlat.sin())
        };
        (outer, inner, lo, hi)
    } else {
        let re = d.coordinate_parameters[0];
        let f = d.coordinate_parameters[1];
        if re <= 0. || f >= 1. {
            return Err(GeoError::BadDas("SPICE(VALUEOUTOFRANGE)"));
        }
        let radial = |lat, alt| {
            let mut p = georec(lat, alt, re, f);
            p[2] = 0.;
            norm(p)
        };
        let (outer, inner) = if minlat >= 0. {
            (radial(minlat, b[5]), radial(maxlat, b[4]))
        } else if maxlat <= 0. {
            (radial(maxlat, b[5]), radial(minlat, b[4]))
        } else {
            (re + b[5], radial(maxlat.abs().max(minlat.abs()), b[4]))
        };
        let (lo, hi) = if minlat >= 0. {
            (
                georec(minlat, b[4], re, f)[2],
                georec(maxlat, b[5], re, f)[2],
            )
        } else if maxlat < 0. {
            (
                georec(minlat, b[5], re, f)[2],
                georec(maxlat, b[4], re, f)[2],
            )
        } else {
            (
                georec(minlat, b[5], re, f)[2],
                georec(maxlat, b[5], re, f)[2],
            )
        };
        (outer, inner, lo, hi)
    };
    let hdlon = (maxlon - minlon) / 2.;
    let midlon = minlon + hdlon;
    let lr = if hdlon <= hpi {
        outrad - inrad * hdlon.cos()
    } else {
        (1. - hdlon.cos()) * outrad
    };
    let lt = if hdlon > hpi {
        outrad * 2.
    } else {
        outrad * 2. * hdlon.sin()
    };
    let lr = lr.max(0.);
    let lt = lt.max(0.);
    let lz = (maxz - minz).max(0.);
    let midr = outrad - lr / 2.;
    Ok((
        [midr * midlon.cos(), midr * midlon.sin(), minz + lz / 2.],
        norm([lr / 2., lt / 2., lz / 2.]),
    ))
}
use crate::surface::{norm, unit};
fn georec(lat: f64, alt: f64, re: f64, f: f64) -> [f64; 3] {
    let rp = re - f * re;
    let cp = lat.cos();
    let sp = lat.sin();
    let big = (re * cp).abs().max((rp * sp).abs());
    let x = re * cp / big;
    let y = rp * sp / big;
    let factor = 1. / (big * (x * x + y * y).sqrt());
    let base = [
        factor * re * re * 1. * cp,
        factor * re * re * 0. * cp,
        factor * rp * rp * sp,
    ];
    let m = re.min(rp);
    let a = m / re;
    let c = m / rp;
    let normal = unit([base[0] * (a * a), base[1] * (a * a), base[2] * (c * c)]);
    std::array::from_fn(|i| 1. * base[i] + alt * normal[i])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(sub(a, b))
}
fn point_on(v: [f64; 3], s: f64, u: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| v[i] + s * u[i])
}
// ZZBQUAD returns bounded roots in its original (cancellation-safe) order.
fn bounded_quadratic(a: f64, b: f64, c: f64, ub: f64) -> Vec<f64> {
    if a == 0. {
        if b != 0. && c.abs() <= (ub * b).abs() {
            return vec![-c / b];
        }
        return vec![];
    }
    let disc = b * b - a * 4. * c;
    if disc < 0. {
        return vec![];
    }
    if disc == 0. {
        if (-b).abs() >= (a * 2. * ub).abs() {
            return vec![];
        }
        return vec![-b / a / 2.];
    }
    let sq = disc.sqrt();
    let (num2, num1) = if b > 0. {
        (-b - sq, -b + sq)
    } else {
        (-b + sq, -b - sq)
    };
    if num2.abs() <= (ub * (a * 2.)).abs() {
        let r2 = num2 / a / 2.;
        vec![if r2.abs() > 0. { c / (a * r2) } else { 0. }, r2]
    } else if num1.abs() <= (ub * (a * 2.)).abs() {
        vec![num1 / a / 2.]
    } else {
        vec![]
    }
}
fn cone_quadratic(a: f64, b: f64, c: f64, ub: f64) -> Vec<f64> {
    if a == 0. && b == 0. {
        return vec![];
    }
    let maxmag = a.abs().max(b.abs()).max(c.abs());
    let mut coeff = [a / maxmag, b / maxmag, c / maxmag];
    let mut maxix = 0;
    for i in 1..3 {
        if coeff[i].abs() > coeff[maxix].abs() {
            maxix = i;
        }
    }
    coeff[maxix] = if coeff[maxix] >= 0. { 1. } else { -1. };
    if coeff[0].abs() >= 1e-8 || coeff[0] == 0. {
        return bounded_quadratic(coeff[0], coeff[1], coeff[2], ub);
    }
    if coeff[2].abs() >= 1e-8 {
        let roots = bounded_quadratic(coeff[2], coeff[1], coeff[0], f64::MAX.sqrt() / 200.);
        if roots.len() == 1 {
            return if (roots[0] * ub).abs() >= 1. {
                vec![1. / roots[0]]
            } else {
                vec![]
            };
        }
        if roots.len() == 2 && (roots[1] * ub).abs() >= 1. {
            let mut out = vec![1. / roots[1]];
            if (roots[0] * ub).abs() >= 1. {
                out.push(1. / roots[0]);
            }
            return out;
        }
        return vec![];
    }
    if b < 0. {
        coeff = coeff.map(|v| -v);
    }
    let mut out = vec![];
    if coeff[2].abs() <= ub {
        out.push(-coeff[2]);
        if (coeff[0] * coeff[2] - 1.).abs() < (coeff[0] * ub).abs() {
            out.push(coeff[2] - 1. / coeff[0]);
        }
    }
    out
}
// INRYPL, for an already constructed normal/constant plane.
fn ray_plane(
    v: [f64; 3],
    direction: [f64; 3],
    normal: [f64; 3],
    constant: f64,
) -> Option<[f64; 3]> {
    let u = unit(direction);
    let m = constant.max(norm(v));
    let (sv, sc) = if m != 0. {
        (v.map(|x| (1. / m) * x), constant / m)
    } else {
        (v, constant)
    };
    let diff = sc - dot(sv, normal);
    if diff == 0. {
        return if dot(normal, u) == 0. { None } else { Some(v) };
    }
    let projection = dot(u, normal);
    if !(projection > 0. && diff > 0. || projection < 0. && diff < 0.) {
        return None;
    }
    let mut big = f64::MAX / 3.;
    if m > 1. {
        big /= m;
    }
    if diff.abs() >= projection.abs() * big {
        return None;
    }
    let scale = diff.abs() / projection.abs();
    Some(point_on(sv, scale, u).map(|x| m * x))
}
fn cone_bisect(
    apex: [f64; 3],
    axis: [f64; 3],
    angle: f64,
    p1: [f64; 3],
    p2: [f64; 3],
) -> Option<[f64; 3]> {
    let (angle, axis) = if angle > std::f64::consts::FRAC_PI_2 {
        (std::f64::consts::PI - angle, axis.map(|x| -x))
    } else {
        (angle, axis)
    };
    let axis = unit(axis);
    let c = angle.cos();
    let off1 = sub(p1, apex);
    let off2 = sub(p2, apex);
    let state1 = dot(unit(off1), axis) >= c;
    if state1 == (dot(unit(off2), axis) >= c) {
        return None;
    }
    let seg = sub(off2, off1);
    let mut low = 0.;
    let mut high = 1.;
    let mut delta = 1.;
    let mut previous = 2.;
    let mut count = 0;
    let mut x = [0.; 3];
    while delta > 1e-15 && delta < previous && count < 1000 {
        let mid = (low + high) / 2.;
        x = point_on(off1, mid, seg);
        if (dot(unit(x), axis) >= c) == state1 {
            low = mid
        } else {
            high = mid
        }
        previous = delta;
        delta = (high - low).abs();
        count += 1;
    }
    Some(std::array::from_fn(|i| apex[i] + x[i]))
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn segment_max_latitude(p1: [f64; 3], p2: [f64; 3]) -> (f64, [f64; 3]) {
    let lat1 = crate::dsk::reclat(p1).2;
    let lat2 = crate::dsk::reclat(p2).2;
    let (mut lat, mut point) = if lat1 <= lat2 { (lat2, p2) } else { (lat1, p1) };
    let normal = cross(p1, p2);
    if normal == [0.; 3] {
        return (lat, point);
    }
    let normal = unit(normal);
    let crease = cross([0., 0., 1.], normal);
    if crease == [0.; 3] {
        return (lat, point);
    }
    let crease = unit(crease);
    let a = dot(p1, crease);
    let b = dot(p2, crease);
    if a > 0. && b < 0. || a < 0. && b > 0. {
        let crease = unit(crease);
        if let Some(p) = ray_plane(p1, sub(p2, p1), crease, 0.) {
            let l = crate::dsk::reclat(p).2;
            if l > lat {
                lat = l;
                point = p;
            }
        }
    }
    (lat, point)
}
// INCNSG's callers here use the Z axis. Keep its FRAME/rotation fallback order.
fn cone_segment(
    apex: [f64; 3],
    angle: f64,
    p1: [f64; 3],
    p2: [f64; 3],
) -> Result<Vec<[f64; 3]>, GeoError> {
    if angle < 0. {
        return Err(GeoError::BadDas("SPICE(INVALIDANGLE)"));
    }
    let direction = sub(p2, p1);
    let dmag = norm(direction);
    if dmag == 0. {
        return Err(GeoError::BadDas("SPICE(ENDPOINTSMATCH)"));
    }
    let u = unit(direction);
    let mut axis = [0., 0., 1.];
    let mut cosine = angle.cos();
    let mut locang = angle;
    if cosine < 0. {
        cosine = -cosine;
        locang = std::f64::consts::PI - angle;
        axis = axis.map(|x| -x);
    }
    let off1 = sub(p1, apex);
    let off2 = sub(p2, apex);
    let dp1 = dot(unit(off1), axis);
    let dp2 = dot(unit(off2), axis);
    let in1 = dp1 >= cosine;
    let in2 = dp2 >= cosine;
    if in1 && in2 || dp1 < 0. && dp2 < 0. {
        return Ok(vec![]);
    }
    if cosine.abs() < 1e-14 {
        let mut constant = dot(apex, axis);
        let mut normal = axis;
        if constant < 0. {
            constant = -constant;
            normal = normal.map(|x| -x);
        }
        return Ok(ray_plane(p1, u, normal, constant)
            .filter(|p| distance(*p, p1) <= dmag)
            .into_iter()
            .collect());
    }
    let uu = dot(u, axis);
    let wuax = dot(off1, axis);
    let wu = dot(off1, u);
    let w2 = dot(off1, off1);
    let ca2 = cosine * cosine;
    let roots = cone_quadratic(
        uu * uu - ca2,
        (wuax * uu - ca2 * wu) * 2.,
        wuax * wuax - w2 * ca2,
        dmag,
    );
    let mut points = Vec::new();
    for s in roots {
        if s >= 0. {
            let p = point_on(p1, s, u);
            let v = sub(p, apex);
            let mag = norm(v);
            let error = if mag > 0. {
                (dot(unit(v), axis) - cosine).abs()
            } else {
                0.
            };
            if mag == 0. || error < 1e-10 {
                points.push(p);
            }
        }
    }
    if points.len() < 2 {
        if in1 != in2 {
            if points.is_empty() {
                if let Some(p) = cone_bisect(apex, axis, locang, p1, p2) {
                    points.push(p);
                }
            }
        } else {
            points.clear();
            // FRAME(+/-Z) gives X=(0,-/+1,0), Y=(1,0,0).
            let axis = unit(axis);
            let x = [0., -axis[2], 0.];
            let y = [1., -axis[0] * x[2], axis[0] * x[1]];
            let matrix = [x, y, axis];
            let rotate = |v| [dot(matrix[0], v), dot(matrix[1], v), dot(matrix[2], v)];
            let back = |v: [f64; 3]| -> [f64; 3] {
                std::array::from_fn(|i| {
                    matrix[0][i] * v[0] + matrix[1][i] * v[1] + matrix[2][i] * v[2]
                })
            };
            let q1 = rotate(off1);
            let q2 = rotate(off2);
            let (maxlat, maxp) = segment_max_latitude(q1, q2);
            let colat = std::f64::consts::FRAC_PI_2 - maxlat;
            if colat < locang {
                for (a, b) in [(q1, maxp), (maxp, q2)] {
                    if let Some(p) = cone_bisect([0.; 3], [0., 0., 1.], locang, a, b) {
                        let p = back(p);
                        points.push(std::array::from_fn(|i| p[i] + apex[i]));
                    }
                }
            } else if colat == locang {
                points.push(maxp);
            }
        }
    }
    Ok(points)
}

fn sphere_intercept(vertex: [f64; 3], u: [f64; 3], r: f64) -> Option<[f64; 3]> {
    let parallel = dot(vertex, u);
    let perp = point_on(vertex, -parallel, u);
    let pmag2 = dot(perp, perp);
    let r2 = r * r;
    if pmag2 > r2 {
        return None;
    }
    let s = (r2 - pmag2).max(0.).sqrt();
    let vmag2 = dot(vertex, vertex);
    if vmag2 > r2 {
        if parallel > 0. {
            None
        } else {
            Some(std::array::from_fn(|i| perp[i] - s * u[i]))
        }
    } else if vmag2 < r2 {
        Some(point_on(perp, s, u))
    } else {
        Some(perp)
    }
}
fn short_ray_plane(v: [f64; 3], u: [f64; 3], normal: [f64; 3], maxd: f64) -> Option<[f64; 3]> {
    let vc = dot(v, normal);
    let dc = dot(u, normal);
    if vc > 0. && dc > 0. || vc < 0. && dc < 0. {
        return None;
    }
    if vc == 0. {
        return if dc != 0. { Some(v) } else { None };
    }
    let h = vc.abs();
    let l = dc.abs();
    if h > maxd * l || l == 0. {
        return None;
    }
    Some(point_on(v, h / l, u))
}
fn ellipse_normal_intercepts(re: f64, rp: f64, lat: f64) -> (f64, f64) {
    let scale = re.max(rp);
    let a = re / scale;
    let b = rp / scale;
    let nx = lat.cos();
    let ny = lat.sin();
    let na = nx * a * a;
    let nb = ny * b * b;
    let lambda = (na * nx + nb * ny + 0.).powf(-0.5) * scale;
    let r = rp / re;
    let s = re / rp;
    ((1. - r * r) * (lambda * na), (1. - s * s) * (lambda * nb))
}
fn valid_planetodetic_cone(re: f64, f: f64, p: [f64; 3], lat: f64) -> bool {
    if f <= 0. || p[2] * lat >= 0. {
        return true;
    }
    if lat.abs() >= 0.01 {
        return false;
    }
    let (x, _) = ellipse_normal_intercepts(re, re * (1. - f), lat);
    let r2 = p[0] * p[0] + p[1] * p[1];
    r2.max(0.).sqrt() >= x
}
fn ray_curved_volume(
    vertex: [f64; 3],
    direction: [f64; 3],
    d: &DskDescriptor,
    margin: f64,
) -> Result<Option<[f64; 3]>, GeoError> {
    let pdt = d.coordinate_system == 4;
    if !pdt && d.coordinate_system != 1 {
        return Err(GeoError::BadDas("SPICE(BADCOORDSYS)"));
    }
    if pdt && direction == [0.; 3] {
        return Err(GeoError::BadDas("SPICE(ZEROVECTOR)"));
    }
    if d.contains_excluding(vertex, if pdt { margin } else { margin * 2. }, 0)? {
        return Ok(Some(vertex));
    }
    let b = d.coordinate_bounds;
    let (minlon, maxlon) = crate::dsk::normalized_longitude(b[0], b[1])?;
    let hpi = std::f64::consts::FRAC_PI_2;
    let re = d.coordinate_parameters[0];
    let f = d.coordinate_parameters[1];
    let rp = re * (1. - f);
    let (inner, outer) = if pdt {
        let low = b[4] - margin * b[4].abs();
        let high = b[5] + margin * b[5].abs();
        let (emin, pmin, emax, pmax) = if re >= rp {
            crate::dsk::bounding_ellipses(re, rp, low, high)?
        } else {
            let (a, b, c, d) = crate::dsk::bounding_ellipses(rp, re, low, high)?;
            (b, a, d, c)
        };
        ([emin, emin, pmin], [emax, emax, pmax])
    } else {
        ([b[4]; 3], [b[5]; 3])
    };
    let u = unit(direction);
    let intersect = |v, dir, axes: [f64; 3]| -> Result<Option<[f64; 3]>, GeoError> {
        if pdt {
            crate::surface::ray_surface_intersection(axes, v, dir)
        } else {
            Ok(sphere_intercept(v, dir, axes[0]))
        }
    };
    let Some(point) = intersect(vertex, u, outer)? else {
        return Ok(None);
    };
    let mut hit = None;
    let mut minimum = f64::MAX;
    if d.contains_excluding(point, margin, 3)? {
        hit = Some(point);
        let outside = if pdt {
            let a = vertex[0] / outer[0];
            let b = vertex[1] / outer[1];
            let c = vertex[2] / outer[2];
            a * a + b * b + c * c > 1.
        } else {
            crate::dsk::reclat(vertex).0 > b[5]
        };
        if outside {
            return Ok(hit);
        }
        minimum = distance(vertex, point);
    }
    let maxr = outer[0].max(outer[2]);
    let s = norm(vertex) + maxr * 1.1;
    let endpoint = std::array::from_fn(|i| 1. * vertex[i] + s * u[i]);
    let vlat = crate::dsk::reclat(vertex).2;
    for upper in [true, false] {
        let lat = if upper { b[3] } else { b[2] };
        if upper && lat >= hpi || !upper && lat <= -hpi {
            continue;
        }
        let angle = if upper {
            (hpi - lat).max(0.)
        } else {
            hpi - lat
        };
        let apex = if pdt {
            [0., 0., ellipse_normal_intercepts(re, rp, lat).1]
        } else {
            [0.; 3]
        };
        let mut points = cone_segment(apex, angle, vertex, endpoint)?;
        let vertex_angle = if pdt {
            separation(sub(vertex, apex), [0., 0., 1.])
        } else {
            0.
        };
        if pdt {
            points.retain(|p| valid_planetodetic_cone(re, f, *p, lat));
        }
        for (index, p) in points.iter().enumerate() {
            if d.contains_excluding(*p, margin, 2)? {
                let dist = distance(vertex, *p);
                if dist < minimum {
                    hit = Some(*p);
                    if index == 0 {
                        let early = if pdt {
                            if upper {
                                vertex_angle < angle && (lat < 0. || vertex[2] > 0.)
                            } else {
                                vertex_angle > angle && (lat > 0. || vertex[2] < 0.)
                            }
                        } else if upper {
                            vlat > b[3]
                        } else {
                            vlat < b[2]
                        };
                        if early {
                            return Ok(hit);
                        }
                    } else if !upper {
                        return Ok(hit);
                    }
                    minimum = dist;
                }
            }
        }
    }
    if (maxlon - minlon).cos() < 1. {
        let s = (norm(vertex) + maxr) * 1.1;
        let west = [minlon.sin(), -minlon.cos(), 0.];
        let east = [-maxlon.sin(), maxlon.cos(), 0.];
        let z = [0., 0., 1.];
        for (normal, back) in [
            (west, crate::surface::unit_cross(west, z)),
            (east, crate::surface::unit_cross(z, east)),
        ] {
            if let Some(p) = short_ray_plane(vertex, u, normal, s) {
                if d.contains_excluding(p, margin, 1)? && dot(p, back) < margin * maxr {
                    let dist = distance(vertex, p);
                    if dist < minimum {
                        hit = Some(p);
                        minimum = dist;
                    }
                }
            }
        }
    }
    if pdt || b[4] > 0. {
        for (v, dir) in [(vertex, u), (endpoint, u.map(|x| -x))] {
            if let Some(p) = intersect(v, dir, inner)? {
                if d.contains_excluding(p, margin, 3)? {
                    let dist = distance(vertex, p);
                    if dist < minimum {
                        hit = Some(p);
                        minimum = dist;
                    }
                }
            }
        }
    }
    Ok(hit)
}
fn separation(a: [f64; 3], b: [f64; 3]) -> f64 {
    let an = norm(a);
    let bn = norm(b);
    if an == 0. || bn == 0. {
        return 0.;
    }
    let u = a.map(|x| x / an);
    let v = b.map(|x| x / bn);
    if dot(u, v) > 0. {
        2. * (0.5 * norm(sub(u, v))).asin()
    } else {
        std::f64::consts::PI - 2. * (0.5 * norm(std::array::from_fn(|i| u[i] + v[i]))).asin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_cone_quadratic_exact() {
        let mut count = 0;
        for line in include_str!("../../tests/golden/dsk/volume_quadratic.csv")
            .lines()
            .skip(1)
        {
            let v: Vec<f64> = line.split(',').map(|s| s.parse().unwrap()).collect();
            let roots = cone_quadratic(v[0], v[1], v[2], v[3]);
            assert_eq!(roots.len(), v[4].max(0.) as usize, "case {count}");
            for (i, r) in roots.iter().enumerate() {
                assert_eq!(
                    r.to_bits(),
                    v[5 + i].to_bits(),
                    "case {count} root {i}: {r} vs {}",
                    v[5 + i]
                );
            }
            count += 1;
        }
        assert_eq!(count, 6412);
    }
    #[test]
    fn native_cone_segment_exact() {
        let mut count = 0;
        for line in include_str!("../../tests/golden/dsk/volume_cones.csv")
            .lines()
            .skip(1)
        {
            let v: Vec<f64> = line.split(',').map(|s| s.parse().unwrap()).collect();
            let points = cone_segment(
                v[0..3].try_into().unwrap(),
                v[3],
                v[4..7].try_into().unwrap(),
                v[7..10].try_into().unwrap(),
            )
            .unwrap();
            assert_eq!(points.len(), v[10] as usize, "case {count}");
            for (i, p) in points.iter().enumerate() {
                for j in 0..3 {
                    assert_eq!(
                        p[j].to_bits(),
                        v[11 + 3 * i + j].to_bits(),
                        "case {count} point {i} axis {j}: {} vs {}",
                        p[j],
                        v[11 + 3 * i + j]
                    );
                }
            }
            count += 1;
        }
        assert_eq!(count, 4200);
    }
    fn descriptor(v: &[f64]) -> DskDescriptor {
        let mut raw = [0.; 24];
        raw.copy_from_slice(&v[..24]);
        DskDescriptor {
            surface_id: raw[0] as i32,
            center: raw[1] as i32,
            data_class: raw[2] as i32,
            data_type: raw[3] as i32,
            frame_id: raw[4] as i32,
            coordinate_system: raw[5] as i32,
            coordinate_parameters: raw[6..16].try_into().unwrap(),
            coordinate_bounds: raw[16..22].try_into().unwrap(),
            start: raw[22],
            end: raw[23],
            raw,
        }
    }
    #[test]
    fn native_volume_bounds_exact() {
        let mut count = 0;
        for line in include_str!("../../tests/golden/dsk/volume_bounds.csv")
            .lines()
            .skip(1)
        {
            let v: Vec<f64> = line.split(',').map(|s| s.parse().unwrap()).collect();
            let (center, radius) = segment_bounding_sphere(&descriptor(&v)).unwrap();
            for (i, actual) in center.into_iter().chain([radius]).enumerate() {
                assert_eq!(
                    actual.to_bits(),
                    v[24 + i].to_bits(),
                    "case {count} field {i}: {actual} vs {}",
                    v[24 + i]
                );
            }
            count += 1;
        }
        assert_eq!(count, 180);
    }
    #[test]
    fn native_volume_rays_exact() {
        let mut count = 0;
        for line in include_str!("../../tests/golden/dsk/volume_rays.csv")
            .lines()
            .skip(1)
        {
            let v: Vec<f64> = line.split(',').map(|s| s.parse().unwrap()).collect();
            let hit = ray_volume_intercept(
                v[24..27].try_into().unwrap(),
                v[27..30].try_into().unwrap(),
                &descriptor(&v),
                v[30],
            )
            .unwrap();
            assert_eq!(hit.is_some(), v[31] == 1., "case {count}");
            if let Some(point) = hit {
                for i in 0..3 {
                    assert_eq!(
                        point[i].to_bits(),
                        v[32 + i].to_bits(),
                        "case {count} field {i}: {} vs {}",
                        point[i],
                        v[32 + i]
                    );
                }
            }
            count += 1;
        }
        assert_eq!(count, 4320);
    }
    #[test]
    fn native_volume_boundary_rays_exact() {
        let mut count = 0;
        for line in include_str!("../../tests/golden/dsk/volume_edges.csv")
            .lines()
            .skip(1)
        {
            let v: Vec<f64> = line.split(',').map(|s| s.parse().unwrap()).collect();
            let hit = ray_volume_intercept(
                v[24..27].try_into().unwrap(),
                v[27..30].try_into().unwrap(),
                &descriptor(&v),
                v[30],
            )
            .unwrap();
            assert_eq!(hit.is_some(), v[31] == 1., "case {count}");
            if let Some(point) = hit {
                for i in 0..3 {
                    assert_eq!(
                        point[i].to_bits(),
                        v[32 + i].to_bits(),
                        "case {count} field {i}: {} vs {}",
                        point[i],
                        v[32 + i]
                    );
                }
            }
            count += 1;
        }
        assert_eq!(count, 2016);
    }
}
