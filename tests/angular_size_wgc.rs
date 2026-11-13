//! Displayed WGC limb extents retained in the live-response evidence.
use std::io::Write;
use std::process::{Command, Stdio};
use stkcraft_geocalc::{
    angular_size_center_ray_extents, angular_size_extents, angular_size_extents_in,
    angular_size_with_method, Aberration, AngularSizeMethod, Frames, Pck, SpkKernel,
};

#[test]
fn limb_extents_cli_uses_explicit_target_frame() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .args([
            "size",
            "target/kernels/pck00011.tpc,target/kernels/earth_1962_260806_2126_combined.bpc",
            "-",
        ])
        .env("STKCRAFT_SPK", "target/kernels/de442.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"301,399,0,NONE,LIMB_EXTENTS,ITRF93\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8(output.stdout).unwrap();
    let fields: Vec<_> = output.trim().split(',').collect();
    assert_eq!(fields.len(), 7, "{output}");
    assert_eq!(
        format!("{:.8}", fields[5].parse::<f64>().unwrap()),
        "1.81029134"
    );
    assert_eq!(
        format!("{:.8}", fields[6].parse::<f64>().unwrap()),
        "1.81616018"
    );
}

#[test]
fn limb_extents_use_explicit_target_frame() {
    let kernel = SpkKernel::open("target/kernels/de442.bsp").unwrap();
    let pck = Pck::open_all(&[
        "target/kernels/pck00011.tpc",
        "target/kernels/earth_1962_260806_2126_combined.bpc",
    ])
    .unwrap();
    let frames = Frames::new();
    let mut checked = 0;
    for line in include_str!("golden/angular_size_frames_exact.csv")
        .lines()
        .skip(2)
    {
        let f: Vec<_> = line.split(',').collect();
        let result = angular_size_extents_in(
            &kernel,
            &pck,
            &frames,
            f[0],
            301,
            399,
            f[2].parse().unwrap(),
            Aberration::parse(f[1]).unwrap(),
        )
        .unwrap();
        let expected = [
            u64::from_str_radix(f[3], 16).unwrap(),
            u64::from_str_radix(f[4], 16).unwrap(),
        ];
        assert_eq!(result.map(f64::to_bits), expected, "{line}");
        checked += 1;
    }
    assert_eq!(checked, 18);
    let result = angular_size_extents_in(
        &kernel,
        &pck,
        &frames,
        "ITRF93",
        301,
        399,
        0.0,
        Aberration::None,
    )
    .unwrap();
    assert_eq!(format!("{:.8}", result[0].to_degrees()), "1.81029134");
    assert!(angular_size_extents_in(
        &kernel,
        &pck,
        &frames,
        "IAU_MOON",
        301,
        399,
        0.0,
        Aberration::None
    )
    .is_err());
}

#[test]
fn limb_extents_match_wgc_earth_from_sun() {
    let kernel = SpkKernel::open("target/kernels/de442.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let values = angular_size_extents(&kernel, &pck, 10, 399, 0.0, Aberration::None).unwrap();
    let scalar = angular_size_with_method(
        &kernel,
        &pck,
        10,
        399,
        0.0,
        Aberration::None,
        AngularSizeMethod::LimbExtents,
    )
    .unwrap();
    assert_eq!(scalar.to_bits(), values[0].to_bits());
    let center_ray =
        angular_size_center_ray_extents(&kernel, &pck, 10, 399, 0.0, Aberration::None).unwrap();
    assert_ne!(center_ray[0].to_bits(), values[0].to_bits());
    // Independent CSPICE EDLIMB, VNORM and atan2 calculation on Windows.
    assert_eq!(
        values.map(f64::to_bits),
        [0x3f16aaebd0e5434f, 0x3f16bb6c9f0f627c]
    );
    assert_eq!(format!("{:.8}", values[0].to_degrees()), "0.00495438");
    assert_eq!(format!("{:.8}", values[1].to_degrees()), "0.00496847");
    for (epoch, expected) in [
        (0.0, [0x3f16aaebb5ded446, 0x3f16bb6c7ca70de4]),
        (500000000.0, [0x3f16793b0f76a0c6, 0x3f168b283c03fb5c]),
    ] {
        let actual =
            angular_size_extents(&kernel, &pck, 10, 399, epoch, Aberration::LightTimeStellar)
                .unwrap();
        assert_eq!(actual.map(f64::to_bits), expected);
    }
}

#[test]
fn coincident_observer_reports_wgc_edlimb_error() {
    let kernel = SpkKernel::open("target/kernels/de442.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let result = angular_size_extents(&kernel, &pck, 399, 399, 0.0, Aberration::None).unwrap_err();
    assert_eq!(result.to_string(), "SPICE(DEGENERATECASE)");
    let scalar = angular_size_with_method(
        &kernel,
        &pck,
        399,
        399,
        0.0,
        Aberration::None,
        AngularSizeMethod::LimbExtents,
    )
    .unwrap_err();
    assert_eq!(scalar.to_string(), "SPICE(DEGENERATECASE)");
}
