use stkcraft_geocalc::{ck::CkKernel, Frames};
use stkcraft_time::{sclk::SclkKernel, LeapSecondTable};

#[test]
fn native_ck456_big_endian_pointing_exact() {
    let kernels: Vec<_> = (4..=6)
        .map(|kind| CkKernel::open(format!("tests/golden/ck{kind}_big.bc")).unwrap())
        .collect();
    let mut count = 0;
    let mut failures = Vec::new();
    for line in include_str!("golden/ck_extended_endian.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let kind = f[0].as_bytes()[2] - b'4';
        let ck = &kernels[kind as usize];
        let (inst, t, tol, reference) = (
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
            f[5].parse().unwrap(),
        );
        let result = if f[4] == "1" {
            ck.ckgpav(inst, t, tol, reference)
        } else {
            ck.ckgp(inst, t, tol, reference)
        };
        match result {
            Ok(Some(p)) if f[6] == "FOUND" => {
                let mut actual = vec![p.clock];
                actual.extend(p.rotation.concat());
                if f[4] == "1" {
                    actual.extend(p.angular_velocity.unwrap());
                }
                let expected: Vec<f64> = f[7..if f[4] == "1" { 20 } else { 17 }]
                    .iter()
                    .map(|v| v.parse().unwrap())
                    .collect();
                if actual != expected {
                    failures.push(format!("{line}:actual{actual:?}"));
                }
            }
            Ok(None) if f[6] == "NONE" => {}
            Err(e) if e.to_string().contains(f[6]) => {}
            other => failures.push(format!("{line}:{other:?}")),
        }
        count += 1;
    }
    assert!(count >= 360);
    assert!(
        failures.is_empty(),
        "{} mismatches/{count}:{}",
        failures.len(),
        failures
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn native_ck456_sclk_frame_transforms_exact() {
    let mut frames = Frames::open("tests/golden/ck_extended.tf").unwrap();
    frames.attach_ck(
        CkKernel::open_all(&[
            "tests/golden/ck4.bc",
            "tests/golden/ck5.bc",
            "tests/golden/ck6.bc",
        ])
        .unwrap(),
    );
    let clocks = std::fs::read_to_string("tests/golden/ck_extended.tsc").unwrap();
    let lsk =
        LeapSecondTable::from_lsk(&std::fs::read_to_string("target/kernels/naif0012.tls").unwrap())
            .unwrap();
    for clock in [-740, -750, -760] {
        frames.attach_sclk(SclkKernel::parse(&clocks, clock).unwrap(), lsk.clone());
    }
    let mut count = 0;
    let mut found = 0;
    let mut failures = Vec::new();
    for line in include_str!("golden/ck_extended_frames.csv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = line.split(',').collect();
        let et = f[3].parse().unwrap();
        let result = if f[0] == "1" {
            frames
                .pxform_with_derivative(f[1], f[2], et, None)
                .map(|x| [x.rotation.concat(), x.rotation_dot.concat()].concat())
        } else {
            frames.pxform(f[1], f[2], et).map(|x| x.concat())
        };
        match result {
            Ok(actual) if f[4] == "FOUND" => {
                found += 1;
                let expected: Vec<f64> = f[5..].iter().map(|v| v.parse().unwrap()).collect();
                if actual != expected {
                    failures.push(format!("{line}:actual{actual:?}"));
                }
            }
            Err(_) if f[4] != "FOUND" => {}
            other => failures.push(format!("{line}:{other:?}")),
        }
        count += 1;
    }
    assert_eq!(count, 832);
    assert!(found > 100);
    assert!(
        failures.is_empty(),
        "{} mismatches/{count}:{}",
        failures.len(),
        failures
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
