//! GFRFOV stellar-only corrections observed in WebGeocalc and CSPICE.
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn ray_finder_stellar_corrections_match_native_windows() {
    assert_eq!(
        stkcraft_geocalc::RayAberration::parse("")
            .unwrap_err()
            .to_string(),
        "SPICE(EMPTYSTRING)"
    );
    assert_eq!(
        stkcraft_geocalc::RayAberration::parse(" ")
            .unwrap_err()
            .to_string(),
        "SPICE(INVALIDOPTION)"
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_gf"))
        .args([
            "target/kernels/cassini/pck00010.tpc,target/kernels/cassini/enceladus_ssd_250728_v2.tpc",
            "target/kernels/cassini/cas_v44.tf",
            "target/kernels/cassini/cas_iss_v10.ti",
            "-82360",
        ])
        .env("STKCRAFT_SPK", "target/kernels/cassini/200128RU_SCPSE_12280_12328.bsp,target/kernels/de442.bsp")
        .env("STKCRAFT_LSK", "target/kernels/naif0012.tls")
        .env("STKCRAFT_CK", "target/kernels/cassini/12289_12294ra.bc")
        .env("STKCRAFT_SCLK", "-82|target/kernels/cassini/cas00172.tsc")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().unwrap();
    let input = "ray_fov_S,GF_RAY_IN_FOV_SEARCH,301,-82,10,10,J2000,IAU_MOON,Near point: ellipsoid,RECTANGULAR,X,INCIDENCE,>,0.0,0.0,S,403902000.0:403906000.0,60.0,0.0,false,NONE,0.0,SECONDS,false,0.0,SECONDS,SECONDS,-0.8695336349509377,-0.4623392348503029,-0.17364817766693033,POINT,POINT,ANY\nray_fov_XS,GF_RAY_IN_FOV_SEARCH,301,-82,10,10,J2000,IAU_MOON,Near point: ellipsoid,RECTANGULAR,X,INCIDENCE,>,0.0,0.0,XS,403902000.0:403906000.0,60.0,0.0,false,NONE,0.0,SECONDS,false,0.0,SECONDS,SECONDS,-0.8695336349509377,-0.4623392348503029,-0.17364817766693033,POINT,POINT,ANY\n";
    let invalid = input
        .lines()
        .next()
        .unwrap()
        .replace("ray_fov_S,", "ray_fov_invalid,")
        .replace(",S,", ",LT,");
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.as_bytes()).unwrap();
    writeln!(stdin, "{invalid}").unwrap();
    writeln!(
        stdin,
        "{}",
        invalid
            .replace("ray_fov_invalid,", "ray_fov_empty,")
            .replace(",LT,", ",,")
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        invalid
            .replace("ray_fov_invalid,", "ray_fov_blank,")
            .replace(",LT,", ", ,")
    )
    .unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.lines()
            .any(|line| line == "ERROR,ray_fov_empty,SPICE(EMPTYSTRING)"),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line == "ERROR,ray_fov_blank,SPICE(INVALIDOPTION)"),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line == "ERROR,ray_fov_invalid,SPICE(INVALIDOPTION)"),
        "{text}"
    );
    let rows: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("ROW,"))
        .collect();
    assert_eq!(rows.len(), 2, "{text}");
    let expected = [
        [403905461.83247435_f64, 403905725.3333049, 263.5008305311203],
        [403905462.5044329_f64, 403905718.7920408, 256.2876079082489],
    ];
    for (row, expected) in rows.iter().zip(expected) {
        let fields: Vec<_> = row.split(',').collect();
        assert_eq!(fields.len(), 6, "{row}");
        for (actual, expected) in fields[3..].iter().zip(expected) {
            assert_eq!(
                actual.parse::<f64>().unwrap().to_bits(),
                expected.to_bits(),
                "{row}"
            );
        }
    }
}
