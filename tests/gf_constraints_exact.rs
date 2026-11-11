//! Native GF RANGE composition and source WN postprocessing, without tolerances.
use stkcraft_geocalc::{
    apply_scalar_constraint, find_distance, find_position_coordinate_with_radii,
    postprocess_windows, window_durations, Aberration, Condition, CoordinateSystem, DurationUnits,
    FinderOptions, Frames, Interval, IntervalAdjustment, IntervalFiltering, Pck, ScalarConstraint,
    ScalarDomain, SpkKernel, WindowDuration, WindowPostprocessing,
};

fn windows(s: &str) -> Vec<Interval> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(';')
        .map(|pair| {
            let (a, b) = pair.split_once(':').unwrap();
            Interval {
                start: a.parse().unwrap(),
                end: b.parse().unwrap(),
            }
        })
        .collect()
}

fn bits(w: &[Interval]) -> Vec<[u64; 2]> {
    w.iter()
        .map(|w| [w.start.to_bits(), w.end.to_bits()])
        .collect()
}

#[test]
fn exact_cspice_native_range_schedules() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::new();
    let mut failures = Vec::new();
    let mut count = 0;
    let mut live_output = String::from("case,windows,durations_seconds\n");
    for line in include_str!("golden/gf_constraints_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let corr = Aberration::parse(f[1]).unwrap();
        let lower = f[4].parse().unwrap();
        let angular = !f[6].is_empty();
        let constraint = if f[3] != "RANGE" {
            ScalarConstraint::Relation {
                condition: Condition::parse(f[3]).unwrap(),
                reference: lower,
            }
        } else if angular {
            ScalarConstraint::AngularRange {
                lower,
                upper: f[5].parse().unwrap(),
                period: f[7].parse().unwrap(),
            }
        } else {
            ScalarConstraint::Range {
                lower,
                upper: f[5].parse().unwrap(),
            }
        };
        let domain = if angular {
            ScalarDomain::Periodic {
                origin: f[6].parse().unwrap(),
                period: f[7].parse().unwrap(),
                upper_inclusive: f[2] == "LATITUDINAL",
            }
        } else {
            ScalarDomain::Linear
        };
        let cf = windows(f[8]);
        let options = FinderOptions {
            step: f[9].parse().unwrap(),
            ..Default::default()
        };
        let actual = apply_scalar_constraint(
            |c, r, cf| {
                if f[0] == "DISTANCE" {
                    find_distance(&kernel, 301, 399, corr, cf, c, r, options)
                } else {
                    let system = CoordinateSystem::parse(f[2]).unwrap();
                    find_position_coordinate_with_radii(
                        &kernel,
                        &pck,
                        301,
                        399,
                        corr,
                        &frames,
                        "IAU_EARTH",
                        system,
                        1,
                        cf,
                        c,
                        r * (std::f64::consts::PI / 180.0),
                        options,
                    )
                }
            },
            &cf,
            constraint,
            domain,
        )
        .unwrap();
        let expected = windows(f[10]);
        if bits(&actual) != bits(&expected) {
            failures.push(format!("{line}\n  actual{actual:?}"));
        }
        if corr == Aberration::None && f[3] == "RANGE" && f[8] == "0.0:4000000.0" {
            let pairs = actual
                .iter()
                .map(|w| format!("{:?}:{:?}", w.start, w.end))
                .collect::<Vec<_>>()
                .join(";");
            let durations = window_durations(&actual, DurationUnits::Seconds)
                .unwrap()
                .iter()
                .map(|x| format!("{x:?}"))
                .collect::<Vec<_>>()
                .join(";");
            live_output.push_str(&format!("distance_{}_{},{pairs},{durations}\n", f[4], f[5]));
        }
        count += 1;
    }
    std::fs::create_dir_all("target/reference").unwrap();
    std::fs::write("target/reference/gf-constraints-local.csv", live_output).unwrap();
    assert!(
        failures.is_empty(),
        "{} mismatches/{count}:\n{}",
        failures.len(),
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{count} exact native GF constraint combinations");
}

#[test]
fn exact_cspice_window_processing_and_duration_units() {
    let mut count = 0;
    for line in include_str!("golden/gf_window_processing_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let adjustment = WindowDuration {
            amount: f[4].parse().unwrap(),
            units: DurationUnits::parse(f[5]).unwrap(),
        };
        let p = WindowPostprocessing {
            complement: f[2] == "1",
            adjustment: match f[3] {
                "EXPAND" => IntervalAdjustment::Expand(adjustment),
                "CONTRACT" => IntervalAdjustment::Contract(adjustment),
                _ => IntervalAdjustment::None,
            },
            filtering: if f[6] == "1" {
                IntervalFiltering::Filter(WindowDuration {
                    amount: f[7].parse().unwrap(),
                    units: DurationUnits::parse(f[8]).unwrap(),
                })
            } else {
                IntervalFiltering::None
            },
            output_duration_units: DurationUnits::parse(f[9]).unwrap(),
        };
        let actual = postprocess_windows(&windows(f[0]), &windows(f[1]), p).unwrap();
        assert_eq!(bits(&actual), bits(&windows(f[10])), "{line}");
        let expected: Vec<u64> = if f[11].is_empty() {
            vec![]
        } else {
            f[11]
                .split(';')
                .map(|x| x.parse::<f64>().unwrap().to_bits())
                .collect()
        };
        assert_eq!(
            window_durations(&actual, p.output_duration_units)
                .unwrap()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            expected,
            "{line}"
        );
        count += 1;
    }
    eprintln!("{count} exact source WN combinations, including duration units");
}

#[test]
fn exact_cspice_native_singleton_confinements() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::new();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/gf_singleton_windows_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let corr = Aberration::parse(f[1]).unwrap();
        let c = Condition::parse(f[3]).unwrap();
        let r = f[4].parse::<f64>().unwrap();
        let cf = windows(f[5]);
        let options = FinderOptions {
            step: 60.,
            ..Default::default()
        };
        let actual = if f[0] == "DISTANCE" {
            find_distance(&kernel, 301, 399, corr, &cf, c, r, options)
        } else {
            find_position_coordinate_with_radii(
                &kernel,
                &pck,
                301,
                399,
                corr,
                &frames,
                "IAU_EARTH",
                CoordinateSystem::parse(f[2]).unwrap(),
                1,
                &cf,
                c,
                r * (std::f64::consts::PI / 180.),
                options,
            )
        }
        .unwrap();
        if bits(&actual) != bits(&windows(f[6])) {
            errors.push(format!("{line}: actual{actual:?}"));
        }
        count += 1;
    }
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{count} exact CSPICE singleton confinement searches");
}
