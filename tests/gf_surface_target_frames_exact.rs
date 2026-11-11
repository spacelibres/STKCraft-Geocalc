//! Native GFSUBC/GFILUM windows in binary-PCK and TK target frames.
use stkcraft_geocalc::finder::{find_illumination_angle_in, find_sub_point_coordinate_in};
use stkcraft_geocalc::{
    Aberration, Condition, CoordinateSystem, FinderOptions, Frames, IlluminationAngle, Interval,
    Pck, SpkKernel, SubPointKind, SubPointMethod,
};

fn context() -> (SpkKernel, Pck, Pck, Frames) {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let native = Pck::open_all(&[
        "target/kernels/pck00011.tpc",
        "target/kernels/moon_pa_de440_200625.bpc",
    ])
    .unwrap();
    let mut triaxial = native.clone();
    triaxial
        .furnsh("KPL/PCK\n\\begindata\nBODY301_RADII=(1738.1 1736.7 1734.2)\n\\begintext")
        .unwrap();
    let mut frames = Frames::new();
    for path in [
        "target/kernels/moon_de440_250416.tf",
        "target/kernels/moon_assoc_me.tf",
        "tests/golden/surface_target_frames.tf",
    ] {
        frames
            .furnsh(&std::fs::read_to_string(path).unwrap())
            .unwrap();
    }
    (kernel, native, triaxial, frames)
}
fn check(actual: Result<Vec<Interval>, stkcraft_geocalc::GeoError>, encoded: &str) -> bool {
    if encoded.starts_with("ERROR:") {
        return actual.is_err();
    }
    let expected: Vec<_> = if encoded.is_empty() {
        vec![]
    } else {
        encoded
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
    actual
        .map(|actual| bits(&actual) == bits(&expected))
        .unwrap_or(false)
}
#[test]
fn exact_cspice_gfsubc_named_target_frames() {
    let (kernel, native, triaxial, frames) = context();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/gf_subpoint_target_frames_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let pck = if f[0] == "native" { &native } else { &triaxial };
        let sys = CoordinateSystem::parse(f[4]).unwrap();
        let actual = find_sub_point_coordinate_in(
            &kernel,
            pck,
            &frames,
            f[1],
            301,
            399,
            Aberration::parse(f[2]).unwrap(),
            SubPointKind::Observer,
            SubPointMethod::parse(f[3]).unwrap(),
            sys,
            sys.coordinate_index(f[5]).unwrap(),
            &[Interval {
                start: 0.,
                end: 1728000.,
            }],
            Condition::parse(f[6]).unwrap(),
            f[7].parse().unwrap(),
            FinderOptions {
                step: 7200.,
                ..Default::default()
            },
        );
        let detail = format!("{actual:?}");
        if !check(actual, f[8]) {
            errors.push(format!("{line}:actual{detail}"));
        }
        count += 1;
    }
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{count} exact named-target GFSUBC source cases");
}
#[test]
fn exact_cspice_gfilum_named_target_frames() {
    let (kernel, native, triaxial, frames) = context();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/gf_illumination_target_frames_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let pck = if f[0] == "native" { &native } else { &triaxial };
        let angle = match f[4] {
            "PHASE" => IlluminationAngle::Phase,
            "INCIDENCE" => IlluminationAngle::Incidence,
            "EMISSION" => IlluminationAngle::Emission,
            _ => unreachable!(),
        };
        let actual = find_illumination_angle_in(
            &kernel,
            pck,
            &frames,
            f[1],
            301,
            f[3].parse().unwrap(),
            399,
            Aberration::parse(f[2]).unwrap(),
            [
                f[5].parse().unwrap(),
                f[6].parse().unwrap(),
                f[7].parse().unwrap(),
            ],
            angle,
            &[Interval {
                start: 0.,
                end: 1728000.,
            }],
            Condition::parse(f[8]).unwrap(),
            f[9].parse().unwrap(),
            FinderOptions {
                step: 7200.,
                ..Default::default()
            },
        );
        let detail = format!("{actual:?}");
        if !check(actual, f[10]) {
            errors.push(format!("{line}:actual{detail}"));
        }
        count += 1;
    }
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{count} exact named-target GFILUM source cases");
}
