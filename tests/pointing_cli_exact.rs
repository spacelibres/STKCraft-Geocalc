use std::io::Write;
use std::process::{Command, Stdio};
fn run(input: &str, extra: &[&str]) -> Vec<Vec<String>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wgc_geometry"));
    command
        .args(["pointing_full", "-", "-"])
        .args(extra)
        .env("STKCRAFT_SPK", "target/kernels/de442.bsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
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
        .map(|line| line.split(',').map(str::to_owned).collect())
        .collect()
}
#[test]
fn typed_cli_matches_all_native_coordinate_outputs() {
    let mut queries = String::new();
    let mut expected = Vec::new();
    for line in include_str!("golden/pointing_coordinates_exact.csv")
        .lines()
        .skip(2)
    {
        let f: Vec<_> = line.split(',').collect();
        let rep = if f[0].starts_with("AZ_EL") {
            "AZ_EL"
        } else {
            f[0]
        };
        let flags = if rep == "AZ_EL" { &f[0][6..] } else { "TT" };
        queries.push_str(&format!(
            "VECTOR,0,399,0,NONE,J2000,J2000,{},{},false,VECTOR,{},{},{},X,{},{}\n",
            f[1],
            rep,
            f[2],
            f[3],
            f[4],
            flags.as_bytes()[0] == b'T',
            flags.as_bytes()[1] == b'T'
        ));
        let mut values: [f64; 3] = [
            f[8].parse().unwrap(),
            f[9].parse().unwrap(),
            f[10].parse().unwrap(),
        ];
        if rep != "RECTANGULAR" {
            values[0] = values[0].to_degrees();
            if rep != "CYLINDRICAL" {
                values[1] = values[1].to_degrees();
            }
        }
        expected.push(values);
    }
    let actual = run(&queries, &[]);
    assert_eq!(actual.len(), 180);
    for (row, expected) in actual.iter().zip(expected) {
        assert_eq!(row.len(), 9);
        assert_eq!(&row[..5], &["VECTOR", "0", "399", "0", "0"]);
        for i in 0..3 {
            assert_eq!(
                row[6 + i].parse::<f64>().unwrap().to_bits(),
                expected[i].to_bits(),
                "{row:?}"
            );
        }
    }
}
#[test]
fn typed_cli_returns_every_cspice_getfov_boundary() {
    let ik = std::env::temp_dir().join(format!("stkcraft-pointing-cli-{}.ti", std::process::id()));
    std::fs::write(&ik,"\\begindata\nINS-999_FOV_CLASS_SPEC='CORNERS'\nINS-999_FOV_SHAPE='RECTANGLE'\nINS-999_FOV_FRAME='J2000'\nINS-999_BORESIGHT=(0.1 0.2 7.3)\nINS-999_FOV_BOUNDARY_CORNERS=(1 1 4 -1 1 4 -1 -1 4 1 -1 4)\n\\begintext").unwrap();
    let mut queries = String::new();
    let mut expected = Vec::new();
    for frame in ["J2000", "ECLIPJ2000"] {
        for magnitude in ["UNIT", "PRESERVE_ORIGINAL"] {
            for (kind, ray) in [("BORESIGHT", "BORESIGHT"), ("BOUNDARY", "FOV_BOUNDARY")] {
                queries.push_str(&format!("VECTOR,0,399,0,NONE,J2000,{frame},{magnitude},RECTANGULAR,false,{ray},0,0,0,X,true,true\n"));
                for line in include_str!("golden/pointing_instrument_exact.csv")
                    .lines()
                    .skip(2)
                {
                    let f: Vec<_> = line.split(',').collect();
                    if f[0] == kind && f[2] == frame && f[3] == magnitude {
                        expected.push((
                            f[1].parse::<usize>().unwrap(),
                            [
                                f[7].parse::<f64>().unwrap(),
                                f[8].parse().unwrap(),
                                f[9].parse().unwrap(),
                            ],
                        ));
                    }
                }
            }
        }
    }
    let actual = run(&queries, &[ik.to_str().unwrap(), "-999"]);
    std::fs::remove_file(ik).unwrap();
    assert_eq!(actual.len(), 20);
    assert_eq!(actual.len(), expected.len());
    for (row, (index, expected)) in actual.iter().zip(expected) {
        assert_eq!(row.len(), 9);
        assert_eq!(row[4].parse::<usize>().unwrap(), index);
        for i in 0..3 {
            assert_eq!(
                row[6 + i].parse::<f64>().unwrap().to_bits(),
                expected[i].to_bits(),
                "{row:?}"
            );
        }
    }
}
