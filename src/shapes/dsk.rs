// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(
    clippy::neg_multiply,
    clippy::excessive_precision,
    clippy::too_many_arguments
)]
//! Native DAS/DLA type-2 digital shape kernels.
use crate::GeoError;

/// DLA linked-list descriptor. Component bases are zero-based logical addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DlaDescriptor {
    pub backward_pointer: i32,
    pub forward_pointer: i32,
    pub integer_base: usize,
    pub integer_size: usize,
    pub double_base: usize,
    pub double_size: usize,
    pub character_base: usize,
    pub character_size: usize,
}
impl DlaDescriptor {
    pub fn as_raw(self) -> [i32; 8] {
        [
            self.backward_pointer,
            self.forward_pointer,
            self.integer_base as i32,
            self.integer_size as i32,
            self.double_base as i32,
            self.double_size as i32,
            self.character_base as i32,
            self.character_size as i32,
        ]
    }
}

/// Numeric DSK descriptor, including the source's full 24-double record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DskDescriptor {
    pub surface_id: i32,
    pub center: i32,
    pub data_class: i32,
    pub data_type: i32,
    pub frame_id: i32,
    pub coordinate_system: i32,
    pub coordinate_parameters: [f64; 10],
    pub coordinate_bounds: [f64; 6],
    pub start: f64,
    pub end: f64,
    pub raw: [f64; 24],
}

