use stkcraft_geocalc::Pck;
#[test]
fn zero_valued_pck_models_are_valid() {
    let pck = Pck::from_text(include_str!("golden/pck_models_exact.tpc")).unwrap();
    for line in include_str!("golden/pck_models_exact.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let body = f[0].parse().unwrap();
        let et = f[1].parse().unwrap();
        let (r, d) = pck
            .body_state_rotation(body, et)
            .expect("zero-valued coefficients define a valid model");
        let actual: Vec<_> = r
            .into_iter()
            .flatten()
            .chain(d.into_iter().flatten())
            .collect();
        let expected: Vec<f64> = f[2..].iter().map(|x| x.parse().unwrap()).collect();
        assert_eq!(actual, expected, "body{body} et{et}");
    }
}
#[test]
fn incomplete_orientation_is_not_filled_with_zeroes() {
    let pck=Pck::from_text("\\begindata\nBODY999004_POLE_RA = ( 0,0,0 )\nBODY999004_POLE_DEC = ( 90,0,0 )\n\\begintext").unwrap();
    assert!(pck.body_state_rotation(999004, 0.).is_none());
    assert!(pck.prime_meridian_rate(999004).is_none());
}
