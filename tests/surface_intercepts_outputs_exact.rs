//! Every SINCPT field, native missing-intercept flag and VNORM/CLIGHT output.
use stkcraft_geocalc::{
    surface_intercept_geometries_with_model_in, surface_intercept_geometry_with_model_in,
    Aberration, DskSet, Frames, GeoError, Pck, RayDirection, SpkKernel, SurfaceModel,
};
fn value(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}
#[test]
fn native_surface_intercept_all_output_cells_exact() {
    let spk = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let mut pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    pck.furnsh("\\begindata\nBODY301_RADII=(3.2 4.1 4.9)\n\\begintext\n")
        .unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    let mut set = DskSet::new();
    set.load("tests/golden/dsk/optimized.bds").unwrap();
    let mut failures = Vec::new();
    let (mut count, mut numeric, mut misses, mut errors) = (0, 0, 0, 0);
    for typed in [false, true] {
        for row in include_str!("golden/surface_intercepts.txt").lines() {
            let fields: Vec<_> = row.split('|').collect();
            let model = match fields[0] {
                "0" => SurfaceModel::Ellipsoid,
                "1" => SurfaceModel::DskUnprioritized {
                    kernels: &set,
                    surfaces: &[],
                },
                "2" => SurfaceModel::DskUnprioritized {
                    kernels: &set,
                    surfaces: &[100],
                },
                "3" => SurfaceModel::DskUnprioritized {
                    kernels: &set,
                    surfaces: &[999],
                },
                other => panic!("Unknown method: {other}"),
            };
            let result = if typed {
                surface_intercept_geometries_with_model_in(
                    &spk,
                    &pck,
                    None,
                    &frames,
                    fields[2],
                    301,
                    if fields[3] == "EARTH" { 399 } else { 10 },
                    value(fields[4]),
                    Aberration::parse(fields[1]).unwrap(),
                    &RayDirection::VectorInFrame {
                        frame: "J2000".into(),
                        vector: [value(fields[5]), value(fields[6]), value(fields[7])],
                        anti: false,
                        stellar_aberration: false,
                    },
                    model,
                )
                .map(|mut values| {
                    assert_eq!(values.len(), 1);
                    values.pop().unwrap()
                })
            } else {
                match surface_intercept_geometry_with_model_in(
                    &spk,
                    &pck,
                    &frames,
                    fields[2],
                    301,
                    if fields[3] == "EARTH" { 399 } else { 10 },
                    value(fields[4]),
                    Aberration::parse(fields[1]).unwrap(),
                    [value(fields[5]), value(fields[6]), value(fields[7])],
                    model,
                ) {
                    Ok(value) => Ok(Some(value)),
                    Err(GeoError::BadSearch("ray does not intersect the target")) => Ok(None),
                    Err(error) => Err(error),
                }
            };
            let actual = match result {
                Ok(Some(g)) => {
                    numeric += 1;
                    let numbers = [
                        g.point[0],
                        g.point[1],
                        g.point[2],
                        g.epoch,
                        g.surface_vector[0],
                        g.surface_vector[1],
                        g.surface_vector[2],
                        g.altitude,
                        g.light_time,
                    ];
                    format!(
                        "N|9|{}",
                        numbers
                            .iter()
                            .map(|v| format!("{:016x}", v.to_bits()))
                            .collect::<Vec<_>>()
                            .join("|")
                    )
                }
                Ok(None) => {
                    misses += 1;
                    "MISS".into()
                }
                Err(error) => {
                    errors += 1;
                    let text = error.to_string();
                    if let Some(start) = text.find("SPICE(") {
                        let end = text[start..].find(')').unwrap() + start;
                        format!("E|{}", &text[start..=end])
                    } else {
                        format!("UNMAPPED_ERROR|{text}")
                    }
                }
            };
            let native = fields[8..].join("|");
            if actual != native {
                failures.push(format!("{row}: actual {actual}"));
            }
            count += 1;
        }
    }
    assert_eq!((count, numeric, misses, errors), (2592, 1700, 244, 648));
    assert!(
        failures.is_empty(),
        "{} native mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
