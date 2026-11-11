//! Independent native CSPICE DAS/DLA/DSK metadata, no tolerance.
use stkcraft_geocalc::DskKernel;
#[test]
fn exact_native_dsk_structure() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/dsk");
    let mut count = 0;
    for line in include_str!("golden/dsk/structure.csv").lines().skip(1) {
        let p: Vec<_> = line.split(',').collect();
        let k = DskKernel::open(root.join(p[0])).unwrap();
        let s = &k.segments()[p[1].parse::<usize>().unwrap()];
        let expected: Vec<f64> = p[3].split(';').map(|x| x.parse().unwrap()).collect();
        let actual: Vec<f64> = match p[2] {
            "dla" => s.dla.as_raw().map(|x| x as f64).to_vec(),
            "descriptor" => s.descriptor.raw.to_vec(),
            "bounds" => s.vertex_bounds.to_vec(),
            "origin" => s.voxel_origin.to_vec(),
            "voxel_size" => vec![s.voxel_size],
            "parameters" => s.integer_parameters.map(|x| x as f64).to_vec(),
            name if name.starts_with("vertex") => {
                s.vertices[name[6..].parse::<usize>().unwrap() - 1].to_vec()
            }
            name if name.starts_with("plate") => s.plates[name[5..].parse::<usize>().unwrap() - 1]
                .map(|x| x as f64)
                .to_vec(),
            name if name.starts_with("normal") => {
                s.plate_normal(name[6..].parse().unwrap()).unwrap().to_vec()
            }
            _ => panic!("{}", p[2]),
        };
        assert_eq!(
            actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "{}",
            line
        );
        count += 1;
    }
    assert_eq!(count, 112);
}