#[derive(Clone, Debug)]
pub struct DskSegment {
    pub dla: DlaDescriptor,
    pub descriptor: DskDescriptor,
    pub vertices: Vec<[f64; 3]>,
    /// One-based vertex IDs, matching DSKP02.
    pub plates: Vec<[i32; 3]>,
    pub vertex_bounds: [f64; 6],
    pub voxel_origin: [f64; 3],
    pub voxel_size: f64,
    pub integer_parameters: [i32; 10],
    pub(crate) integer_data: Vec<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DskHit {
    pub kernel_index: usize,
    pub segment_index: usize,
    /// Native DSKXSI IC(1), which can belong to a later tested segment.
    /// N0067 preserves the winning point/descriptor but does not restore IC.
    pub plate_id: i32,
    /// Plate that actually supplied the winning point and normal.
    pub geometric_plate_id: i32,
    pub point: [f64; 3],
    pub normal: [f64; 3],
}

#[derive(Clone, Debug)]
pub struct DskKernel {
    pub internal_name: String,
    segments: Vec<DskSegment>,
}
impl DskKernel {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, GeoError> {
        Self::from_bytes(std::fs::read(path)?)
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, GeoError> {
        let das = DasData::read(&bytes)?;
        if das.ints.len() < 3 || das.ints[0] != 1_000_000 {
            return Err(GeoError::BadDas("invalid DLA header"));
        }
        let mut pointer = das.ints[1];
        let mut seen = std::collections::HashSet::new();
        let mut segments = Vec::new();
        let mut previous = -1;
        while pointer != -1 {
            if pointer < 1 || !seen.insert(pointer) {
                return Err(GeoError::BadDas("invalid DLA segment link"));
            }
            let r = das
                .ints
                .get(pointer as usize - 1..pointer as usize + 7)
                .ok_or(GeoError::BadDas("truncated DLA descriptor"))?;
            if r[0] != previous || r[2..].iter().any(|v| *v < 0) {
                return Err(GeoError::BadDas("invalid DLA component bounds"));
            }
            let dla = DlaDescriptor {
                backward_pointer: r[0],
                forward_pointer: r[1],
                integer_base: r[2] as usize,
                integer_size: r[3] as usize,
                double_base: r[4] as usize,
                double_size: r[5] as usize,
                character_base: r[6] as usize,
                character_size: r[7] as usize,
            };
            let i = component(&das.ints, dla.integer_base, dla.integer_size)?;
            let d = component(&das.doubles, dla.double_base, dla.double_size)?;
            component(&das.characters, dla.character_base, dla.character_size)?;
            if d.len() < 24 {
                return Err(GeoError::BadDas("truncated DSK descriptor"));
            }
            let raw: [f64; 24] = d[..24].try_into().unwrap();
            let descriptor = DskDescriptor {
                surface_id: integer_double(raw[0])?,
                center: integer_double(raw[1])?,
                data_class: integer_double(raw[2])?,
                data_type: integer_double(raw[3])?,
                frame_id: integer_double(raw[4])?,
                coordinate_system: integer_double(raw[5])?,
                coordinate_parameters: raw[6..16].try_into().unwrap(),
                coordinate_bounds: raw[16..22].try_into().unwrap(),
                start: raw[22],
                end: raw[23],
                raw,
            };
            if descriptor.data_type != 2 {
                return Err(GeoError::UnsupportedDskType(descriptor.data_type));
            }
            if i.len() < 10 || d.len() < 34 || i[0] <= 0 || i[1] <= 0 {
                return Err(GeoError::BadDas("invalid type-2 component"));
            }
            let nv = i[0] as usize;
            let np = i[1] as usize;
            let vertex_words = component(
                d,
                34,
                nv.checked_mul(3)
                    .ok_or(GeoError::BadDas("invalid vertex count"))?,
            )?;
            let plate_words = component(
                i,
                10,
                np.checked_mul(3)
                    .ok_or(GeoError::BadDas("invalid plate count"))?,
            )?;
            let vertices = vertex_words
                .chunks_exact(3)
                .map(|v| v.try_into().unwrap())
                .collect();
            let plates: Vec<[i32; 3]> = plate_words
                .chunks_exact(3)
                .map(|v| v.try_into().unwrap())
                .collect();
            if plates.iter().flatten().any(|v| *v < 1 || *v as usize > nv) {
                return Err(GeoError::BadDas("invalid plate vertex ID"));
            }
            let segment = DskSegment {
                dla,
                descriptor,
                vertices,
                plates,
                vertex_bounds: d[24..30].try_into().unwrap(),
                voxel_origin: d[30..33].try_into().unwrap(),
                voxel_size: d[33],
                integer_parameters: i[..10].try_into().unwrap(),
                integer_data: i.to_vec(),
            };
            segment.validate_spatial_index()?;
            segments.push(segment);
            previous = pointer;
            pointer = r[1];
        }
        if previous != das.ints[2] {
            return Err(GeoError::BadDas("inconsistent final DLA pointer"));
        }
        Ok(Self {
            internal_name: das.internal_name,
            segments,
        })
    }
    pub fn segments(&self) -> &[DskSegment] {
        &self.segments
    }
}

fn component<T>(data: &[T], base: usize, size: usize) -> Result<&[T], GeoError> {
    let end = base
        .checked_add(size)
        .ok_or(GeoError::BadDas("component address overflow"))?;
    data.get(base..end)
        .ok_or(GeoError::BadDas("component exceeds logical array"))
}
fn integer_double(value: f64) -> Result<i32, GeoError> {
    if !value.is_finite()
        || value.fract() != 0.
        || value < i32::MIN as f64
        || value > i32::MAX as f64
    {
        return Err(GeoError::BadDas("invalid integer descriptor field"));
    }
    Ok(value as i32)
}
struct DasData {
    internal_name: String,
    ints: Vec<i32>,
    doubles: Vec<f64>,
    characters: Vec<u8>,
}
impl DasData {
    fn read(bytes: &[u8]) -> Result<Self, GeoError> {
        if bytes.len() < 1024 || &bytes[..8] != b"DAS/DSK " {
            return Err(GeoError::NotDas);
        }
        if bytes.len() % 1024 != 0 {
            return Err(GeoError::BadDas("partial physical record"));
        }
        let little = match &bytes[84..92] {
            b"LTL-IEEE" => true,
            b"BIG-IEEE" => false,
            _ => return Err(GeoError::BadDas("unsupported DAS binary format")),
        };
        let word = |b: &[u8]| -> i32 {
            let a = b.try_into().unwrap();
            if little {
                i32::from_le_bytes(a)
            } else {
                i32::from_be_bytes(a)
            }
        };
        let number = |b: &[u8]| -> f64 {
            let a = b.try_into().unwrap();
            if little {
                f64::from_le_bytes(a)
            } else {
                f64::from_be_bytes(a)
            }
        };
        let nrr = word(&bytes[68..72]);
        let ncr = word(&bytes[76..80]);
        if nrr < 0 || ncr < 0 {
            return Err(GeoError::BadDas("negative reserved or comment count"));
        }
        let mut directory = nrr as usize + ncr as usize + 2;
        let mut seen = std::collections::HashSet::new();
        let mut data = Self {
            internal_name: String::from_utf8_lossy(&bytes[8..68])
                .trim_end()
                .to_string(),
            ints: Vec::new(),
            doubles: Vec::new(),
            characters: Vec::new(),
        };
        let mut maxima = [0; 3];
        let mut previous = 0;
        while directory != 0 {
            if directory > bytes.len() / 1024 || !seen.insert(directory) {
                return Err(GeoError::BadDas("invalid directory link"));
            }
            let record = &bytes[(directory - 1) * 1024..directory * 1024];
            let dir: Vec<i32> = record.chunks_exact(4).map(word).collect();
            if dir[0] != previous || dir[1] < 0 {
                return Err(GeoError::BadDas("invalid directory backward link"));
            }
            let mut offsets = [0; 3];
            for ty in 0..3 {
                let lo = dir[2 + 2 * ty];
                let hi = dir[3 + 2 * ty];
                if lo < 0 || hi < lo || (lo == 0 && hi != 0) {
                    return Err(GeoError::BadDas("invalid directory logical range"));
                }
                offsets[ty] = lo.saturating_sub(1) as usize;
                maxima[ty] = maxima[ty].max(hi as usize);
            }
            if !(1..=3).contains(&dir[8]) {
                if dir[2..8].iter().all(|v| *v == 0) {
                    break;
                }
                return Err(GeoError::BadDas("invalid cluster data type"));
            }
            let next = [2usize, 3, 1];
            let prev = [3usize, 1, 2];
            let mut kind = prev[dir[8] as usize - 1];
            let mut physical = directory + 1;
            for &count in &dir[9..] {
                if count == 0 {
                    break;
                }
                kind = if count > 0 {
                    next[kind - 1]
                } else {
                    prev[kind - 1]
                };
                let count = count.unsigned_abs() as usize;
                let end = physical
                    .checked_add(count)
                    .ok_or(GeoError::BadDas("cluster size overflow"))?;
                if end - 1 > bytes.len() / 1024 {
                    return Err(GeoError::BadDas("cluster exceeds file"));
                }
                let block = &bytes[(physical - 1) * 1024..(end - 1) * 1024];
                let n = [1024, 128, 256][kind - 1] * count;
                let offset = offsets[kind - 1];
                let limit = offset
                    .checked_add(n)
                    .ok_or(GeoError::BadDas("logical range overflow"))?;
                // Logical capacities cannot exceed the number of bytes in this file.
                if limit > bytes.len() {
                    return Err(GeoError::BadDas("invalid logical cluster address"));
                }
                match kind {
                    1 => {
                        data.characters.resize(data.characters.len().max(limit), 0);
                        data.characters[offset..limit].copy_from_slice(block);
                    }
                    2 => {
                        data.doubles.resize(data.doubles.len().max(limit), 0.);
                        for (to, from) in data.doubles[offset..limit]
                            .iter_mut()
                            .zip(block.chunks_exact(8))
                        {
                            *to = number(from);
                        }
                    }
                    3 => {
                        data.ints.resize(data.ints.len().max(limit), 0);
                        for (to, from) in data.ints[offset..limit]
                            .iter_mut()
                            .zip(block.chunks_exact(4))
                        {
                            *to = word(from);
                        }
                    }
                    _ => unreachable!(),
                }
                offsets[kind - 1] = limit;
                physical = end;
            }
            previous = directory as i32;
            directory = dir[1] as usize;
        }
        if maxima[0] > data.characters.len()
            || maxima[1] > data.doubles.len()
            || maxima[2] > data.ints.len()
        {
            return Err(GeoError::BadDas("logical range exceeds allocated records"));
        }
        data.characters.truncate(maxima[0]);
        data.doubles.truncate(maxima[1]);
        data.ints.truncate(maxima[2]);
        Ok(data)
    }
}

impl DskSegment {
    fn validate_spatial_index(&self) -> Result<(), GeoError> {
        let p = self.integer_parameters;
        if p.iter().any(|n| *n < 0)
            || p[0] as usize != self.vertices.len()
            || p[1] as usize != self.plates.len()
            || p[2] < 1
            || p[3..7].iter().any(|n| *n < 1)
            || self.voxel_size <= 0.
            || !self.voxel_size.is_finite()
            || self
                .voxel_origin
                .iter()
                .chain(self.vertices.iter().flatten())
                .any(|v| !v.is_finite())
        {
            return Err(GeoError::BadDas("invalid type-2 spatial index parameters"));
        }
        let coarse = p[6] as i64;
        let cube = coarse
            .checked_mul(coarse)
            .and_then(|n| n.checked_mul(coarse))
            .ok_or(GeoError::BadDas("coarse scale overflow"))?;
        let count = (p[3] as i64)
            .checked_mul(p[4] as i64)
            .and_then(|n| n.checked_mul(p[5] as i64))
            .ok_or(GeoError::BadDas("voxel dimensions overflow"))?;
        if count != p[2] as i64 || p[3..6].iter().any(|n| *n % p[6] != 0) || count % cube != 0 {
            return Err(GeoError::BadDas("inconsistent voxel grid dimensions"));
        }
        let total = 10i64
            + 3 * self.plates.len() as i64
            + p[7] as i64
            + p[8] as i64
            + self.vertices.len() as i64
            + p[9] as i64
            + count / cube;
        if total > self.integer_data.len() as i64 {
            return Err(GeoError::BadDas("truncated type-2 spatial index"));
        }
        Ok(())
    }
    /// Unit outward normal for a one-based plate ID (DSKN02).
    pub fn plate_normal(&self, plate_id: i32) -> Result<[f64; 3], GeoError> {
        if plate_id < 1 {
            return Err(GeoError::BadDas("plate ID out of range"));
        }
        let plate = self
            .plates
            .get((plate_id - 1) as usize)
            .ok_or(GeoError::BadDas("plate ID out of range"))?;
        let a = self.vertices[plate[0] as usize - 1];
        let b = self.vertices[plate[1] as usize - 1];
        let c = self.vertices[plate[2] as usize - 1];
        Ok(crate::surface::unit_cross(sub(b, a), sub(c, a)))
    }
    /// Intersect a ray in the segment's own reference frame, using DSKX02.
    pub fn ray_intercept(
        &self,
        vertex: [f64; 3],
        direction: [f64; 3],
    ) -> Result<Option<(i32, [f64; 3])>, GeoError> {
        self.validate_spatial_index()?;
        if direction == [0.; 3] {
            return Err(GeoError::BadSearch("ray direction is zero"));
        }
        if vertex
            .iter()
            .chain(direction.iter())
            .any(|v| !v.is_finite())
        {
            return Err(GeoError::BadSearch("ray must be finite"));
        }
        let unit = crate::surface::unit(direction);
        let size = self.voxel_size;
        if size <= 0. {
            return Err(GeoError::BadDas("voxel size must be positive"));
        }
        let p = self.integer_parameters;
        let ext: [i32; 3] = p[3..6].try_into().unwrap();
        let coarse = p[6];
        if coarse < 1 || ext.iter().any(|v| *v < 1 || *v % coarse != 0) {
            return Err(GeoError::BadDas("invalid voxel grid"));
        }
        let dimensions = ext.map(|n| n as f64 * size);
        let largest = dimensions[0].max(dimensions[1]).max(dimensions[2]);
        let Some(mut start) = ray_box(vertex, direction, self.voxel_origin, dimensions)? else {
            return Ok(None);
        };
        let coordinate = to_grid(start, self.voxel_origin, size);
        let voxels = voxel_walk(coordinate, unit, ext)?;
        let offset = sub(vertex, self.voxel_origin);
        let grid_margin = largest * 1e-12;
        if (0..3).any(|i| offset[i] < -grid_margin || offset[i] > grid_margin + dimensions[i]) {
            for i in 0..3 {
                start[i] = 1. * self.voxel_origin[i] + size * coordinate[i] + (-1.) * unit[i];
            }
        }
        let cgext = ext.map(|n| n / coarse);
        let cgsq = coarse * coarse;
        let voxel_pointer_base = 10 + 3 * self.plates.len();
        let voxel_list_base = voxel_pointer_base + p[7] as usize;
        let coarse_base = voxel_list_base + p[8] as usize + self.vertices.len() + p[9] as usize;
        let item = |base: usize, index: i32| -> Result<i32, GeoError> {
            if index < 1 {
                return Err(GeoError::BadDas("invalid spatial index pointer"));
            }
            self.integer_data
                .get(base + index as usize - 1)
                .copied()
                .ok_or(GeoError::BadDas("spatial index exceeds segment"))
        };
        let mut active = Vec::new();
        for voxel in voxels {
            let cg = voxel.map(|n| (n - 1) / coarse + 1);
            let id = cg[0] + cgext[0] * (cg[1] + cg[2] * cgext[1] - (cgext[1] + 1));
            let pointer = item(coarse_base, id)?;
            if pointer > 0 {
                active.push((voxel, cg, pointer));
            }
        }
        if active.is_empty() {
            return Ok(None);
        }
        let group_size = active.len().div_ceil(2).max(1);
        let mut closest = f64::MAX;
        let mut result = None;
        let tolerance = 1e-10 * largest;
        for group in active.chunks(group_size) {
            let mut candidates = Vec::new();
            for (vi, (voxel, cg, pointer)) in group.iter().enumerate() {
                let fine: [i32; 3] = std::array::from_fn(|j| voxel[j] - coarse * (cg[j] - 1));
                let offset = fine[0] + coarse * (fine[1] - 1) + cgsq * (fine[2] - 1);
                let list = item(voxel_pointer_base, pointer + offset - 1)?;
                if list != -1 {
                    let count = item(voxel_list_base, list)?;
                    if count < 0 {
                        return Err(GeoError::BadDas("negative voxel plate count"));
                    }
                    for n in 1..=count {
                        let plate = item(voxel_list_base, list + n)?;
                        if plate < 1 || plate as usize > self.plates.len() {
                            return Err(GeoError::BadDas("spatial index plate out of range"));
                        }
                        if candidates.len() >= 256000 {
                            return Err(GeoError::BadDas(
                                "voxel plate list exceeds native capacity",
                            ));
                        }
                        candidates.push((plate, vi));
                    }
                }
            }
            let mut tested = std::collections::HashSet::new();
            let mut final_voxel = usize::MAX;
            for (plate, vi) in candidates {
                if result.is_some() && vi > final_voxel {
                    break;
                }
                if !tested.insert(plate) {
                    continue;
                }
                let ids = self.plates[plate as usize - 1];
                let points = ids.map(|id| self.vertices[id as usize - 1]);
                let edges = points.map(|point| sub(point, start));
                let expanded = expand_plate(edges, 1e-10);
                let Some(scale) = solid_angle_intercept(unit, expanded) else {
                    continue;
                };
                let normal = cross(sub(points[1], points[0]), sub(points[2], points[1]));
                if dot(unit, normal) > 0. {
                    continue;
                }
                if result.is_none() || scale < closest {
                    let point = std::array::from_fn(|j| 1. * start[j] + scale * unit[j]);
                    let hit_coordinate = to_grid(point, self.voxel_origin, size);
                    let voxel = group[vi].0;
                    let inside = (0..3).all(|j| {
                        hit_coordinate[j] > voxel[j] as f64 - tolerance - 1.
                            && hit_coordinate[j] < voxel[j] as f64 + tolerance
                    });
                    if inside {
                        if self.descriptor.contains(point, 1e-8)? {
                            result = Some((plate, point));
                            closest = scale;
                            final_voxel = vi;
                        }
                    } else {
                        tested.remove(&plate);
                    }
                }
            }
            if result.is_some() {
                break;
            }
        }
        Ok(result)
    }
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
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
fn to_grid(point: [f64; 3], origin: [f64; 3], size: f64) -> [f64; 3] {
    std::array::from_fn(|i| (point[i] - origin[i]) / size)
}
// PLTEXP expands in the observer-centered frame; expanding global vertices
// before subtraction produces different last bits and edge predicates.
fn expand_plate(vertices: [[f64; 3]; 3], delta: f64) -> [[f64; 3]; 3] {
    let d = delta / 3.;
    let center: [f64; 3] =
        std::array::from_fn(|j| d * (vertices[0][j] + vertices[1][j] + vertices[2][j]));
    let scale = delta + 1.;
    vertices.map(|v| std::array::from_fn(|j| scale * v[j] - center[j]))
}
fn solid_angle_intercept(v: [f64; 3], edges: [[f64; 3]; 3]) -> Option<f64> {
    let n12 = cross(edges[0], edges[1]);
    let en = dot(edges[2], n12);
    let vn12 = dot(v, n12);
    if en == 0. || (en > 0. && vn12 < 0.) || (en < 0. && vn12 > 0.) {
        return None;
    }
    let vn23 = dot(v, cross(edges[1], edges[2]));
    if (en > 0. && vn23 < 0.) || (en < 0. && vn23 > 0.) {
        return None;
    }
    let vn31 = dot(v, cross(edges[2], edges[0]));
    if (en > 0. && vn31 < 0.) || (en < 0. && vn31 > 0.) {
        return None;
    }
    let denom = vn12 + vn23 + vn31;
    if denom == 0. {
        None
    } else {
        Some(en / denom)
    }
}
// ZZRAYBOX: first move to a bounding sphere, then select and sharpen the
// appropriate box face. The sphere step is material for remote observers.
pub(crate) fn ray_box(
    vertex: [f64; 3],
    direction: [f64; 3],
    origin: [f64; 3],
    extent: [f64; 3],
) -> Result<Option<[f64; 3]>, GeoError> {
    let unit = crate::surface::unit(direction);
    let center = std::array::from_fn(|i| origin[i] + extent[i] / 2.);
    let offset = sub(vertex, center);
    if (0..3).all(|i| offset[i].abs() <= extent[i] / 2.) {
        return Ok(Some(vertex));
    }
    let radius = crate::surface::norm(extent) * 0.50049999999999994;
    let sphere = if crate::surface::norm(offset) < radius {
        offset
    } else {
        let Some(hit) = crate::surface::ray_surface_intersection([radius; 3], offset, unit)? else {
            return Ok(None);
        };
        hit
    };
    let sphvtx: [f64; 3] = std::array::from_fn(|i| sphere[i] + center[i] - origin[i]);
    let sector: [i32; 3] = std::array::from_fn(|i| {
        if sphvtx[i] < 0. {
            1
        } else if sphvtx[i] > extent[i] {
            3
        } else {
            2
        }
    });
    let near: [f64; 3] = std::array::from_fn(|i| if sector[i] == 1 { 0. } else { extent[i] });
    let mut max_t = -1.;
    let mut max_index = 0;
    for i in 0..3 {
        if sector[i] != 2 {
            let distance = near[i] - sphvtx[i];
            let limit = radius * 2. * unit[i].abs();
            if distance.abs() > limit {
                return Ok(None);
            }
            let t = distance / unit[i];
            if t < 0. {
                return Ok(None);
            }
            if t > max_t {
                max_index = i;
                max_t = t;
            }
        }
    }
    if max_t < 0. {
        return Ok(None);
    }
    let mut point: [f64; 3] = std::array::from_fn(|i| 1. * sphvtx[i] + max_t * unit[i]);
    for i in 0..3 {
        if i == max_index {
            point[i] = near[i];
        } else if sector[i] == 2 {
            if point[i] < 0. || point[i] > extent[i] {
                return Ok(None);
            }
        } else if sector[i] == 1 {
            point[i] = point[i].max(0.);
            if point[i] > extent[i] {
                return Ok(None);
            }
        } else {
            point[i] = point[i].min(extent[i]);
            if point[i] < 0. {
                return Ok(None);
            }
        }
    }
    Ok(Some(std::array::from_fn(|i| point[i] + origin[i])))
}
// XDDA keeps source primary-axis error increments and its tie order.
fn voxel_walk(
    vertex: [f64; 3],
    direction: [f64; 3],
    extent: [i32; 3],
) -> Result<Vec<[i32; 3]>, GeoError> {
    let mut primary = 0;
    for i in 1..3 {
        if direction[i].abs() > direction[primary].abs() {
            primary = i;
        }
    }
    let axes = [primary, (primary + 1) % 3, (primary + 2) % 3];
    let mut coord: [i32; 3] =
        std::array::from_fn(|i| (vertex[axes[i]] as i32 + 1).clamp(1, extent[axes[i]]));
    let fraction: [f64; 3] =
        std::array::from_fn(|i| (vertex[axes[i]] - (coord[i] - 1) as f64).clamp(0., 1.));
    let limit = 1e-20 / extent[primary] as f64 * direction[primary].abs();
    let mut error = [f64::MAX; 2];
    let mut slopes = [0.; 2];
    for j in 1..3 {
        if direction[axes[j]].abs() > limit {
            let slope = (direction[primary] / direction[axes[j]]).abs();
            slopes[j - 1] = slope;
            let off = if direction[axes[j]] > 0. {
                1. - fraction[j]
            } else {
                fraction[j]
            };
            error[j - 1] = if direction[primary] > 0. {
                slope * off + fraction[0] - 1.
            } else {
                slope * off - fraction[0]
            };
        }
    }
    let step: [i32; 3] = std::array::from_fn(|i| {
        if direction[axes[i]] > limit {
            1
        } else if direction[axes[i]] < -limit {
            -1
        } else {
            0
        }
    });
    let mut output = Vec::new();
    loop {
        let mut voxel = [0; 3];
        for i in 0..3 {
            voxel[axes[i]] = coord[i];
        }
        if (0..3).any(|i| voxel[i] < 1 || voxel[i] > extent[i]) {
            break;
        }
        if output.len() >= 50000 {
            return Err(GeoError::BadDas("ray voxel list exceeds native capacity"));
        }
        output.push(voxel);
        if error[0] < 0. || error[1] < 0. {
            let j = if error[0] < error[1] { 0 } else { 1 };
            coord[j + 1] += step[j + 1];
            error[j] += slopes[j];
        } else {
            coord[0] += step[0];
            for j in 0..2 {
                if step[j + 1] != 0 {
                    error[j] += -1.;
                }
            }
        }
    }
    Ok(output)
}
impl DskDescriptor {
    fn contains(&self, point: [f64; 3], margin: f64) -> Result<bool, GeoError> {
        self.contains_excluding(point, margin, 0)
    }
    pub(crate) fn contains_excluding(
        &self,
        point: [f64; 3],
        margin: f64,
        excluded: i32,
    ) -> Result<bool, GeoError> {
        if margin < 0. || !(0..=3).contains(&excluded) {
            return Err(GeoError::BadDas(
                "invalid volume margin or excluded coordinate",
            ));
        }
        let b = self.coordinate_bounds;
        match self.coordinate_system {
            3 => Ok((0..3).all(|i| {
                let delta = margin * (b[2 * i + 1] - b[2 * i]).abs();
                excluded == i as i32 + 1
                    || (point[i] >= b[2 * i] - delta && point[i] <= b[2 * i + 1] + delta)
            })),
            1 => {
                let (r, lon, lat) = reclat(point);
                if r <= margin && b[4] <= margin {
                    return Ok(true);
                }
                if excluded != 3
                    && (r < 0_f64.max(b[4] * (1. - margin)) || r > b[5] * (margin + 1.))
                {
                    return Ok(false);
                }
                if excluded != 2
                    && (lat < (-std::f64::consts::FRAC_PI_2).max(b[2] - margin)
                        || lat > std::f64::consts::FRAC_PI_2.min(b[3] + margin))
                {
                    return Ok(false);
                }
                if excluded != 1 {
                    if margin == 0. {
                        longitude_inside_zero(lon, b[0], b[1])
                    } else {
                        longitude_inside(lon, lat, b[0], b[1], margin, true)
                    }
                } else {
                    Ok(true)
                }
            }
            4 => self.contains_planetodetic(point, margin, excluded),
            _ => Err(GeoError::Unsupported("DSK coordinate system".into())),
        }
    }
}

pub(crate) fn reclat(p: [f64; 3]) -> (f64, f64, f64) {
    let biggest = p[0].abs().max(p[1].abs()).max(p[2].abs());
    if biggest == 0. {
        return (0., 0., 0.);
    }
    let x = p[0] / biggest;
    let y = p[1] / biggest;
    let z = p[2] / biggest;
    let r = biggest * (x * x + y * y + z * z).sqrt();
    let lon = if p[0] == 0. && p[1] == 0. {
        0.
    } else {
        p[1].atan2(p[0])
    };
    let lat = z.atan2((x * x + y * y).sqrt());
    (r, lon, lat)
}
fn longitude_inside(
    mut lon: f64,
    lat: f64,
    lower: f64,
    upper: f64,
    margin: f64,
    skip_poles: bool,
) -> Result<bool, GeoError> {
    let (min, max) = normalized_longitude(lower, upper)?;
    let hpi = std::f64::consts::FRAC_PI_2;
    if skip_poles && (lat > hpi - 1e-8 || lat < -hpi + 1e-8) {
        return Ok(true);
    }
    let delta = 1e-12_f64.max(margin.abs()) / lat.cos().abs().max(1e-8);
    let lower = min - delta;
    let upper = max + delta;
    if lon < lower {
        if lon < lower - 1e-12 {
            lon += std::f64::consts::TAU
        } else {
            lon = lower;
        }
    } else if lon > upper {
        if lon > upper + 1e-12 {
            lon -= std::f64::consts::TAU
        } else {
            lon = upper;
        }
    }
    Ok(lon >= lower && lon <= upper)
}
pub(crate) fn normalized_longitude(lower: f64, upper: f64) -> Result<(f64, f64), GeoError> {
    let tau = std::f64::consts::TAU;
    let tol = 1e-12;
    if lower < -tau - tol
        || lower > tau + tol
        || upper < -tau - tol
        || upper > tau + tol
        || lower == upper
    {
        return Err(GeoError::BadDas("invalid longitude bounds"));
    }
    let mut min = lower.clamp(-tau, tau);
    let mut max = upper.clamp(-tau, tau);
    if max <= min + tol {
        if max <= 0. {
            max = (max + tau).min(tau);
            if max < min {
                min = (min - tau).max(-tau);
            }
        } else {
            min = (min - tau).max(-tau);
        }
    }
    if max - min > tau + tol {
        max -= tau;
    }
    if min == max {
        return Err(GeoError::BadDas("zero longitude extent"));
    }
    Ok((min, max))
}

impl DskDescriptor {
    fn contains_planetodetic(
        &self,
        point: [f64; 3],
        margin: f64,
        excluded: i32,
    ) -> Result<bool, GeoError> {
        if margin == 0. {
            return self.contains_planetodetic_zero(point, excluded);
        }
        let b = self.coordinate_bounds;
        let re = self.coordinate_parameters[0];
        let f = self.coordinate_parameters[1];
        let hpi = std::f64::consts::FRAC_PI_2;
        let minlat = (-hpi - 1e-12).max(b[2] - margin);
        let maxlat = (hpi + 1e-12).min(b[3] + margin);
        if excluded != 2
            && (planetodetic_compare(re, f, point, minlat)? == -1
                || planetodetic_compare(re, f, point, maxlat)? == 1)
        {
            return Ok(false);
        }
        if excluded != 3 {
            let mut altitude_desc = *self;
            altitude_desc.coordinate_bounds = [
                -std::f64::consts::PI,
                std::f64::consts::PI,
                -hpi,
                hpi,
                b[4] - margin * b[4].abs(),
                b[5] + margin * b[5].abs(),
            ];
            if !altitude_desc.contains_planetodetic_zero(point, 1)? {
                return Ok(false);
            }
        }
        let (_, lon, pc_lat) = reclat(point);
        if excluded == 1 {
            return Ok(true);
        }
        if planetodetic_compare(re, f, point, hpi - 1e-8)? != 1
            && planetodetic_compare(re, f, point, -hpi + 1e-8)? != -1
        {
            longitude_inside(
                lon,
                pc_lat,
                self.coordinate_bounds[0],
                self.coordinate_bounds[1],
                margin,
                false,
            )
        } else {
            Ok(true)
        }
    }
    fn contains_planetodetic_zero(&self, point: [f64; 3], excluded: i32) -> Result<bool, GeoError> {
        let b = self.coordinate_bounds;
        let re = self.coordinate_parameters[0];
        let f = self.coordinate_parameters[1];
        let rp = re * (1. - f);
        let hpi = std::f64::consts::FRAC_PI_2;
        if excluded != 2 {
            let min = (b[2] - 1e-12).max(-hpi);
            let max = (b[3] + 1e-12).min(hpi);
            if planetodetic_compare(re, f, point, min)? == -1
                || planetodetic_compare(re, f, point, max)? == 1
            {
                return Ok(false);
            }
        }
        if excluded != 1 && !longitude_inside_zero(reclat(point).1, b[0], b[1])? {
            return Ok(false);
        }
        if excluded != 3 {
            let (min_a, min_b, max_a, max_b) = if f >= 0. {
                bounding_ellipses(re, rp, b[4], b[5])?
            } else {
                let (a, b, c, d) = bounding_ellipses(rp, re, b[4], b[5])?;
                (b, a, d, c)
            };
            let level = |a: f64, b: f64| {
                let x = point[0] / a;
                let y = point[1] / a;
                let z = point[2] / b;
                x * x + y * y + z * z
            };
            if level(max_a, max_b) > 1. || level(min_a, min_b) < 1. {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
pub(crate) fn bounding_ellipses(
    a: f64,
    b: f64,
    min_alt: f64,
    max_alt: f64,
) -> Result<(f64, f64, f64, f64), GeoError> {
    if b <= 0.
        || b > a
        || b + min_alt <= 0.
        || (min_alt < 0. && b + a / b * min_alt <= 0.)
        || min_alt > max_alt
    {
        return Err(GeoError::BadDas("invalid planetodetic altitude bounds"));
    }
    let (outer_a, outer_b) = if max_alt >= 0. {
        (a + max_alt, b + max_alt * (a / b))
    } else {
        (a + max_alt * (b / a), b + max_alt)
    };
    let (inner_a, inner_b) = if min_alt <= 0. {
        (a + min_alt, b + min_alt * (a / b))
    } else {
        (a + min_alt * (b / a), b + min_alt)
    };
    Ok((inner_a, inner_b, outer_a, outer_b))
}
pub(crate) fn planetodetic_compare(
    re: f64,
    f: f64,
    point: [f64; 3],
    latitude: f64,
) -> Result<i32, GeoError> {
    let hpi = std::f64::consts::FRAC_PI_2;
    if point[0] == 0. && point[1] == 0. {
        return Ok(if point[2] > 0. {
            if latitude == hpi {
                0
            } else {
                1
            }
        } else if point[2] == 0. {
            if latitude > 0. {
                -1
            } else if latitude == 0. {
                0
            } else {
                1
            }
        } else if latitude == -hpi {
            0
        } else {
            -1
        });
    }
    let rp = re * (1. - f);
    if re <= 0. || rp <= 0. {
        return Err(GeoError::BadDas("invalid planetodetic shape parameters"));
    }
    // ZZELNAXX calls EDNMPT; retain its scaling and power operation order.
    let scale = re.max(rp);
    let a = re / scale;
    let b = rp / scale;
    let nx = latitude.cos();
    let ny = latitude.sin();
    let na = nx * a * a;
    let nb = ny * b * b;
    let arg = na * nx + nb * ny + 0.;
    let lambda = arg.powf(-0.5) * scale;
    let ratio = re / rp;
    let y_intercept = (1. - ratio * ratio) * (lambda * nb);
    if latitude == 0. || y_intercept == 0. {
        return Ok(if point[2] > 0. {
            1
        } else if point[2] == 0. {
            0
        } else {
            -1
        });
    }
    let (_, _, offset_lat) = reclat([point[0] - 0., point[1] - 0., point[2] - y_intercept]);
    let cmp = if offset_lat > latitude {
        1
    } else if offset_lat == latitude {
        0
    } else {
        -1
    };
    if latitude > 0. {
        if y_intercept > 0. || point[2] > 0. {
            Ok(cmp)
        } else {
            Ok(-1)
        }
    } else if y_intercept < 0. || point[2] < 0. {
        Ok(cmp)
    } else {
        Ok(1)
    }
}
#[cfg(test)]
mod bounds_tests {
    use super::*;
    #[test]
    fn exact_native_dsk_volume_predicates() {
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/dsk/contains.csv")
            .lines()
            .skip(1)
        {
            let p: Vec<f64> = line.split(',').map(|v| v.parse().unwrap()).collect();
            let desc = DskDescriptor {
                surface_id: 0,
                center: 0,
                data_class: 2,
                data_type: 2,
                frame_id: 1,
                coordinate_system: p[0] as i32,
                coordinate_parameters: p[1..11].try_into().unwrap(),
                coordinate_bounds: p[11..17].try_into().unwrap(),
                start: 0.,
                end: 0.,
                raw: [0.; 24],
            };
            let got = desc.contains(p[17..20].try_into().unwrap(), p[20]).unwrap();
            if got != (p[21] != 0.) {
                errors.push(line.to_string());
            }
            count += 1;
        }
        assert_eq!(count, 5600);
        assert!(
            errors.is_empty(),
            "{} mismatches\n{}",
            errors.len(),
            errors
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// DSKs in load order. Geometry uses the unprioritized union of matching
/// surfaces, with the same segment traversal and distance ties as DSKXSI.
#[derive(Clone, Debug, Default)]
pub struct DskSet {
    kernels: Vec<DskKernel>,
}
impl DskSet {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn load(&mut self, path: impl AsRef<std::path::Path>) -> Result<(), GeoError> {
        self.kernels.push(DskKernel::open(path)?);
        Ok(())
    }
    pub fn push(&mut self, kernel: DskKernel) {
        self.kernels.push(kernel);
    }
    pub fn kernels(&self) -> &[DskKernel] {
        &self.kernels
    }
    pub fn ray_intercept_in(
        &self,
        ephemeris: &crate::SpkKernel,
        pck: &crate::Pck,
        frames: &crate::Frames,
        target: i32,
        epoch: f64,
        frame: &str,
        vertex: [f64; 3],
        direction: [f64; 3],
        surfaces: &[i32],
    ) -> Result<Option<DskHit>, GeoError> {
        if direction == [0.; 3] {
            return Err(GeoError::BadSearch("ray direction is zero"));
        }
        if frames.center(frame)? != target {
            return Err(GeoError::BadSearch(
                "DSK ray frame must be centered on the target",
            ));
        }
        let fixid = frames.frame_id(frame)?;
        struct Segment<'a> {
            kernel: usize,
            index: usize,
            data: &'a DskSegment,
            offset: [f64; 3],
            center: [f64; 3],
            radius: f64,
        }
        let mut segments = Vec::new();
        for (ki, kernel) in self.kernels.iter().enumerate().rev() {
            for (si, segment) in kernel.segments.iter().enumerate().rev() {
                let desc = &segment.descriptor;
                if desc.center != target {
                    continue;
                }
                let segment_frame = desc.frame_id.to_string();
                let frame_center = frames.center(&segment_frame)?;
                let offset = if frame_center == target {
                    [0.; 3]
                } else {
                    ephemeris.position_in_frame(
                        frame_center,
                        target,
                        (desc.start + desc.end) / 2.,
                        frames,
                        Some(pck),
                        &segment_frame,
                    )?
                };
                let (center, radius) = crate::dsk_volume::segment_bounding_sphere(desc)?;
                segments.push(Segment {
                    kernel: ki,
                    index: si,
                    data: segment,
                    offset,
                    center,
                    radius,
                });
            }
        }
        if segments.is_empty() {
            return Err(GeoError::BadDas("no DSK segments for target"));
        }
        // ZZDSKSPH bounds ignore coverage time: all matching surfaces contribute.
        let mut max_radius = -f64::MAX;
        for segment in &segments {
            let d = &segment.data.descriptor;
            if !surfaces.is_empty() && !surfaces.contains(&d.surface_id) {
                continue;
            }
            let radius = match d.coordinate_system {
                1 => d.coordinate_bounds[5],
                4 => {
                    let re = d.coordinate_parameters[0];
                    let f = d.coordinate_parameters[1];
                    let rp = re * (1. - f);
                    if f >= 0. {
                        re + d.coordinate_bounds[5]
                    } else {
                        rp + d.coordinate_bounds[5]
                    }
                }
                3 => {
                    let b = d.coordinate_bounds;
                    let dimensions: [f64; 3] = std::array::from_fn(|i| b[2 * i + 1] - b[2 * i]);
                    let center = std::array::from_fn(|i| b[2 * i] + dimensions[i] / 2.);
                    crate::surface::norm(center) + crate::surface::norm(dimensions.map(|v| v / 2.))
                }
                _ => return Err(GeoError::Unsupported("DSK coordinate system".into())),
            };
            max_radius = max_radius.max(radius + crate::surface::norm(segment.offset));
        }
        if max_radius < 0. {
            return Err(GeoError::BadDas("no DSK data match requested surfaces"));
        }
        let limit = max_radius * 1.01;
        let mut local_vertex = vertex;
        let mut local_direction = direction;
        if crate::surface::norm(local_vertex) > limit {
            let Some(hit) = crate::surface::ray_surface_intersection(
                [limit; 3],
                local_vertex,
                local_direction,
            )?
            else {
                return Ok(None);
            };
            local_vertex = hit;
        }
        let mut multiframe = false;
        let mut previous_frame = 0;
        let mut rotation = crate::frame::IDENTITY;
        let mut candidates = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            let d = &segment.data.descriptor;
            if (!surfaces.is_empty() && !surfaces.contains(&d.surface_id))
                || epoch < d.start
                || epoch > d.end
            {
                continue;
            }
            if d.frame_id != fixid {
                multiframe = true;
                if d.frame_id != previous_frame {
                    rotation =
                        frames.pxform_in(frame, &d.frame_id.to_string(), epoch, Some(pck))?;
                    local_direction = rotate(&rotation, direction);
                    local_vertex = sub(rotate(&rotation, vertex), segment.offset);
                }
            } else if multiframe {
                local_vertex = vertex;
                local_direction = direction;
            }
            let nearest = add(
                crate::surface::project(sub(segment.center, local_vertex), local_direction),
                local_vertex,
            );
            let distance = crate::surface::norm(sub(nearest, segment.center));
            if distance <= segment.radius {
                if let Some(hit) =
                    crate::dsk_volume::ray_volume_intercept(local_vertex, local_direction, d, 1e-8)?
                {
                    if candidates.len() >= 1000 {
                        return Err(GeoError::BadDas("ray intersects more than 1000 segments"));
                    }
                    candidates.push((
                        index,
                        crate::surface::norm(sub(local_vertex, hit)),
                        rotation,
                    ));
                }
            }
            previous_frame = d.frame_id;
        }
        let order = source_order(&candidates.iter().map(|c| c.1).collect::<Vec<_>>());
        let mut result: Option<DskHit> = None;
        let mut source_plate = 0;
        let mut distance = f64::MAX;
        for (position, &candidate_index) in order.iter().enumerate() {
            let (index, _, transform) = candidates[candidate_index];
            let segment = &segments[index];
            let d = &segment.data.descriptor;
            if d.frame_id != fixid {
                local_vertex = sub(rotate(&transform, vertex), segment.offset);
                local_direction = rotate(&transform, direction);
            } else if multiframe {
                local_vertex = vertex;
                local_direction = direction;
            }
            if let Some((plate, point)) =
                segment.data.ray_intercept(local_vertex, local_direction)?
            {
                source_plate = plate;
                let new_distance = crate::surface::norm(sub(point, local_vertex));
                if result.is_none() || new_distance < distance {
                    distance = new_distance;
                    let normal = segment.data.plate_normal(plate)?;
                    let (point, normal) = if d.frame_id != fixid {
                        let point = if segment.offset == [0.; 3] {
                            point
                        } else {
                            add(point, segment.offset)
                        };
                        let inverse = crate::frame::transpose(&transform);
                        (rotate(&inverse, point), rotate(&inverse, normal))
                    } else {
                        (point, normal)
                    };
                    result = Some(DskHit {
                        kernel_index: segment.kernel,
                        segment_index: segment.index,
                        plate_id: plate,
                        geometric_plate_id: plate,
                        point,
                        normal,
                    });
                }
            }
            if result.is_some()
                && position + 1 < order.len()
                && distance <= candidates[order[position + 1]].1
            {
                break;
            }
        }
        if let Some(hit) = result.as_mut() {
            hit.plate_id = source_plate;
        }
        Ok(result)
    }
}

fn longitude_inside_zero(mut lon: f64, lower: f64, upper: f64) -> Result<bool, GeoError> {
    let (min, max) = normalized_longitude(lower, upper)?;
    if lon < min - 1e-12 {
        lon += std::f64::consts::TAU;
    } else if lon > max + 1e-12 {
        lon -= std::f64::consts::TAU;
    }
    Ok(lon >= min - 1e-12 && lon <= max + 1e-12)
}
#[cfg(test)]
mod excluding_tests {
    use super::*;
    #[test]
    fn exact_native_dsk_excluded_predicates() {
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/dsk/contains_excluding.csv")
            .lines()
            .skip(1)
        {
            let p: Vec<f64> = line.split(',').map(|v| v.parse().unwrap()).collect();
            let desc = DskDescriptor {
                surface_id: 0,
                center: 0,
                data_class: 2,
                data_type: 2,
                frame_id: 1,
                coordinate_system: p[0] as i32,
                coordinate_parameters: p[1..11].try_into().unwrap(),
                coordinate_bounds: p[11..17].try_into().unwrap(),
                start: 0.,
                end: 0.,
                raw: [0.; 24],
            };
            let got = desc
                .contains_excluding(p[17..20].try_into().unwrap(), p[20], p[21] as i32)
                .unwrap();
            if got != (p[22] != 0.) {
                errors.push(line.to_string());
            }
            count += 1;
        }
        assert_eq!(count, 8400);
        assert!(
            errors.is_empty(),
            "{} mismatches\n{}",
            errors.len(),
            errors
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn rotate(matrix: &crate::Mat3, v: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| matrix[i][0] * v[0] + matrix[i][1] * v[1] + matrix[i][2] * v[2])
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
// ORDERD's Shell sort is observable when equal-distance segments tie.
fn source_order(values: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    let mut gap = values.len() / 2;
    while gap > 0 {
        for i in gap..values.len() {
            let mut j = i - gap;
            loop {
                let next = j + gap;
                if values[order[j]] <= values[order[next]] {
                    break;
                }
                order.swap(j, next);
                if j < gap {
                    break;
                }
                j -= gap;
            }
        }
        gap /= 2;
    }
    order
}

impl DskSet {
    /// Source ZZDSKSPH inner and outer radial bounds, independent of epoch.
    pub(crate) fn radial_bounds(
        &self,
        ephemeris: &crate::SpkKernel,
        pck: &crate::Pck,
        frames: &crate::Frames,
        target: i32,
        surfaces: &[i32],
    ) -> Result<(f64, f64), GeoError> {
        let mut min = f64::MAX;
        let mut max = -f64::MAX;
        for kernel in self.kernels.iter().rev() {
            for segment in kernel.segments.iter().rev() {
                let d = &segment.descriptor;
                if d.center != target || (!surfaces.is_empty() && !surfaces.contains(&d.surface_id))
                {
                    continue;
                }
                let frame = d.frame_id.to_string();
                let center = frames.center(&frame)?;
                let offset = if center == target {
                    0.
                } else {
                    crate::surface::norm(ephemeris.position_in_frame(
                        center,
                        target,
                        (d.start + d.end) / 2.,
                        frames,
                        Some(pck),
                        &frame,
                    )?)
                };
                let b = d.coordinate_bounds;
                let (mut lo, mut hi) = match d.coordinate_system {
                    1 => (b[4], b[5]),
                    4 => {
                        let re = d.coordinate_parameters[0];
                        let f = d.coordinate_parameters[1];
                        let rp = re * (1. - f);
                        if f >= 0. {
                            (rp + b[4], re + b[5])
                        } else {
                            (re + b[4], rp + b[5])
                        }
                    }
                    3 => {
                        let dim: [f64; 3] = std::array::from_fn(|i| b[2 * i + 1] - b[2 * i]);
                        let center = std::array::from_fn(|i| b[2 * i] + dim[i] / 2.);
                        let radius = crate::surface::norm(dim.map(|v| v / 2.));
                        let length = crate::surface::norm(center);
                        ((length - radius).max(0.), length + radius)
                    }
                    _ => return Err(GeoError::Unsupported("DSK coordinate system".into())),
                };
                lo = if offset <= lo {
                    lo - offset
                } else if offset >= hi {
                    offset - hi
                } else {
                    0.
                };
                hi += offset;
                min = min.min(lo);
                max = max.max(hi);
            }
        }
        if max < 0. {
            return Err(GeoError::BadDas("no DSK data match target and surfaces"));
        }
        Ok((min, max))
    }
}

impl DskSet {
    /// Unprioritized surface normal, following ZZSBFNRM/ZZDSKBUN. The
    /// segment normal is normalized before its rotation to `frame`.
    pub fn normal_at_in(
        &self,
        ephemeris: &crate::SpkKernel,
        pck: &crate::Pck,
        frames: &crate::Frames,
        target: i32,
        epoch: f64,
        frame: &str,
        point: [f64; 3],
        surfaces: &[i32],
    ) -> Result<[f64; 3], GeoError> {
        let fixid = frames.frame_id(frame)?;
        let mut multiframe = false;
        let mut previous = 0;
        let mut local = point;
        let mut rotation = crate::frame::IDENTITY;
        let mut candidates = Vec::new();
        for kernel in self.kernels.iter().rev() {
            for segment in kernel.segments.iter().rev() {
                let d = &segment.descriptor;
                if d.center != target {
                    continue;
                }
                let id = d.frame_id.to_string();
                let center = frames.center(&id)?;
                let offset = if center == target {
                    [0.; 3]
                } else {
                    ephemeris.position_in_frame(
                        center,
                        target,
                        (d.start + d.end) / 2.,
                        frames,
                        Some(pck),
                        &id,
                    )?
                };
                let (sphere_center, radius) = crate::dsk_volume::segment_bounding_sphere(d)?;
                if (!surfaces.is_empty() && !surfaces.contains(&d.surface_id))
                    || epoch < d.start
                    || epoch > d.end
                {
                    continue;
                }
                if d.frame_id != fixid {
                    multiframe = true;
                    if d.frame_id != previous {
                        rotation = frames.pxform_in(frame, &id, epoch, Some(pck))?;
                        local = sub(rotate(&rotation, point), offset);
                    }
                } else if multiframe {
                    local = point;
                }
                if crate::surface::norm(sub(sphere_center, local)) <= radius
                    && d.contains_excluding(local, 1e-8, 0)?
                {
                    candidates.push((segment, offset, rotation));
                }
                previous = d.frame_id;
            }
        }
        if candidates.is_empty() {
            return Err(GeoError::BadDas("point is not inside a DSK segment"));
        }
        for (segment, offset, rotation) in candidates {
            let different = segment.descriptor.frame_id != fixid;
            let local = if different {
                sub(rotate(&rotation, point), offset)
            } else {
                point
            };
            if let Some(plate) = crate::dsk_point::plate_at_point(segment, local)? {
                let vertices =
                    segment.plates[plate as usize - 1].map(|id| segment.vertices[id as usize - 1]);
                let normal = crate::surface::unit(cross(
                    sub(vertices[1], vertices[0]),
                    sub(vertices[2], vertices[1]),
                ));
                return Ok(if different {
                    rotate(&crate::frame::transpose(&rotation), normal)
                } else {
                    normal
                });
            }
        }
        Err(GeoError::BadDas("point is not on a DSK surface"))
    }
}
