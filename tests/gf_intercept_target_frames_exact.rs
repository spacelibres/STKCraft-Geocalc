//! Native named-FIXREF GFSNTC with binary lunar orientation and triaxial radii.
use stkcraft_geocalc::finder::find_surface_intercept_ray_coordinate_with_frame;
use stkcraft_geocalc::{
    Aberration, Condition, CoordinateSystem, FinderOptions, Frames, Interval, Pck, SpkKernel,
};

#[test]
fn exact_cspice_gfsntc_explicit_target_frames() {
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
    let mut errors = Vec::new();
    let mut count = 0;
    let mut expected_errors = 0;
    for line in include_str!("golden/gf_intercept_target_frames_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let pck = if f[0] == "native" { &native } else { &triaxial };
        let et = f[2].parse().unwrap();
        let system = CoordinateSystem::parse(f[8]).unwrap();
        let actual = find_surface_intercept_ray_coordinate_with_frame(
            &kernel,
            pck,
            &frames,
            f[1],
            301,
            399,
            Aberration::parse(f[3]).unwrap(),
            [
                f[5].parse().unwrap(),
                f[6].parse().unwrap(),
                f[7].parse().unwrap(),
            ],
            f[4],
            system,
            system.coordinate_index(f[9]).unwrap(),
            &[Interval {
                start: et,
                end: et + 300.,
            }],
            Condition::parse(f[10]).unwrap(),
            f[11].parse().unwrap(),
            FinderOptions {
                step: 10.,
                ..Default::default()
            },
        );
        if f[12].starts_with("ERROR:") {
            expected_errors += 1;
            if actual.is_ok() {
                errors.push(format!("expected source error {line}:actual{actual:?}"));
            }
        } else {
            let expected: Vec<_> = if f[12].is_empty() {
                vec![]
            } else {
                f[12]
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
            match actual {
                Ok(actual) => {
                    if bits(&actual) != bits(&expected) {
                        errors.push(format!("{line}:actual{actual:?}"));
                    }
                }
                Err(e) => errors.push(format!("{line}:error{e}")),
            }
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
    eprintln!("{count} exact named-target GFSNTC cases ({expected_errors} native rejection cases)");
}

#[test]
fn rejects_unequal_equatorial_radii_before_empty_search() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let mut pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    pck.furnsh("KPL/PCK\n\\begindata\nBODY301_RADII=(1738.1 1736.7 1734.2)\n\\begintext")
        .unwrap();
    for system in [CoordinateSystem::Geodetic, CoordinateSystem::Planetographic] {
        assert!(find_surface_intercept_ray_coordinate_with_frame(
            &kernel,
            &pck,
            &Frames::new(),
            "IAU_MOON",
            301,
            399,
            Aberration::None,
            [1., 0., 0.],
            "J2000",
            system,
            0,
            &[],
            Condition::GreaterThan,
            0.,
            FinderOptions::default()
        )
        .is_err());
    }
}
