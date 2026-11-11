use stkcraft_geocalc::{
    frame_transformation, Aberration, FrameTimeLocation, Frames, Pck, SpkKernel,
};
#[test]
fn frame_two_epoch_state_transforms_exact() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::new();
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in include_str!("golden/frame_epochs_exact.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let n = |i: usize| f[i].parse::<f64>().unwrap();
        let result = frame_transformation(
            &kernel,
            &frames,
            Some(&pck),
            f[0],
            f[1],
            n(2),
            Aberration::parse(f[3]).unwrap(),
            FrameTimeLocation::parse(f[4]).unwrap(),
        );
        match result {
            Ok(value) => {
                let mut actual = vec![value.frame1_epoch, value.frame2_epoch, value.light_time];
                actual.extend(value.state_matrix().into_iter().flatten());
                let expected: Vec<_> = (5..f.len()).map(n).collect();
                if actual != expected {
                    failures.push(format!("{:?}: {actual:?} != {expected:?}", &f[..5]));
                }
            }
            Err(error) => failures.push(error.to_string()),
        }
        checked += 1;
    }
    assert_eq!(checked, 180);
    std::fs::write("target/frame-epoch-mismatches.txt", failures.join("\n")).unwrap();
    assert!(
        failures.is_empty(),
        "{} frame two-epoch rows differ:\n{}",
        failures.len(),
        failures
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
