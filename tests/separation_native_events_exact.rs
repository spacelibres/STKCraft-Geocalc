//! Legacy event API must preserve native GFSEP epochs, values and direction flags.
use stkcraft_geocalc::{
    search_separation, search_separation_shaped, Aberration, AngularShape, EventKind, Pck,
    SearchOptions, SpkKernel,
};
#[test]
fn exact_cspice_native_separation_events() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/separation_native_events_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let observer = f[0].parse().unwrap();
        let t1 = f[1].parse().unwrap();
        let t2 = f[2].parse().unwrap();
        let corr = Aberration::parse(f[3]).unwrap();
        let a = AngularShape::parse(f[4]).unwrap();
        let b = AngularShape::parse(f[5]).unwrap();
        let options = SearchOptions {
            step: 3600.,
            tolerance: 1e-6,
            threshold: if f[6].is_empty() {
                None
            } else {
                Some(f[6].parse().unwrap())
            },
        };
        let actual = search_separation_shaped(
            &kernel, &pck, observer, t1, t2, corr, 0., 2592000., options, a, b,
        )
        .unwrap();
        let expected: Vec<_> = if f[7].is_empty() {
            vec![]
        } else {
            f[7].split(';')
                .map(|v| {
                    let x: Vec<_> = v.split(':').collect();
                    (
                        x[0].parse::<f64>().unwrap().to_bits(),
                        match x[1] {
                            "MIN" => EventKind::LocalMinimum,
                            "MAX" => EventKind::LocalMaximum,
                            "UP" => EventKind::CrossingUp,
                            "DOWN" => EventKind::CrossingDown,
                            _ => unreachable!(),
                        },
                        x[2].parse::<f64>().unwrap().to_bits(),
                    )
                })
                .collect()
        };
        let signature = |v: &[stkcraft_geocalc::SeparationEvent]| {
            v.iter()
                .map(|e| (e.et.to_bits(), e.kind, e.separation.to_bits()))
                .collect::<Vec<_>>()
        };
        if signature(&actual) != expected {
            errors.push(format!("{line}:actual{actual:?}"));
        }
        if a == AngularShape::Point && b == AngularShape::Point {
            let raw =
                search_separation(&kernel, observer, t1, t2, corr, 0., 2592000., options).unwrap();
            if signature(&raw) != expected {
                errors.push(format!("point wrapper {line}:actual{raw:?}"));
            }
        }
        count += 1;
    }
    assert!(
        errors.is_empty(),
        "{} differing event sets/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{count} exact native GFSEP event cases");
}

#[test]
fn constant_native_separation_has_no_isolated_events() {
    let kernel = SpkKernel::open("tests/golden/gf_fixed.bsp").unwrap();
    let result = search_separation(
        &kernel,
        0,
        10001,
        10002,
        Aberration::None,
        -1.,
        1.,
        SearchOptions {
            step: 0.1,
            tolerance: 1e-6,
            threshold: Some(0.),
        },
    );
    // Unmodified native GFSEP returns no LOCMIN, LOCMAX or equality
    // intervals for this identically-zero separation, including refval=0.
    assert!(result.unwrap().is_empty());
    assert!(search_separation(
        &kernel,
        0,
        10001,
        10001,
        Aberration::None,
        -1.,
        1.,
        SearchOptions::default()
    )
    .is_err());
}
