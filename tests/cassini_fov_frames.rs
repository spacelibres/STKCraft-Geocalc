use stkcraft_geocalc::{ck::CkKernel, Frames};
use stkcraft_time::{LeapSecondTable, SclkKernel};
#[test]
fn real_cassini_sclk_and_pointing_are_exact() {
    let clock = SclkKernel::parse(
        &std::fs::read_to_string("target/kernels/cassini/cas00172.tsc").unwrap(),
        -82,
    )
    .unwrap();
    let lsk =
        LeapSecondTable::from_lsk(&std::fs::read_to_string("target/kernels/naif0012.tls").unwrap())
            .unwrap();
    let mut frames = Frames::open("target/kernels/cassini/cas_v44.tf").unwrap();
    frames.attach_ck(CkKernel::open("target/kernels/cassini/12289_12294ra.bc").unwrap());
    frames.attach_sclk(clock.clone(), lsk.clone());
    let (mut checked, mut different) = (0, 0);
    for line in include_str!("golden/cassini_fov_frames.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let et = f[0].parse().unwrap();
        let ticks = clock.et_to_ticks(et, &lsk).unwrap();
        let rotation = frames
            .pxform_with_derivative("CASSINI_ISS_NAC", "J2000", et, None)
            .unwrap()
            .rotation;
        let actual = std::iter::once(ticks).chain(rotation.iter().flatten().copied());
        for (i, a) in actual.enumerate() {
            let e: f64 = f[i + 1].parse().unwrap();
            if a.to_bits() != e.to_bits() {
                if different < 5 {
                    eprintln!("et{et} column{i} actual{a:?} expected{e:?}");
                }
                different += 1;
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 120);
    assert_eq!(different, 0);
}
