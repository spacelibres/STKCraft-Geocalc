//! Independent native GFOCLT windows, both depth orders and native errors.
use stkcraft_geocalc::{
    find_occultation_with_models_in, Aberration, DskSet, FinderOptions, Frames, Interval,
    OccultationModel, OccultationType, Pck, SpkKernel, SurfaceModel,
};

#[test]
fn native_dsk_point_occultation_windows_exact() {
    let spk = SpkKernel::open_all(&[
        "target/kernels/de440s.bsp",
        "tests/golden/dsk/occultation-points.bsp",
    ])
    .unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    let mut set = DskSet::new();
    set.load("tests/golden/dsk/optimized.bds").unwrap();
    let mut failures = Vec::new();
    let (mut count, mut windows, mut nonempty, mut errors) = (0, 0, 0, 0);
    for row in include_str!("golden/dsk_occultation.txt").lines() {
        let f: Vec<_> = row.split('|').collect();
        let point = if f[0] == "0" { -999101 } else { -999102 };
        let surfaces: &[i32] = match f[4] {
            "0" => &[],
            "1" => &[100],
            "2" => &[999],
            other => panic!("Unknown surface selection: {other}"),
        };
        let extended = OccultationModel::Surface {
            frame: f[3],
            model: SurfaceModel::DskUnprioritized {
                kernels: &set,
                surfaces,
            },
        };
        let (front, fm, back, bm) = if f[1] == "0" {
            (301, extended, point, OccultationModel::Point)
        } else {
            (point, OccultationModel::Point, 301, extended)
        };
        let result = find_occultation_with_models_in(
            &spk,
            &pck,
            &frames,
            front,
            fm,
            back,
            bm,
            -999100,
            Aberration::parse(f[2]).unwrap(),
            OccultationType::parse(f[5]).unwrap(),
            &[Interval {
                start: -120.0,
                end: 120.0,
            }],
            FinderOptions {
                step: 2.5,
                tolerance: 1e-6,
                adjustment: 0.0,
            },
        );
        let actual = match result {
            Ok(values) => {
                windows += 1;
                nonempty += usize::from(!values.is_empty());
                let mut text = format!("W|{}", values.len());
                for value in values {
                    for number in [value.start, value.end, value.end - value.start] {
                        text.push_str(&format!("|{:016x}", number.to_bits()));
                    }
                }
                text
            }
            Err(error) => {
                errors += 1;
                let text = error.to_string();
                if let Some(start) = text.find("SPICE(") {
                    let end = start + text[start..].find(')').unwrap();
                    format!("E|{}", &text[start..=end])
                } else {
                    format!("UNMAPPED_ERROR|{text}")
                }
            }
        };
        if actual != f[6..].join("|") {
            failures.push(format!("{row}: actual {actual}"));
        }
        count += 1;
    }
    assert_eq!((count, windows, nonempty, errors), (864, 144, 72, 720));
    assert!(
        failures.is_empty(),
        "{} native mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn explicit_occultation_models_preserve_native_ellipsoid_windows() {
    let spk = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::from_text("").unwrap();
    let names = |body| match body {
        301 => "IAU_MOON",
        10 => "IAU_SUN",
        399 => "IAU_EARTH",
        299 => "IAU_VENUS",
        other => panic!("Unrecognized native fixture body: {other}"),
    };
    let mut count = 0;
    for row in include_str!("golden/gf_occultation.csv").lines().skip(2) {
        let f: Vec<_> = row.split(',').collect();
        let front = f[0].parse().unwrap();
        let back = f[1].parse().unwrap();
        let model = |body, shape| {
            if shape == "POINT" {
                OccultationModel::Point
            } else {
                OccultationModel::Surface {
                    frame: names(body),
                    model: SurfaceModel::Ellipsoid,
                }
            }
        };
        let actual = find_occultation_with_models_in(
            &spk,
            &pck,
            &frames,
            front,
            model(front, f[3]),
            back,
            model(back, f[4]),
            f[2].parse().unwrap(),
            Aberration::parse(f[5]).unwrap(),
            OccultationType::parse(f[6]).unwrap(),
            &[Interval {
                start: f[7].parse().unwrap(),
                end: f[8].parse().unwrap(),
            }],
            FinderOptions {
                step: 600.0,
                ..FinderOptions::default()
            },
        )
        .unwrap();
        let expected: Vec<_> = if f[9].is_empty() {
            Vec::new()
        } else {
            f[9].split(';')
                .map(|item| {
                    let (start, end) = item.split_once(':').unwrap();
                    (
                        start.parse::<f64>().unwrap().to_bits(),
                        end.parse::<f64>().unwrap().to_bits(),
                    )
                })
                .collect()
        };
        assert_eq!(
            actual
                .iter()
                .map(|w| (w.start.to_bits(), w.end.to_bits()))
                .collect::<Vec<_>>(),
            expected,
            "{row}"
        );
        count += 1;
    }
    assert_eq!(count, 162);
}
