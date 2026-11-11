use stkcraft_geocalc::{
    find_occultation, occultation_state, Aberration, FinderOptions, Interval, OccultShape,
    OccultationState, OccultationType, Pck, SpkKernel,
};
fn shape(s: &str) -> OccultShape {
    if s == "POINT" {
        OccultShape::Point
    } else {
        OccultShape::Ellipsoid
    }
}
fn parse(s: &str) -> Vec<Interval> {
    if s.is_empty() {
        return vec![];
    }
    s.split(';')
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
fn exact_occultation_classifications() {
    let k = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let p = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/gf_occultation_states.csv")
        .lines()
        .skip(2)
    {
        let f: Vec<_> = line.split(',').collect();
        let expected = match f[7] {
            "0" => OccultationState::None,
            "1" => OccultationState::Partial,
            "2" => OccultationState::Annular,
            _ => OccultationState::Full,
        };
        let result = occultation_state(
            &k,
            &p,
            f[0].parse().unwrap(),
            shape(f[3]),
            f[1].parse().unwrap(),
            shape(f[4]),
            f[2].parse().unwrap(),
            f[6].parse().unwrap(),
            Aberration::parse(f[5]).unwrap(),
        )
        .unwrap();
        if result != expected {
            errors.push(format!("{line}: actual{result:?}"));
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
}
#[test]
fn exact_occultation_windows() {
    let k = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let p = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/gf_occultation.csv").lines().skip(2) {
        let f: Vec<_> = line.split(',').collect();
        let expected = parse(f[9]);
        let result = find_occultation(
            &k,
            &p,
            f[0].parse().unwrap(),
            shape(f[3]),
            f[1].parse().unwrap(),
            shape(f[4]),
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
        if result != expected {
            errors.push(format!("{line}: actual{result:?}"));
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
}
