use stkcraft_geocalc::SpkKernel;
#[test]
fn ordered_spk_layers_equal_cspice() {
    let a = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/spk_layer_a.bsp");
    let b = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/spk_layer_b.bsp");
    let ab = SpkKernel::open_all(&[a, b]).unwrap();
    let ba = SpkKernel::open_all(&[b, a]).unwrap();
    let mut count = 0;
    for line in include_str!("golden/spk_layers.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let k = if f[0] == "ab" { &ab } else { &ba };
        let actual = k
            .state(
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
                f[3].parse().unwrap(),
            )
            .unwrap();
        let expected: Vec<f64> = f[4..].iter().map(|s| s.parse().unwrap()).collect();
        assert_eq!(
            [actual.position, actual.velocity].concat(),
            expected,
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 56);
}

#[test]
fn segment_coverage_is_not_extended_by_an_epsilon() {
    let k = SpkKernel::open("tests/golden/spk_priority.bsp").unwrap();
    assert!(k.state(10005, 0, -5e-10).is_err());
    assert!(k.state(10005, 0, 30. + 5e-10).is_err());
}
