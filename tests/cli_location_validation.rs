use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn location_cli_rejects_unknown_frame_locus_without_calculating() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let prefix = "OBJECT,301,0,J2000,0,0,0,0,0,0,0,OBJECT,399,0,J2000,0,0,0,0,0,0,0,J2000,SPICE";
    let input = format!(
        "{prefix},garbage,NONE,RECTANGULAR,1,1,0\n{prefix},CENTER,NONE,RECTANGULAR,1,1,0\n"
    );
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0], "ERROR,unknown frame locus garbage");
    assert!(rows[1].starts_with("COLS,"));
    assert!(rows[2].starts_with("ROW,"));
}

#[test]
fn location_cli_rejects_unknown_frame_type_without_calculating() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let prefix = "OBJECT,301,0,J2000,0,0,0,0,0,0,0,OBJECT,399,0,J2000,0,0,0,0,0,0,0,J2000";
    let input = format!("{prefix},garbage,CENTER,NONE,RECTANGULAR,1,1,0\n{prefix},SPICE_FRAME,CENTER,NONE,RECTANGULAR,1,1,0\n");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0], "ERROR,unknown frame type garbage");
    assert!(rows[1].starts_with("COLS,"));
    assert!(rows[2].starts_with("ROW,"));
}

#[test]
fn location_cli_rejects_unknown_endpoint_kind_without_calculating() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let target = "0,301,J2000,1,0,0,0,0,0,0";
    let observer = "OBJECT,399,0,J2000,0,0,0,0,0,0,0";
    let suffix = "J2000,SPICE_FRAME,CENTER,NONE,RECTANGULAR,1,1,0";
    let input =
        format!("garbage,{target},{observer},{suffix}\nFIXED_POINT,{target},garbage,399,0,J2000,0,0,0,0,0,0,0,{suffix}\nFIXED_POINT,{target},{observer},{suffix}\nCONSTANT_VELOCITY_POINT,{target},{observer},{suffix}\n");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), 6);
    assert_eq!(rows[0], "ERROR,unknown target location type garbage");
    assert_eq!(rows[1], "ERROR,unknown observer location type garbage");
    assert!(rows[2].starts_with("COLS,"));
    assert!(rows[3].starts_with("ROW,"));
    assert!(rows[4].starts_with("COLS,"));
    assert!(rows[5].starts_with("ROW,"));
}

#[test]
fn location_cli_fixed_point_never_uses_velocity() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let observer = "OBJECT,399,0,J2000,0,0,0,0,0,0,0";
    let suffix = "J2000,SPICE_FRAME,CENTER,NONE,RECTANGULAR,1,1,100";
    let input = format!("FIXED_POINT,0,301,J2000,1,0,0,1,0,0,10,{observer},{suffix}\nOBJECT,301,0,J2000,0,0,0,0,0,0,0,FIXED_POINT,0,399,J2000,1,0,0,1,0,0,10,{suffix}\nFIXED_POINT,0,301,J2000,1,0,0,0,0,0,10,{observer},{suffix}\nCONSTANT_VELOCITY_POINT,0,301,J2000,1,0,0,1,0,0,10,{observer},{suffix}\n");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), 6);
    assert_eq!(rows[0], "ERROR,fixed target point must have zero velocity");
    assert_eq!(
        rows[1],
        "ERROR,fixed observer point must have zero velocity"
    );
    let fixed: Vec<_> = rows[3].split(',').collect();
    let moving: Vec<_> = rows[5].split(',').collect();
    let difference = moving[3].parse::<f64>().unwrap() - fixed[3].parse::<f64>().unwrap();
    assert_eq!(difference, 90.0);
    let actual: Vec<_> = moving[3..9]
        .iter()
        .chain(std::iter::once(&moving[10]))
        .map(|value| value.parse::<f64>().unwrap().to_bits())
        .collect();
    assert_eq!(
        actual,
        [
            0xc111c9f416ebd23e,
            0xc110487dbc63ac5b,
            0xc0f29649e0ac8dab,
            0x3ffa4ca6a03e40d0,
            0xbfe54f3fa5faed72,
            0xbfd348291f02b597,
            0x3ff579b6700e3840
        ]
    );
}

#[test]
fn location_cli_rejects_invalid_orientation_flags() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let prefix = "OBJECT,301,0,J2000,0,0,0,0,0,0,0,OBJECT,399,0,J2000,0,0,0,0,0,0,0,J2000,SPICE_FRAME,CENTER,NONE,RECTANGULAR";
    let input = format!("{prefix},garbage,1,0\n{prefix},0,garbage,0\n{prefix},0,1,0\n");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0], "ERROR,azccw must be 0 or 1");
    assert_eq!(rows[1], "ERROR,elplsz must be 0 or 1");
    assert!(rows[2].starts_with("COLS,"));
    assert!(rows[3].starts_with("ROW,"));
}