#[test]
fn exact_native_dsk_rays() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/dsk");
    let mut kernels = std::collections::HashMap::new();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/dsk/rays.csv")
        .lines()
        .skip(1)
        .chain(include_str!("golden/dsk/dense_rays.csv").lines().skip(1))
    {
        let p: Vec<_> = line.split(',').collect();
        let k = kernels
            .entry(p[0])
            .or_insert_with(|| DskKernel::open(root.join(p[0])).unwrap());
        let s = &k.segments()[p[1].parse::<usize>().unwrap()];
        let v: Vec<f64> = p[2..8].iter().map(|x| x.parse().unwrap()).collect();
        let got = s
            .ray_intercept(v[..3].try_into().unwrap(), v[3..].try_into().unwrap())
            .unwrap();
        let actual = match got {
            Some((plate, point)) => [
                vec![plate as f64],
                point.to_vec(),
                s.plate_normal(plate).unwrap().to_vec(),
            ]
            .concat(),
            None => Vec::new(),
        };
        let expected: Vec<f64> = if p[8].is_empty() {
            vec![]
        } else {
            p[8].split(';').map(|x| x.parse().unwrap()).collect()
        };
        if actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            != expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        {
            errors.push(format!("{}: {:?} != {:?}", line, actual, expected));
        }
        count += 1;
    }
    assert_eq!(count, 12290);
    assert!(
        errors.is_empty(),
        "{} mismatches\n{}",
        errors.len(),
        errors
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn exact_native_das_layouts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/dsk");
    let mut kernels = std::collections::HashMap::new();
    let mut count = 0;
    for line in include_str!("golden/dsk/layouts.csv").lines().skip(1) {
        let p: Vec<_> = line.split(',').collect();
        let k = kernels
            .entry(p[0])
            .or_insert_with(|| DskKernel::open(root.join(p[0])).unwrap());
        let s = &k.segments()[p[1].parse::<usize>().unwrap()];
        let dla: Vec<i32> = p[2].split(';').map(|v| v.parse().unwrap()).collect();
        assert_eq!(s.dla.as_raw().to_vec(), dla, "{line}");
        assert_eq!(s.descriptor.surface_id, p[3].parse::<i32>().unwrap());
        let vertices: Vec<u64> = p[4]
            .split(';')
            .map(|v| v.parse::<f64>().unwrap().to_bits())
            .collect();
        assert_eq!(
            s.vertices
                .iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            vertices
        );
        let plates: Vec<i32> = p[5].split(';').map(|v| v.parse().unwrap()).collect();
        assert_eq!(
            s.plates.iter().flatten().copied().collect::<Vec<_>>(),
            plates
        );
        count += 1;
    }
    assert_eq!(count, 2004);
}
#[test]
fn rejects_corrupt_das_without_panics() {
    let valid = include_bytes!("golden/dsk/optimized.bds");
    for end in [0, 8, 68, 84, 92, 1023, 1024, 5120, valid.len() - 1] {
        assert!(DskKernel::from_bytes(valid[..end].to_vec()).is_err());
    }
    for (offset, word) in [
        (68, -1i32),
        (76, -1),
        (5 * 1024 + 4, 6),
        (5 * 1024 + 8 * 4, 99),
        (5 * 1024 + 9 * 4, i32::MIN),
    ] {
        let mut bytes = valid.to_vec();
        bytes[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
        assert!(DskKernel::from_bytes(bytes).is_err());
    }
}

#[test]
fn exact_native_unprioritized_dsk_union() {
    use stkcraft_geocalc::dsk::DskSet;
    use stkcraft_geocalc::{Frames, Pck, SpkKernel};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let data = root.join("tests/golden/dsk");
    let spk = SpkKernel::open(root.join("target/kernels/de440s.bsp")).unwrap();
    let pck = Pck::open(root.join("target/kernels/pck00011.tpc")).unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    let mut sets = std::collections::HashMap::new();
    let mut count = 0;
    let mut errors = Vec::new();
    for line in include_str!("golden/dsk/union.csv").lines().skip(1) {
        let p: Vec<_> = line.split(',').collect();
        let set = sets.entry(p[0]).or_insert_with(|| {
            let mut set = DskSet::new();
            for file in p[0].split(';') {
                set.load(data.join(file)).unwrap();
            }
            set
        });
        let vec: Vec<f64> = p[4..10].iter().map(|x| x.parse().unwrap()).collect();
        let surfaces: Vec<i32> = if p[3].is_empty() {
            vec![]
        } else {
            p[3].split(';').map(|x| x.parse().unwrap()).collect()
        };
        let result = set.ray_intercept_in(
            &spk,
            &pck,
            &frames,
            301,
            p[2].parse().unwrap(),
            p[1],
            vec[..3].try_into().unwrap(),
            vec[3..].try_into().unwrap(),
            &surfaces,
        );
        if p[10] == "ERROR" {
            assert!(result.is_err());
            count += 1;
            continue;
        }
        let actual = match result {
            Ok(Some(hit)) => vec![
                hit.kernel_index as f64,
                set.kernels()[hit.kernel_index].segments()[hit.segment_index]
                    .dla
                    .integer_base as f64,
                hit.plate_id as f64,
                hit.point[0],
                hit.point[1],
                hit.point[2],
            ],
            Ok(None) => vec![],
            Err(e) => {
                errors.push(format!("{line}: {e}"));
                count += 1;
                continue;
            }
        };
        let expected: Vec<f64> = if p[10].is_empty() {
            vec![]
        } else {
            p[10].split(';').map(|x| x.parse().unwrap()).collect()
        };
        if actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            != expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        {
            errors.push(format!("{line}: {actual:?} != {expected:?}"));
        }
        count += 1;
    }
    assert_eq!(count, 8640);
    assert!(
        errors.is_empty(),
        "{} mismatches\n{}",
        errors.len(),
        errors
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn exact_native_dsk_surface_geometry() {
    use stkcraft_geocalc::dsk::DskSet;
    use stkcraft_geocalc::surface::{
        sub_observer_dsk_geometry_in, surface_intercept_geometry_with_model_in, DskSubPointMethod,
        SurfaceModel,
    };
    use stkcraft_geocalc::{Aberration, Frames, Pck, SpkKernel};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let data = root.join("tests/golden/dsk");
    let spk = SpkKernel::open(root.join("target/kernels/de440s.bsp")).unwrap();
    let mut pck = Pck::open(root.join("target/kernels/pck00011.tpc")).unwrap();
    pck.furnsh("KPL/PCK\n\\begindata\nBODY301_RADII=(3.2 4.1 4.9)\n\\begintext")
        .unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    let mut sets = std::collections::HashMap::new();
    let mut count = 0;
    let mut errors = Vec::new();
    for line in include_str!("golden/dsk/surface.csv").lines().skip(1) {
        let p: Vec<_> = line.split(',').collect();
        let set = sets.entry(p[0]).or_insert_with(|| {
            let mut set = DskSet::new();
            set.load(data.join(p[0])).unwrap();
            set
        });
        let et = p[3].parse().unwrap();
        let observer = if p[4] == "EARTH" { 399 } else { 10 };
        let corr = Aberration::parse(p[5]).unwrap();
        let surfaces: Vec<i32> = if p[6].is_empty() {
            vec![]
        } else {
            p[6].split(';').map(|x| x.parse().unwrap()).collect()
        };
        let direction: [f64; 3] = p[7..10]
            .iter()
            .map(|x| x.parse().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let result = if p[1] == "intercept" {
            surface_intercept_geometry_with_model_in(
                &spk,
                &pck,
                &frames,
                p[2],
                301,
                observer,
                et,
                corr,
                direction,
                SurfaceModel::DskUnprioritized {
                    kernels: set,
                    surfaces: &surfaces,
                },
            )
        } else {
            sub_observer_dsk_geometry_in(
                &spk,
                &pck,
                &frames,
                p[2],
                301,
                observer,
                et,
                corr,
                set,
                &surfaces,
                if p[1] == "NADIR" {
                    DskSubPointMethod::Nadir
                } else {
                    DskSubPointMethod::Intercept
                },
            )
        };
        if p[10] == "ERROR" {
            assert!(result.is_err());
            count += 1;
            continue;
        }
        let actual = match result {
            Ok(g) => vec![
                g.point[0],
                g.point[1],
                g.point[2],
                g.epoch,
                g.surface_vector[0],
                g.surface_vector[1],
                g.surface_vector[2],
                g.altitude,
            ],
            Err(_) if p[10].is_empty() => vec![],
            Err(e) => {
                errors.push(format!("{line}: {e}"));
                count += 1;
                continue;
            }
        };
        let expected: Vec<f64> = if p[10].is_empty() {
            vec![]
        } else {
            p[10].split(';').map(|x| x.parse().unwrap()).collect()
        };
        if actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            != expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        {
            errors.push(format!("{line}: {actual:?} != {expected:?}"));
        }
        count += 1;
    }
    assert_eq!(count, 2160);
    assert!(
        errors.is_empty(),
        "{} mismatches\n{}",
        errors.len(),
        errors
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn exact_native_dynamic_surface_models() {
    use stkcraft_geocalc::dsk::DskSet;
    use stkcraft_geocalc::surface::*;
    use stkcraft_geocalc::{Aberration, Frames, Pck, SpkKernel, SubPointMethod};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let data = root.join("tests/golden/dsk");
    let spk = SpkKernel::open_all(&[
        root.join("target/kernels/de440s.bsp"),
        root.join("tests/golden/spk_dynamic_sources.bsp"),
    ])
    .unwrap();
    let native = Pck::open(root.join("target/kernels/pck00011.tpc")).unwrap();
    let mut dskpck = native.clone();
    dskpck
        .furnsh("KPL/PCK\n\\begindata\nBODY301_RADII=(3.2 4.1 4.9)\n\\begintext")
        .unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/dynamic_frames.tf")).unwrap();
    let mut set = DskSet::new();
    set.load(data.join("dense_lat.bds")).unwrap();
    let mut count = 0;
    let mut errors = Vec::new();
    for line in include_str!("golden/dsk/dynamic_surface.csv")
        .lines()
        .skip(1)
    {
        let p: Vec<_> = line.split(',').collect();
        let dsk = p[0] == "dsk";
        let pck = if dsk { &dskpck } else { &native };
        let et = p[3].parse().unwrap();
        let observer = p[4].parse().unwrap();
        let corr = Aberration::parse(p[5]).unwrap();
        let direction: [f64; 3] = p[6..9]
            .iter()
            .map(|x| x.parse().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let method = if p[1].ends_with("Near") {
            SubPointMethod::NearPoint
        } else {
            SubPointMethod::Intercept
        };
        let dskmethod = if p[1].ends_with("Near") {
            DskSubPointMethod::Nadir
        } else {
            DskSubPointMethod::Intercept
        };
        let result = if p[1] == "intercept" {
            surface_intercept_geometry_with_model_in(
                &spk,
                pck,
                &frames,
                p[2],
                301,
                observer,
                et,
                corr,
                direction,
                if dsk {
                    SurfaceModel::DskUnprioritized {
                        kernels: &set,
                        surfaces: &[],
                    }
                } else {
                    SurfaceModel::Ellipsoid
                },
            )
        } else if p[1].starts_with("solar") {
            if dsk {
                sub_solar_dsk_geometry_in(
                    &spk,
                    pck,
                    &frames,
                    p[2],
                    301,
                    observer,
                    et,
                    corr,
                    &set,
                    &[],
                    dskmethod,
                )
            } else {
                sub_solar_geometry_in(&spk, pck, &frames, p[2], 301, observer, et, corr, method)
            }
        } else if dsk {
            sub_observer_dsk_geometry_in(
                &spk,
                pck,
                &frames,
                p[2],
                301,
                observer,
                et,
                corr,
                &set,
                &[],
                dskmethod,
            )
        } else {
            sub_observer_geometry_in(&spk, pck, &frames, p[2], 301, observer, et, corr, method)
        };
        if p[9].starts_with("ERROR:") {
            assert!(result.is_err());
            count += 1;
            continue;
        }
        let actual = match result {
            Ok(g) => vec![
                g.point[0],
                g.point[1],
                g.point[2],
                g.epoch,
                g.surface_vector[0],
                g.surface_vector[1],
                g.surface_vector[2],
                g.altitude,
            ],
            Err(_) if p[9].is_empty() => vec![],
            Err(e) => {
                errors.push(format!("{line}: {e}"));
                count += 1;
                continue;
            }
        };
        let expected: Vec<f64> = if p[9].is_empty() {
            vec![]
        } else {
            p[9].split(';').map(|x| x.parse().unwrap()).collect()
        };
        if actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            != expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        {
            errors.push(format!("{line}: {actual:?} != {expected:?}"));
        }
        count += 1;
    }
    assert_eq!(count, 2160);
    assert!(
        errors.is_empty(),
        "{} mismatches\n{}",
        errors.len(),
        errors
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn exact_native_dsk_illumination() {
    use stkcraft_geocalc::dsk::DskSet;
    use stkcraft_geocalc::surface::{illumination_geometry_with_model_in, SurfaceModel};
    use stkcraft_geocalc::{Aberration, Frames, Pck, SpkKernel};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let data = root.join("tests/golden/dsk");
    let spk = SpkKernel::open_all(&[
        root.join("target/kernels/de440s.bsp"),
        root.join("tests/golden/spk_dynamic_sources.bsp"),
    ])
    .unwrap();
    let pck = Pck::open(root.join("target/kernels/pck00011.tpc")).unwrap();
    let mut frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    frames
        .furnsh(include_str!("golden/dsk/dynamic_frames.tf"))
        .unwrap();
    let mut sets = std::collections::HashMap::new();
    let mut count = 0;
    let mut errors = Vec::new();
    for line in include_str!("golden/dsk/illumination.csv").lines().skip(1) {
        let p: Vec<_> = line.split(',').collect();
        let set = sets.entry(p[0]).or_insert_with(|| {
            let mut set = DskSet::new();
            for file in p[0].split(';') {
                set.load(data.join(file)).unwrap();
            }
            set
        });
        let et = p[2].parse().unwrap();
        let observer = p[3].parse().unwrap();
        let illuminator = p[4].parse().unwrap();
        let corr = Aberration::parse(p[5]).unwrap();
        let surfaces: Vec<i32> = if p[6].is_empty() {
            vec![]
        } else {
            p[6].split(';').map(|x| x.parse().unwrap()).collect()
        };
        let point: [f64; 3] = p[7..10]
            .iter()
            .map(|x| x.parse().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let result = illumination_geometry_with_model_in(
            &spk,
            &pck,
            &frames,
            p[1],
            301,
            illuminator,
            observer,
            et,
            corr,
            point,
            SurfaceModel::DskUnprioritized {
                kernels: set,
                surfaces: &surfaces,
            },
        );
        if p[10].starts_with("ERROR:") {
            if result.is_ok() {
                errors.push(format!("expectederror: {line}"));
            }
            count += 1;
            continue;
        }
        let actual = match result {
            Ok(g) => vec![
                g.angles.phase,
                g.angles.solar,
                g.angles.emission,
                g.epoch,
                g.surface_vector[0],
                g.surface_vector[1],
                g.surface_vector[2],
                g.visible as i32 as f64,
                g.illuminated as i32 as f64,
            ],
            Err(e) => {
                errors.push(format!("{line}: {e}"));
                count += 1;
                continue;
            }
        };
        let expected: Vec<f64> = p[10].split(';').map(|x| x.parse().unwrap()).collect();
        if actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            != expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        {
            errors.push(format!("{line}: {actual:?} != {expected:?}"));
        }
        count += 1;
    }
    assert_eq!(count, 10368);
    assert!(
        errors.is_empty(),
        "{} mismatches\n{}",
        errors.len(),
        errors
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
