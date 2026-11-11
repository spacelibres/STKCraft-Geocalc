//! Named direction-frame GFSNTC oracle windows, requiring identical endpoint bits.
use stkcraft_geocalc::finder::find_surface_intercept_ray_coordinate_in;
use stkcraft_geocalc::{
    Aberration, Condition, CoordinateSystem, FinderOptions, Frames, Interval, Pck, SpkKernel,
};

#[test]
fn exact_cspice_named_intercept_direction_frames() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::from_text(include_str!("golden/gf_intercept_dref.tf")).unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/gf_intercept_dref_precision.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let epoch = f[0].parse::<f64>().unwrap();
        let sys = CoordinateSystem::parse(f[6]).unwrap();
        let coordinate = sys.coordinate_index(f[7]).unwrap();
        let actual = find_surface_intercept_ray_coordinate_in(
            &kernel,
            &pck,
            &frames,
            f[11].parse().unwrap(),
            f[12].parse().unwrap(),
            Aberration::parse(f[1]).unwrap(),
            [
                f[3].parse().unwrap(),
                f[4].parse().unwrap(),
                f[5].parse().unwrap(),
            ],
            f[2],
            sys,
            coordinate,
            &[Interval {
                start: epoch,
                end: epoch + 300.,
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
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("{count} exact GFSNTC direction-frame search cases");
}
