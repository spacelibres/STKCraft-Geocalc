use stkcraft_geocalc::{Frames, Pck};
#[test]
fn native_dynamic_euler_frames_exact() {
    let frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/dynamic_euler.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let et = f[3].parse().unwrap();
        let actual = if f[0] == "rotation" {
            frames
                .pxform_in(f[1], f[2], et, Some(&pck))
                .map(|r| r.concat())
        } else {
            frames
                .pxform_with_derivative(f[1], f[2], et, Some(&pck))
                .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat())
        };
        let expected: Vec<f64> = f[4..].iter().map(|x| x.parse().unwrap()).collect();
        if actual.as_ref().ok() != Some(&expected) {
            errors.push(format!(
                "{} {} {} {et}: {actual:?} expected{expected:?}",
                f[0], f[1], f[2]
            ));
        }
        count += 1;
    }
    assert_eq!(count, 12960);
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn native_dynamic_euler_public_sxform_exact() {
    let frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    let mut count = 0;
    for line in include_str!("golden/dynamic_euler.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        if f[0] != "state" {
            continue;
        }
        let name = if f[1].starts_with("DYN_EULER_") {
            f[1]
        } else {
            f[2]
        };
        let id: usize = name.strip_prefix("DYN_EULER_").unwrap().parse().unwrap();
        if id % 4 >= 2 {
            continue;
        }
        let et = f[3].parse().unwrap();
        let expected: Vec<f64> = f[4..].iter().map(|x| x.parse().unwrap()).collect();
        let actual = frames.sxform(f[1], f[2], et).unwrap();
        for i in 0..6 {
            for j in 0..6 {
                let value = if i < 3 && j < 3 {
                    expected[i * 3 + j]
                } else if i >= 3 && j >= 3 {
                    expected[(i - 3) * 3 + j - 3]
                } else if i >= 3 {
                    expected[9 + (i - 3) * 3 + j]
                } else {
                    0.0
                };
                assert_eq!(actual[i][j], value, "{name} {et} {i}/{j}");
            }
        }
        count += 1;
    }
    assert_eq!(count, 3240);
}
#[test]
fn native_dynamic_constant_two_vector_frames_exact() {
    let mut frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    frames
        .furnsh(&std::fs::read_to_string("tests/golden/dynamic_two.tf").unwrap())
        .unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/dynamic_two.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let et = f[3].parse().unwrap();
        let actual = if f[0] == "rotation" {
            frames
                .pxform_in(f[1], f[2], et, Some(&pck))
                .map(|r| r.concat())
        } else {
            frames
                .pxform_with_derivative(f[1], f[2], et, Some(&pck))
                .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat())
        };
        if f[4] == "OK" {
            let expected: Vec<f64> = f[5..].iter().map(|x| x.parse().unwrap()).collect();
            if actual.as_ref().ok() != Some(&expected) {
                errors.push(format!(
                    "{} {} {} {et}: {actual:?} expected{expected:?}",
                    f[0], f[1], f[2]
                ));
            }
        } else if actual.is_ok() {
            errors.push(format!("expected{}: {actual:?}", f[4]));
        }
        count += 1;
    }
    assert_eq!(count, 6912);
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn native_dynamic_product_frames_exact() {
    let mut frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    for path in [
        "tests/golden/dynamic_two.tf",
        "tests/golden/dynamic_product.tf",
    ] {
        frames
            .furnsh(&std::fs::read_to_string(path).unwrap())
            .unwrap();
    }
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/dynamic_product.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let et = f[3].parse().unwrap();
        let actual = if f[0] == "rotation" {
            frames
                .pxform_in(f[1], f[2], et, Some(&pck))
                .map(|r| r.concat())
        } else {
            frames
                .pxform_with_derivative(f[1], f[2], et, Some(&pck))
                .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat())
        };
        if f[4] == "OK" {
            let expected: Vec<f64> = f[5..].iter().map(|x| x.parse().unwrap()).collect();
            if actual.as_ref().ok() != Some(&expected) {
                errors.push(format!(
                    "{} {} {} {et}: {actual:?} expected{expected:?}",
                    f[0], f[1], f[2]
                ));
            }
        } else if actual.is_ok() {
            errors.push(format!("expected{}: {actual:?}", f[4]));
        }
        count += 1;
    }
    assert_eq!(count, 1176);
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn native_switch_frames_exact() {
    use stkcraft_geocalc::ck::CkKernel;
    use stkcraft_time::{sclk::SclkKernel, LeapSecondTable};
    let mut frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    for path in ["tests/golden/ck_test.tf", "tests/golden/switch_frames.tf"] {
        frames
            .furnsh(&std::fs::read_to_string(path).unwrap())
            .unwrap();
    }
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let text = std::fs::read_to_string("tests/golden/ck_test.tsc").unwrap();
    let lsk =
        LeapSecondTable::from_lsk(&std::fs::read_to_string("target/kernels/naif0012.tls").unwrap())
            .unwrap();
    frames.attach_ck(
        CkKernel::open_all(&["tests/golden/ck_types_a.bc", "tests/golden/ck_types_b.bc"]).unwrap(),
    );
    for clock in [-100, -200, -300] {
        frames.attach_sclk(SclkKernel::parse(&text, clock).unwrap(), lsk.clone());
    }
    let mut errors = Vec::new();
    let mut count = 0;
    let mut found = 0;
    for line in include_str!("golden/switch_frames.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let et = f[3].parse().unwrap();
        let actual = if f[0] == "rotation" {
            frames
                .pxform_in(f[1], f[2], et, Some(&pck))
                .map(|r| r.concat())
        } else {
            frames
                .pxform_with_derivative(f[1], f[2], et, Some(&pck))
                .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat())
        };
        if f[4] == "OK" {
            found += 1;
            let expected: Vec<f64> = f[5..].iter().map(|x| x.parse().unwrap()).collect();
            if actual.as_ref().ok() != Some(&expected) {
                errors.push(format!(
                    "{} {} {} {et}: {actual:?} expected{expected:?}",
                    f[0], f[1], f[2]
                ));
            }
        } else if actual.is_ok() {
            errors.push(format!(
                "expected{} {} {} {et}: {actual:?}",
                f[4], f[1], f[2]
            ));
        }
        count += 1;
    }
    assert_eq!(count, 1728);
    assert_eq!(found, 1288);
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn native_dynamic_of_date_frames_exact() {
    let mut frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    frames
        .furnsh(&std::fs::read_to_string("tests/golden/dynamic_of_date.tf").unwrap())
        .unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut errors = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/dynamic_of_date.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let et = f[3].parse().unwrap();
        let actual = if f[0] == "rotation" {
            frames
                .pxform_in(f[1], f[2], et, Some(&pck))
                .map(|r| r.concat())
        } else {
            frames
                .pxform_with_derivative(f[1], f[2], et, Some(&pck))
                .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat())
        };
        let expected: Vec<f64> = f[4..].iter().map(|x| x.parse().unwrap()).collect();
        if actual.as_ref().ok() != Some(&expected) {
            errors.push(format!(
                "{} {} {} {et}: {actual:?} expected{expected:?}",
                f[0], f[1], f[2]
            ));
        }
        count += 1;
    }
    assert_eq!(count, 2592);
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn native_dynamic_ephemeris_two_vector_frames_exact() {
    use std::sync::Arc;
    use stkcraft_geocalc::SpkKernel;
    let mut frames = Frames::open("tests/golden/dynamic_euler.tf").unwrap();
    frames
        .furnsh(&std::fs::read_to_string("tests/golden/dynamic_ephemeris.tf").unwrap())
        .unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    frames.attach_spk(Arc::new(
        SpkKernel::open_all(&[
            "target/kernels/de440s.bsp",
            "tests/golden/spk_dynamic_sources.bsp",
        ])
        .unwrap(),
    ));
    let mut errors = Vec::new();
    let mut count = 0;
    let mut found = 0;
    for line in include_str!("golden/dynamic_ephemeris.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let et = f[3].parse().unwrap();
        let actual = if f[0] == "rotation" {
            frames
                .pxform_in(f[1], f[2], et, Some(&pck))
                .map(|r| r.concat())
        } else {
            frames
                .pxform_with_derivative(f[1], f[2], et, Some(&pck))
                .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat())
        };
        if f[4] == "OK" {
            found += 1;
            let expected: Vec<f64> = f[5..].iter().map(|x| x.parse().unwrap()).collect();
            if actual.as_ref().ok() != Some(&expected) {
                errors.push(format!(
                    "{} {} {} {et}: {actual:?} expected{expected:?}",
                    f[0], f[1], f[2]
                ));
            }
        } else if !matches!(&actual,Err(stkcraft_geocalc::GeoError::BadSearch(message)) if *message==f[4])
        {
            errors.push(format!(
                "expected{} {} {} {et}: {actual:?}",
                f[4], f[1], f[2]
            ));
        }
        count += 1;
    }
    assert_eq!(count, 13680);
    assert_eq!(found, 13440);
    assert!(
        errors.is_empty(),
        "{} mismatches/{count}:\n{}",
        errors.len(),
        errors
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn native_frame_body_alias_append_and_precedence() {
    let frames = Frames::open("tests/golden/frame_body_aliases.tf").unwrap();
    for line in include_str!("golden/frame_body_aliases.csv")
        .lines()
        .skip(1)
    {
        let (name, code) = line.split_once(',').unwrap();
        assert_eq!(
            frames.body_id(name).unwrap(),
            code.parse::<i32>().unwrap(),
            "{name}"
        );
    }
}
#[test]
fn native_body_frame_uses_kernel_preferred_body_name() {
    let frames = Frames::open("tests/golden/frame_body_aliases.tf").unwrap();
    assert_eq!(frames.body_frame(301).unwrap(), "ECLIPJ2000");
}
