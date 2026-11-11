use stkcraft_geocalc::{
    find_position_coordinate, Aberration, Condition, CoordinateSystem, FinderOptions, Frames,
    Interval, SpkKernel,
};
#[test]
fn exact_cspice_longitude_branch_cuts() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let frames = Frames::new();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/gf_longitudes.csv").lines().skip(2) {
        let f: Vec<_> = line.split(',').collect();
        let sys = CoordinateSystem::parse(f[0]).unwrap();
        let expected: Vec<_> = if f[5].is_empty() {
            vec![]
        } else {
            f[5].split(';')
                .map(|s| {
                    let (a, b) = s.split_once(':').unwrap();
                    Interval {
                        start: a.parse().unwrap(),
                        end: b.parse().unwrap(),
                    }
                })
                .collect()
        };
        let actual = find_position_coordinate(
            &kernel,
            301,
            399,
            Aberration::parse(f[1]).unwrap(),
            &frames,
            "J2000",
            sys,
            1,
            &[Interval {
                start: 0.0,
                end: 2592000.0,
            }],
            Condition::parse(f[2]).unwrap(),
            f[3].parse().unwrap(),
            FinderOptions {
                step: 3600.0,
                tolerance: 1e-6,
                adjustment: f[4].parse().unwrap(),
            },
        )
        .unwrap();
        if actual != expected {
            errors.push(format!("{line}: actual{actual:?}"));
        }
        count += 1;
    }
    assert!(
        errors.is_empty(),
        "{} mismatches/{count} cases:\n{}",
        errors.len(),
        errors
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
