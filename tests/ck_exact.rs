use stkcraft_geocalc::{ck::CkKernel, SpkKernel};
#[test]
fn native_ck_encoded_tick_pointing_exact() {
    let a = "tests/golden/ck_types_a.bc";
    let b = "tests/golden/ck_types_b.bc";
    let ab = CkKernel::open_all(&[a, b]).unwrap();
    let ba = CkKernel::open_all(&[b, a]).unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    let mut found = 0;
    for line in include_str!("golden/ck_ticks.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let ck = if f[0] == "ab" { &ab } else { &ba };
        let inst = f[1].parse().unwrap();
        let ticks = f[2].parse().unwrap();
        let tol = f[3].parse().unwrap();
        let reference = f[5].parse().unwrap();
        let result = if f[4] == "1" {
            ck.ckgpav(inst, ticks, tol, reference)
        } else {
            ck.ckgp(inst, ticks, tol, reference)
        };
        let expected_found = f[6] == "1";
        if expected_found {
            found += 1;
        }
        match result {
            Ok(Some(p)) if expected_found => {
                let mut actual = vec![p.clock];
                actual.extend(p.rotation.concat());
                let mut expected: Vec<f64> = f[7..17].iter().map(|v| v.parse().unwrap()).collect();
                if f[4] == "1" {
                    actual.extend(p.angular_velocity.unwrap());
                    expected.extend(f[17..20].iter().map(|v| v.parse::<f64>().unwrap()));
                }
                if actual != expected || p.reference_frame != reference {
                    errors.push(format!(
                        "{} {} {} {} {} {}:actual{actual:?} expected{expected:?}",
                        f[0], f[1], f[2], f[3], f[4], f[5]
                    ));
                }
            }
            Ok(None) if !expected_found => {}
            other => errors.push(format!(
                "{} {} {} {} {} {}:actual{other:?} expectedfound{expected_found}",
                f[0], f[1], f[2], f[3], f[4], f[5]
            )),
        };
        count += 1;
    }
    assert_eq!(count, 32622);
    assert!(found > 10000);
    assert!(
        errors.is_empty(),
        "{}mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn ck_container_cannot_be_loaded_as_spk() {
    assert!(SpkKernel::open("tests/golden/daf_attitude.bc").is_err());
}

#[test]
fn native_ck_frames_use_exact_spacecraft_clock_conversion() {
    use stkcraft_geocalc::Frames;
    use stkcraft_time::{sclk::SclkKernel, LeapSecondTable};
    let mut frames = Frames::open("tests/golden/ck_test.tf").unwrap();
    let lsk =
        LeapSecondTable::from_lsk(&std::fs::read_to_string("target/kernels/naif0012.tls").unwrap())
            .unwrap();
    let text = std::fs::read_to_string("tests/golden/ck_test.tsc").unwrap();
    frames.attach_ck(
        CkKernel::open_all(&["tests/golden/ck_types_a.bc", "tests/golden/ck_types_b.bc"]).unwrap(),
    );
    for clock in [-100, -200, -300] {
        frames.attach_sclk(SclkKernel::parse(&text, clock).unwrap(), lsk.clone());
    }
    let mut errors = Vec::new();
    let mut count = 0;
    let mut found = 0;
    for line in include_str!("golden/ck_frames.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let et = f[3].parse().unwrap();
        let expected_found = f[4] == "1";
        if expected_found {
            found += 1;
        }
        let result = if f[0] == "state" {
            frames
                .pxform_with_derivative(f[1], f[2], et, None)
                .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat())
        } else {
            frames.pxform(f[1], f[2], et).map(|x| x.concat())
        };
        match result {
            Ok(actual) if expected_found => {
                let expected: Vec<f64> = f[5..if f[0] == "state" { 23 } else { 14 }]
                    .iter()
                    .map(|x| x.parse().unwrap())
                    .collect();
                if actual != expected {
                    errors.push(format!("{line}:actual{actual:?}"));
                }
            }
            Err(_) if !expected_found => {}
            other => errors.push(format!("{line}:actual{other:?}")),
        };
        count += 1;
    }
    assert_eq!(count, 420);
    assert!(found > 100);
    assert!(
        errors.is_empty(),
        "{}mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn native_sclk_ticks_for_ck_frames() {
    use stkcraft_time::{sclk::SclkKernel, LeapSecondTable};
    let lsk =
        LeapSecondTable::from_lsk(&std::fs::read_to_string("target/kernels/naif0012.tls").unwrap())
            .unwrap();
    let text = std::fs::read_to_string("tests/golden/ck_test.tsc").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/ck_sclk_ticks.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let clock = SclkKernel::parse(&text, f[0].parse().unwrap()).unwrap();
        let actual = clock.et_to_ticks(f[1].parse().unwrap(), &lsk).unwrap();
        let expected: f64 = f[2].parse().unwrap();
        if actual != expected {
            errors.push(format!("{line}:actual{actual:?}"));
        }
        count += 1;
    }
    assert_eq!(count, 42);
    assert!(
        errors.is_empty(),
        "{}tickmismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn native_ck_tdb_frame_subset_exact() {
    use stkcraft_geocalc::Frames;
    use stkcraft_time::{sclk::SclkKernel, LeapSecondTable};
    let mut frames = Frames::open("tests/golden/ck_test.tf").unwrap();
    let lsk =
        LeapSecondTable::from_lsk(&std::fs::read_to_string("target/kernels/naif0012.tls").unwrap())
            .unwrap();
    let text = std::fs::read_to_string("tests/golden/ck_test.tsc").unwrap();
    frames.attach_ck(
        CkKernel::open_all(&["tests/golden/ck_types_a.bc", "tests/golden/ck_types_b.bc"]).unwrap(),
    );
    for clock in [-100, -200] {
        frames.attach_sclk(SclkKernel::parse(&text, clock).unwrap(), lsk.clone());
    }
    let mut count = 0;
    for line in include_str!("golden/ck_frames.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        if [f[1], f[2]]
            .iter()
            .any(|x| x.contains("T3") || x.contains("CHILD"))
        {
            continue;
        }
        let et = f[3].parse().unwrap();
        let expected_found = f[4] == "1";
        let result = if f[0] == "state" {
            frames
                .pxform_with_derivative(f[1], f[2], et, None)
                .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat())
        } else {
            frames.pxform(f[1], f[2], et).map(|x| x.concat())
        };
        match result {
            Ok(actual) if expected_found => {
                let expected: Vec<f64> = f[5..if f[0] == "state" { 23 } else { 14 }]
                    .iter()
                    .map(|x| x.parse().unwrap())
                    .collect();
                assert_eq!(actual, expected, "{line}");
            }
            Err(_) if !expected_found => {}
            other => panic!("{line}:actual{other:?}"),
        };
        count += 1;
    }
    assert_eq!(count, 168);
}
#[test]
fn native_valid_scalar_sclk_kernel_is_accepted() {
    use stkcraft_time::sclk::SclkKernel;
    let text = std::fs::read_to_string("tests/golden/ck_test_scalar.tsc").unwrap();
    for clock in [-100, -200, -300] {
        assert!(
            SclkKernel::parse(&text, clock).is_ok(),
            "nativeFURNSHvalidscalarSCLK{clock}"
        );
    }
}
