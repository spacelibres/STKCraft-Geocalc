use std::collections::{BTreeMap, HashMap};
use stkcraft_geocalc::{
    find_ray_in_fov_in, find_target_in_fov_shaped, ray_in_fov_in, target_in_fov_with_pck,
    Aberration, FinderOptions, Fov, FovTargetShape, Frames, Interval, Pck, RayAberration,
    SpkKernel,
};
const IK: &str = include_str!("golden/fov_precision.ti");
fn setup() -> (SpkKernel, Pck, Frames, HashMap<i32, Fov>) {
    (
        SpkKernel::open("target/kernels/de440s.bsp").unwrap(),
        Pck::open("target/kernels/pck00011.tpc").unwrap(),
        Frames::new(),
        (919001..919005)
            .map(|id| (id, Fov::from_text(IK, id).unwrap()))
            .collect(),
    )
}
#[test]
fn fov_predicates_all_corrections_and_frames_exact() {
    let (kernel, pck, frames, fovs) = setup();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/fov_precision.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let inst = f[1].parse().unwrap();
        let fov = &fovs[&inst];
        let et = f[7].parse().unwrap();
        let actual = if f[0] == "target" {
            target_in_fov_with_pck(
                &kernel,
                Some(&pck),
                fov,
                301,
                399,
                et,
                Aberration::parse(f[2]).unwrap(),
                &frames,
            )
        } else {
            ray_in_fov_in(
                &kernel,
                Some(&pck),
                fov,
                399,
                et,
                RayAberration::parse(f[2]).unwrap(),
                [
                    f[4].parse().unwrap(),
                    f[5].parse().unwrap(),
                    f[6].parse().unwrap(),
                ],
                f[3],
                &frames,
            )
        }
        .unwrap();
        if actual != (f[8] == "1") {
            failures.push(line.to_string());
        }
        count += 1;
    }
    assert_eq!(count, 1692);
    assert!(
        failures.is_empty(),
        "{} predicate mismatches: {:?}",
        failures.len(),
        failures.iter().take(8).collect::<Vec<_>>()
    );
}
#[test]
fn fov_gf_windows_exact() {
    let (kernel, pck, frames, fovs) = setup();
    let mut groups = BTreeMap::<Vec<String>, Vec<Interval>>::new();
    for line in include_str!("golden/fov_windows_exact.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let key = f[..8].iter().map(|x| x.to_string()).collect();
        let windows = groups.entry(key).or_default();
        if f[8] != "-1" {
            windows.push(Interval {
                start: f[9].parse().unwrap(),
                end: f[10].parse().unwrap(),
            });
        }
    }
    assert_eq!(groups.len(), 144);
    let mut failures = Vec::new();
    for (f, expected) in groups {
        let fov = &fovs[&f[1].parse::<i32>().unwrap()];
        let options = FinderOptions {
            step: f[4].parse().unwrap(),
            tolerance: 1e-6,
            adjustment: 0.,
        };
        let windows = [Interval {
            start: 0.,
            end: 259200.,
        }];
        let actual = if f[0] == "target" {
            find_target_in_fov_shaped(
                &kernel,
                &pck,
                fov,
                301,
                399,
                Aberration::parse(&f[2]).unwrap(),
                &frames,
                &windows,
                options,
                FovTargetShape::Point,
            )
        } else {
            find_ray_in_fov_in(
                &kernel,
                Some(&pck),
                fov,
                399,
                RayAberration::parse(&f[2]).unwrap(),
                [
                    f[5].parse().unwrap(),
                    f[6].parse().unwrap(),
                    f[7].parse().unwrap(),
                ],
                &f[3],
                &frames,
                &windows,
                options,
            )
        }
        .unwrap();
        if actual != expected {
            failures.push(format!("{f:?}: {actual:?} != {expected:?}"));
        }
    }
    std::fs::write("target/fov-window-mismatches.txt", failures.join("\n")).unwrap();
    assert!(
        failures.is_empty(),
        "{} FOV windows differ:\n{}",
        failures.len(),
        failures
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
