use std::process::Command;

#[test]
fn backend_reports_actual_rotation_profile() {
    for binary in [
        env!("CARGO_BIN_EXE_wgc_geometry"),
        env!("CARGO_BIN_EXE_wgc_gf"),
    ] {
        let native = Command::new(binary)
            .arg("--rotation-profile")
            .env_remove("STKCRAFT_WGC_GLIBC_ROT3")
            .output()
            .unwrap();
        assert!(native.status.success());
        assert_eq!(String::from_utf8(native.stdout).unwrap().trim(), "native");
        let requested = Command::new(binary)
            .arg("--rotation-profile")
            .env("STKCRAFT_WGC_GLIBC_ROT3", "1")
            .output()
            .unwrap();
        assert!(requested.status.success());
        let expected = if cfg!(feature = "wgc-glibc-rotation") {
            "glibc-fma-r3"
        } else {
            "native"
        };
        assert_eq!(
            String::from_utf8(requested.stdout).unwrap().trim(),
            expected
        );
    }
}
