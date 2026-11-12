//! Independent native LATSRF points and short-error symbols for both models.
use stkcraft_geocalc::{
    surface_coordinates::latitudinal_surface_points_in, DskSet, Frames, Pck, SpkKernel,
    SurfaceModel,
};

fn number(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

#[test]
fn native_latitudinal_surface_coordinates_exact() {
    let spk = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let mut pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    pck.furnsh("\\begindata\nBODY301_RADII=(3.2 4.1 4.9)\n\\begintext\n")
        .unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    let mut sets = std::collections::HashMap::new();
    let mut failures = Vec::new();
    let (mut count, mut numeric, mut errors) = (0, 0, 0);
    for row in include_str!("golden/surface_coordinates.txt").lines() {
        let parts: Vec<_> = row.split('|').collect();
        let set = sets.entry(parts[0]).or_insert_with(|| {
            let mut set = DskSet::new();
            set.load(format!("tests/golden/dsk/{}", parts[0])).unwrap();
            set
        });
        let model = match parts[1] {
            "0" => SurfaceModel::Ellipsoid,
            "1" => SurfaceModel::DskUnprioritized {
                kernels: set,
                surfaces: &[],
            },
            "2" => SurfaceModel::DskUnprioritized {
                kernels: set,
                surfaces: &[100],
            },
            "3" => SurfaceModel::DskUnprioritized {
                kernels: set,
                surfaces: &[999],
            },
            other => panic!("Unknown native method: {other}"),
        };
        let result = latitudinal_surface_points_in(
            &spk,
            &pck,
            &frames,
            301,
            number(parts[3]),
            parts[2],
            &[[number(parts[4]), number(parts[5])]],
            model,
        );
        let actual = match result {
            Ok(points) => {
                assert_eq!(points.len(), 1);
                numeric += 1;
                format!(
                    "N|3|{}",
                    points[0]
                        .iter()
                        .map(|v| format!("{:016x}", v.to_bits()))
                        .collect::<Vec<_>>()
                        .join("|")
                )
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
        let expected = parts[6..].join("|");
        if actual != expected {
            failures.push(format!("{row}: actual {actual}"));
        }
        count += 1;
    }
    assert_eq!((count, numeric, errors), (2352, 1470, 882));
    assert!(
        failures.is_empty(),
        "{} native mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
