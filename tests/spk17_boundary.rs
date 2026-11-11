use stkcraft_geocalc::SpkKernel;
#[test]
fn spk17_accepts_native_exact_eccentricity_boundary() {
    let kernel = SpkKernel::open("tests/golden/spk17_boundary.bsp").unwrap();
    let mut checked = 0;
    for line in include_str!("golden/spk17_boundary.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let actual = kernel
            .state_inertial(
                f[0].parse().unwrap(),
                0,
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
            )
            .unwrap();
        for (i, value) in actual
            .position
            .iter()
            .chain(actual.velocity.iter())
            .enumerate()
        {
            assert_eq!(
                value.to_bits(),
                f[i + 3].parse::<f64>().unwrap().to_bits(),
                "{line}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 216);
}

#[test]
fn spk17_rejects_native_kepleq_eccentricity_boundary_error() {
    let kernel = SpkKernel::open("tests/golden/spk17_boundary.bsp").unwrap();
    // N0067 EQNCPV accepts initial ECC==0.9, then KEPLEQ checks the
    // propagated H*H + K*K sum. At this epoch it signals ECCOUTOFBOUNDS.
    assert!(kernel.state(717100, 0, 0.).is_err());
}
