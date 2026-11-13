use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn subobserver_solar_time_uses_body_associated_frame() {
    // WGC responses in docs/reference/wgc-calculators/sub-observer-lst.jsonl.
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args([
            "subpoint",
            "target/kernels/pck00011.tpc,target/kernels/moon_pa_de440_200625.bpc",
            "target/kernels/moon_de440_250416.tf,target/kernels/moon_assoc_me.tf",
        ])
        .env("STKCRAFT_SPK", "target/kernels/de442.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"observer,301,399,0,CN+S,NEAR_POINT/ELLIPSOID,IAU_MOON\nobserver,301,399,60,CN+S,NEAR_POINT/ELLIPSOID,IAU_MOON\nobserver,301,399,300,CN+S,NEAR_POINT/ELLIPSOID,IAU_MOON\nobserver,301,399,300,LT,NEAR_POINT/ELLIPSOID,MOON_ME\nobserver,399,301,0,CN+S,NEAR_POINT/ELLIPSOID,IAU_EARTH\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let actual: Vec<_> = stdout
        .lines()
        .map(|line| line.rsplit(',').next().unwrap())
        .collect();
    assert_eq!(
        actual,
        ["20:11:20", "20:11:21", "20:11:29", "20:11:29", "08:04:39"]
    );
}

#[test]
fn subpoint_cli_rejects_unknown_kind_without_calculating() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args(["subpoint", "target/kernels/pck00011.tpc", "-"])
        .env("STKCRAFT_SPK", "target/kernels/de440s.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"garbage,301,399,0,NONE,NEAR_POINT/ELLIPSOID,IAU_MOON\nobserver,301,399,0,NONE,NEAR_POINT/ELLIPSOID,IAU_MOON\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = stdout.lines().collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], "ERROR,unknown sub-point kind garbage");
    assert!(rows[1].starts_with("observer,301,399,"));
}