#[test]
fn moving_observer_cli_matches_spkcvo_bits() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"OBJECT,301,0,J2000,0,0,0,0,0,0,0,CONSTANT_VELOCITY_POINT,0,399,J2000,1,0,0,1,0,0,10,J2000,SPICE_FRAME,CENTER,NONE,RECTANGULAR,1,1,100\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), 2);
    let values: Vec<_> = rows[1].split(',').collect();
    let bits: Vec<_> = values[3..9]
        .iter()
        .chain(std::iter::once(&values[10]))
        .map(|value| value.parse::<f64>().unwrap().to_bits())
        .collect();
    assert_eq!(
        bits,
        [
            0xc111cccc16ebd23e,
            0xc110487dbc63ac5b,
            0xc0f29649e0ac8dab,
            0xbfd6cd657f06fcc0,
            0xbfe54f3fa5faed72,
            0xbfd348291f02b597,
            0x3ff57b8396129845
        ]
    );
}

#[test]
fn moving_location_cli_matches_native_correction_matrix() {
    let cases: Vec<_> = include_str!("../docs/review-2026-11-12/location-cli-native/matrix.csv")
        .lines()
        .skip(1)
        .collect();
    assert_eq!(cases.len(), 18);
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = String::new();
    for case in &cases {
        let fields: Vec<_> = case.split(',').collect();
        assert_eq!(fields.len(), 9);
        let point_target = "CONSTANT_VELOCITY_POINT,0,301,J2000,1,0,0,1,0,0,10";
        let object_target = "OBJECT,301,0,J2000,0,0,0,0,0,0,0";
        let point_observer = "CONSTANT_VELOCITY_POINT,0,399,J2000,1,0,0,1,0,0,10";
        let object_observer = "OBJECT,399,0,J2000,0,0,0,0,0,0,0";
        let (target, observer) = match fields[0] {
            "target" => (point_target, object_observer),
            "observer" => (object_target, point_observer),
            other => panic!("unexpected reference endpoint {other}"),
        };
        input.push_str(&format!(
            "{target},{observer},J2000,SPICE_FRAME,CENTER,{},RECTANGULAR,1,1,100\n",
            fields[1]
        ));
    }
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), cases.len() * 2);
    for (index, case) in cases.iter().enumerate() {
        let reference: Vec<_> = case.split(',').collect();
        assert!(rows[index * 2].starts_with("COLS,"));
        let row: Vec<_> = rows[index * 2 + 1].split(',').collect();
        assert_eq!(row[0], "ROW");
        let actual: Vec<_> = row[3..9]
            .iter()
            .chain(std::iter::once(&row[10]))
            .map(|value| value.parse::<f64>().unwrap().to_bits())
            .collect();
        let expected: Vec<_> = reference[2..]
            .iter()
            .map(|value| u64::from_str_radix(value, 16).unwrap())
            .collect();
        assert_eq!(actual, expected, "{} {}", reference[0], reference[1]);
    }
}

#[test]
fn moving_location_cli_matches_native_rotating_frames() {
    let cases: Vec<_> = include_str!("../docs/review-2026-11-12/location-cli-native/rotating.csv")
        .lines()
        .skip(1)
        .collect();
    assert_eq!(cases.len(), 108);
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = String::new();
    for case in &cases {
        let f: Vec<_> = case.split(',').collect();
        assert_eq!(f.len(), 11);
        let point_target = "CONSTANT_VELOCITY_POINT,0,301,J2000,1,0,0,1,0,0,10";
        let object_target = "OBJECT,301,0,J2000,0,0,0,0,0,0,0";
        let point_observer = "CONSTANT_VELOCITY_POINT,0,399,J2000,1,0,0,1,0,0,10";
        let object_observer = "OBJECT,399,0,J2000,0,0,0,0,0,0,0";
        let (target, observer) = match f[2] {
            "target" => (point_target, object_observer),
            "observer" => (object_target, point_observer),
            other => panic!("unexpected reference endpoint {other}"),
        };
        input.push_str(&format!(
            "{target},{observer},{},SPICE_FRAME,{},{},RECTANGULAR,1,1,100\n",
            f[0], f[1], f[3]
        ));
    }
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), cases.len() * 2);
    for (index, case) in cases.iter().enumerate() {
        let reference: Vec<_> = case.split(',').collect();
        assert!(rows[index * 2].starts_with("COLS,"));
        let row: Vec<_> = rows[index * 2 + 1].split(',').collect();
        assert_eq!(row[0], "ROW");
        let actual: Vec<_> = row[3..9]
            .iter()
            .chain(std::iter::once(&row[10]))
            .map(|value| value.parse::<f64>().unwrap().to_bits())
            .collect();
        let expected: Vec<_> = reference[4..]
            .iter()
            .map(|value| u64::from_str_radix(value, 16).unwrap())
            .collect();
        assert_eq!(
            actual, expected,
            "{} {} {} {}",
            reference[0], reference[1], reference[2], reference[3]
        );
    }
}

