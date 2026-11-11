//! GFUDS with the native UDDC/UDDF derivative callback, not a replacement solver.
use stkcraft_geocalc::finder::search_scalar_with_uddc;
use stkcraft_geocalc::{search_scalar, Condition, FinderOptions, Interval};
fn value(name: &str, t: f64) -> f64 {
    match name {
        "linear" => t,
        "quadratic" => t * t,
        "negative_quadratic" => -t * t,
        "cubic" => t * (t - 2.) * (t + 2.),
        "quartic" => (t - 1.) * (t - 1.) * (t + 2.) * (t + 2.),
        "constant" => 2.,
        "small_linear" => 1e-12 * t,
        "small_quadratic" => 1e-12 * (t - 0.37) * (t - 0.37),
        _ => unreachable!(),
    }
}
fn windows(text: &str) -> Vec<Interval> {
    if text.is_empty() {
        return vec![];
    }
    text.split(';')
        .map(|p| {
            let (a, b) = p.split_once(':').unwrap();
            Interval {
                start: a.parse().unwrap(),
                end: b.parse().unwrap(),
            }
        })
        .collect()
}
#[test]
fn exact_cspice_gfuds_with_native_uddc() {
    let mut errors = Vec::new();
    let mut count = 0;
    let bits = |w: &[Interval]| {
        w.iter()
            .map(|w| [w.start.to_bits(), w.end.to_bits()])
            .collect::<Vec<_>>()
    };
    for line in include_str!("golden/gf_uddc_precision.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let dt = f[1].parse().unwrap();
        let condition = Condition::parse(f[2]).unwrap();
        let reference = f[3].parse().unwrap();
        let options = FinderOptions {
            step: 0.7,
            tolerance: 1e-6,
            adjustment: f[4].parse().unwrap(),
        };
        let cf = windows(f[5]);
        let actual = search_scalar_with_uddc(
            |t| Ok(value(f[0], t)),
            dt,
            &cf,
            condition,
            reference,
            options,
        )
        .unwrap();
        let expected = windows(f[6]);
        if bits(&actual) != bits(&expected) {
            errors.push(format!("{line}:actual{actual:?}"));
        }
        if dt == 1. {
            let default =
                search_scalar(|t| Ok(value(f[0], t)), &cf, condition, reference, options).unwrap();
            if bits(&default) != bits(&expected) {
                errors.push(format!("default {line}:actual{default:?}"));
            }
        }
        count += 1;
    }
    assert!(
        errors.is_empty(),
        "{} differing results/{count} cases:\n{}",
        errors.len(),
        errors
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{count} exact native GFUDS/UDDC cases");
}

#[test]
fn periodic_value_only_constraints_require_a_native_domain() {
    let actual = stkcraft_geocalc::search_scalar_constraint(
        Ok,
        &[Interval {
            start: -4.,
            end: 4.,
        }],
        stkcraft_geocalc::ScalarConstraint::AngularRange {
            lower: 3.,
            upper: -3.,
            period: 8.,
        },
        FinderOptions::default(),
    );
    assert!(matches!(
        actual,
        Err(stkcraft_geocalc::GeoError::Unsupported(_))
    ));
}
