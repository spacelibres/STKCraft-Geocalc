//! All four fixed-ray GF direction specifications against native GFSNTC.
use stkcraft_geocalc::finder::find_surface_intercept_direction_coordinate;
use stkcraft_geocalc::{
    Aberration, Condition, CoordinateSystem, FinderOptions, Fov, FrameAxis, Frames, Interval, Pck,
    RayDirection, SpkKernel,
};

#[test]
fn exact_cspice_gf_intercept_four_direction_specs() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut frames = Frames::from_text(include_str!("golden/gf_intercept_dref.tf")).unwrap();
    frames
        .furnsh(include_str!("golden/gf_intercept_specs.ti"))
        .unwrap();
    let fov = Fov::from_text(include_str!("golden/gf_intercept_specs.ti"), 1481001).unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/gf_intercept_specs_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let vector = [
            f[3].parse().unwrap(),
            f[4].parse().unwrap(),
            f[5].parse().unwrap(),
        ];
        let direction = match f[0] {
            "BORESIGHT" => RayDirection::InstrumentBoresight,
            "AXIS" => RayDirection::FrameAxis {
                frame: f[2].into(),
                axis: FrameAxis::X,
                anti: false,
            },
            "VECTOR_FOV" => RayDirection::VectorInFov { vector },
            "VECTOR_FRAME" => RayDirection::VectorInFrame {
                frame: f[2].into(),
                vector,
                anti: false,
                stellar_aberration: false,
            },
            _ => unreachable!(),
        };
        let system = CoordinateSystem::parse(f[6]).unwrap();
        let actual = find_surface_intercept_direction_coordinate(
            &kernel,
            &pck,
            Some(&fov),
            &frames,
            301,
            399,
            Aberration::parse(f[1]).unwrap(),
            &direction,
            system,
            system.coordinate_index(f[7]).unwrap(),
            &[Interval {
                start: 0.,
                end: 1000.,
            }],
            Condition::parse(f[8]).unwrap(),
            f[9].parse().unwrap(),
            FinderOptions {
                step: 10.,
                ..Default::default()
            },
        )
        .unwrap();
        let expected: Vec<_> = if f[10].is_empty() {
            vec![]
        } else {
            f[10]
                .split(';')
                .map(|p| {
                    let (a, b) = p.split_once(':').unwrap();
                    Interval {
                        start: a.parse().unwrap(),
                        end: b.parse().unwrap(),
                    }
                })
                .collect()
        };
        let bits = |w: &[Interval]| {
            w.iter()
                .map(|w| [w.start.to_bits(), w.end.to_bits()])
                .collect::<Vec<_>>()
        };
        if bits(&actual) != bits(&expected) {
            errors.push(format!("{line}:actual{actual:?}"));
        }
        count += 1;
    }
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{count} exact native GF four-direction specifications");
}
