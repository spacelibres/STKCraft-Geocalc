use std::{
    io::Write,
    process::{Command, Stdio},
};
fn run(binary: &str, args: &[&str], input: &str) -> String {
    let mut child = Command::new(binary)
        .args(args)
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
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
#[test]
fn native_dynamic_pointing_cli_context_exact() {
    let mut input = String::new();
    let mut expected = Vec::new();
    for line in include_str!("golden/cli_ephemeris_context.csv")
        .lines()
        .skip(1)
    {
        let p: Vec<_> = line.split(',').collect();
        input.push_str(&format!(
            "{},301,399,{},{},{},{},PRESERVE_ORIGINAL,RECTANGULAR,false,VECTOR,1,0,0,X,true,true\n",
            p[0], p[2], p[3], p[1], p[1]
        ));
        expected.push(
            p[4..7]
                .iter()
                .map(|s| s.parse::<f64>().unwrap())
                .collect::<Vec<_>>(),
        );
    }
    let output = run(
        env!("CARGO_BIN_EXE_wgc_geometry"),
        &[
            "pointing_full",
            "target/kernels/pck00011.tpc",
            "tests/golden/dynamic_ephemeris.tf",
        ],
        &input,
    );
    let rows: Vec<_> = output.lines().collect();
    assert_eq!(rows.len(), 108);
    assert_eq!(rows.len(), expected.len());
    for (i, (line, reference)) in rows.iter().zip(expected).enumerate() {
        let p: Vec<_> = line.split(',').collect();
        assert_eq!(p.len(), 9, "query {i}: {line}");
        for j in 0..3 {
            let actual = p[6 + j].parse::<f64>().unwrap();
            assert_eq!(
                actual.to_bits(),
                reference[j].to_bits(),
                "query {i} column {j}"
            );
        }
    }
}
#[test]
fn native_dynamic_coordinate_finder_cli_context_exact() {
    let query="dynamic,GF_COORDINATE_SEARCH,301,399,0,10,EPHEM_TWO_0,IAU_301,Near point: ellipsoid,RECTANGULAR,X,PHASE,>,400000,0,NONE,0:2000000,10800,0,false,NONE,0,SECONDS,false,0,SECONDS,SECONDS,1,0,0,POINT,POINT,ANY\n";
    let output = run(
        env!("CARGO_BIN_EXE_wgc_gf"),
        &[
            "target/kernels/pck00011.tpc",
            "tests/golden/dynamic_ephemeris.tf",
        ],
        query,
    );
    let rows: Vec<_> = output.lines().collect();
    let references: Vec<Vec<f64>> = include_str!("golden/cli_ephemeris_context_gf.csv")
        .lines()
        .skip(1)
        .map(|line| line.split(',').map(|s| s.parse().unwrap()).collect())
        .collect();
    assert_eq!(rows.len(), references.len() + 2, "{output}");
    assert_eq!(rows[0], format!("BEGIN,dynamic,{}", references.len()));
    assert_eq!(rows.last().unwrap(), &"END,dynamic");
    assert!(!references.is_empty());
    for (i, reference) in references.iter().enumerate() {
        let p: Vec<_> = rows[i + 1].split(',').collect();
        assert_eq!(&p[..3], &["ROW", "dynamic", &i.to_string()]);
        for j in 0..3 {
            let actual = p[3 + j].parse::<f64>().unwrap();
            assert_eq!(
                actual.to_bits(),
                reference[j].to_bits(),
                "row {i} column {j}"
            );
        }
    }
}
