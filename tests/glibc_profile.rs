#![cfg(feature = "wgc-glibc-rotation")]
//! Independent Linux CSPICE N0067 cases for the explicit glibc rotation profile.
use std::io::Write;
use std::process::{Command, Stdio};

fn run(mode: &str, input: &str, glibc: bool) -> Vec<Vec<String>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"));
    command.args([mode,"target/kernels/pck00011.tpc,target/kernels/mars_iau2000_v1.tpc,target/kernels/gm_de431.tpc,target/kernels/moon_pa_de440_200625.bpc","target/kernels/moon_de440_250416.tf,target/kernels/moon_assoc_me.tf"])
        .env("STKCRAFT_SPK","target/kernels/de442.bsp").stdin(Stdio::piped())
        .stdout(Stdio::piped()).stderr(Stdio::piped());
    if glibc {
        command.env("STKCRAFT_WGC_GLIBC_ROT3", "1");
    } else {
        command.env_remove("STKCRAFT_WGC_GLIBC_ROT3");
    }
    let mut child = command.spawn().unwrap();
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
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| line.split(',').map(str::to_string).collect())
        .collect()
}

#[test]
fn surface_and_tangent_match_linux_cspice() {
    let rows=run("subpoint","observer,301,399,0,CN+S,NEAR_POINT_ELLIPSOID,IAU_MOON\nobserver,301,399,0,CN+S,INTERCEPT_ELLIPSOID,IAU_MOON\n",true);
    assert_eq!(rows.len(), 2);
    for (row, reference) in rows
        .iter()
        .zip([4.967179787420746e-13_f64, 4.950566822287042e-13_f64])
    {
        assert_eq!(
            row[16].parse::<f64>().unwrap().to_bits(),
            reference.to_bits()
        );
    }
    let rows=run("tangent","301,399,0,LT+S,0,0,0,TANGENT_POINT,IAU_MOON\n301,399,0,CN+S,0,0,0,TANGENT_POINT,IAU_MOON\n",true);
    assert_eq!(rows.len(), 2);
    for (row, reference) in rows
        .iter()
        .zip([-3.1585034756325834e-11_f64, 7.603899392373249e-11_f64])
    {
        assert_eq!(
            row[19].parse::<f64>().unwrap().to_bits(),
            reference.to_bits()
        );
    }
}

#[cfg(windows)]
#[test]
fn native_windows_profile_remains_default() {
    let rows=run("subpoint","observer,301,399,0,CN+S,NEAR_POINT_ELLIPSOID,IAU_MOON\nobserver,301,399,0,CN+S,INTERCEPT_ELLIPSOID,IAU_MOON\n",false);
    for (row, reference) in rows
        .iter()
        .zip([4.938188851404903e-13_f64, 4.93094333424261e-13_f64])
    {
        assert_eq!(
            row[16].parse::<f64>().unwrap().to_bits(),
            reference.to_bits()
        );
    }
    let rows=run("tangent","301,399,0,LT+S,0,0,0,TANGENT_POINT,IAU_MOON\n301,399,0,CN+S,0,0,0,TANGENT_POINT,IAU_MOON\n",false);
    for (row, reference) in rows
        .iter()
        .zip([-3.217017937901701e-11_f64, 7.486870357508969e-11_f64])
    {
        assert_eq!(
            row[19].parse::<f64>().unwrap().to_bits(),
            reference.to_bits()
        );
    }
}
