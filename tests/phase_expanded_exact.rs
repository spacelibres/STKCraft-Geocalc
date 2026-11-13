//! CSPICE PHASEQ results for additional WGC epochs and corrections.
use stkcraft_geocalc::{phase_angle, Aberration, SpkKernel};

#[test]
fn phase_angle_additional_corrections_match_native_bits() {
    let kernel = SpkKernel::open("target/kernels/de442.bsp").unwrap();
    let mut count = 0;
    for line in include_str!("golden/phase_expanded_exact.csv")
        .lines()
        .skip(2)
    {
        let fields: Vec<_> = line.split(',').collect();
        let angle = phase_angle(
            &kernel,
            fields[0].parse().unwrap(),
            fields[1].parse().unwrap(),
            fields[2].parse().unwrap(),
            fields[3].parse().unwrap(),
            Aberration::parse(fields[4]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            angle.to_bits(),
            u64::from_str_radix(fields[5], 16).unwrap(),
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 18);
}