#[test]
fn fixed_location_cli_matches_native_frame_matrix() {
    let cases: Vec<_> = include_str!("../docs/review-2026-11-12/location-cli-native/fixed.csv")
        .lines()
        .skip(1)
        .collect();
    assert_eq!(cases.len(), 162);
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = String::new();
    for case in &cases {
        let f: Vec<_> = case.split(',').collect();
        assert_eq!(f.len(), 11);
        let point_target = "FIXED_POINT,0,301,J2000,1,0,0,0,0,0,10";
        let object_target = "OBJECT,301,0,J2000,0,0,0,0,0,0,0";
        let point_observer = "FIXED_POINT,0,399,J2000,1,0,0,0,0,0,10";
        let object_observer = "OBJECT,399,0,J2000,0,0,0,0,0,0,0";
        let (target, observer) = match f[2] {
            "target" => (point_target, object_observer),
            "observer" => (object_target, point_observer),
            other => panic!("unexpected reference endpoint {other}"),
        };
        input.push_str(&format!(
            "{target},{observer},{},SPICE_FRAME,{},{},RECTANGULAR,1,1,100\n",
            f[0], f[1], f[3]
        ));
    }
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), cases.len() * 2);
    for (index, case) in cases.iter().enumerate() {
        let reference: Vec<_> = case.split(',').collect();
        assert!(rows[index * 2].starts_with("COLS,"));
        let row: Vec<_> = rows[index * 2 + 1].split(',').collect();
        assert_eq!(row[0], "ROW");
        let actual: Vec<_> = row[3..9]
            .iter()
            .chain(std::iter::once(&row[10]))
            .map(|value| value.parse::<f64>().unwrap().to_bits())
            .collect();
        let expected: Vec<_> = reference[4..]
            .iter()
            .map(|value| u64::from_str_radix(value, 16).unwrap())
            .collect();
        assert_eq!(
            actual, expected,
            "{} {} {} {}",
            reference[0], reference[1], reference[2], reference[3]
        );
    }
}

#[test]
fn moving_location_cli_matches_native_derived_fields() {
    let cases: Vec<_> = include_str!("../docs/review-2026-11-12/location-cli-native/derived.csv")
        .lines()
        .skip(1)
        .collect();
    assert_eq!(cases.len(), 18);
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["location", "target/kernels/pck00011.tpc"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = String::new();
    for case in &cases {
        let fields: Vec<_> = case.split(',').collect();
        assert_eq!(fields.len(), 5);
        let point_target = "CONSTANT_VELOCITY_POINT,0,301,J2000,1,0,0,1,0,0,10";
        let object_target = "OBJECT,301,0,J2000,0,0,0,0,0,0,0";
        let point_observer = "CONSTANT_VELOCITY_POINT,0,399,J2000,1,0,0,1,0,0,10";
        let object_observer = "OBJECT,399,0,J2000,0,0,0,0,0,0,0";
        let (target, observer) = match fields[0] {
            "target" => (point_target, object_observer),
            "observer" => (object_target, point_observer),
            other => panic!("unexpected reference endpoint {other}"),
        };
        input.push_str(&format!(
            "{target},{observer},J2000,SPICE_FRAME,CENTER,{},RECTANGULAR,1,1,100\n",
            fields[1]
        ));
    }
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), cases.len() * 2);
    for (index, case) in cases.iter().enumerate() {
        let reference: Vec<_> = case.split(',').collect();
        assert!(rows[index * 2].starts_with("COLS,"));
        let row: Vec<_> = rows[index * 2 + 1].split(',').collect();
        assert_eq!(row[0], "ROW");
        let actual: Vec<_> = [row[1], row[2], row[9]]
            .iter()
            .map(|value| value.parse::<f64>().unwrap().to_bits())
            .collect();
        let expected: Vec<_> = reference[2..]
            .iter()
            .map(|value| u64::from_str_radix(value, 16).unwrap())
            .collect();
        assert_eq!(actual, expected, "{} {}", reference[0], reference[1]);
    }
}
