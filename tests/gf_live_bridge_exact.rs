//! Actual DE442 and Cassini2012 GF results, generated independently by N0067.
use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn all_official_gf_cli_families_match_native_endpoint_bits() {
    let expected: BTreeMap<String, Vec<[f64; 3]>> = {
        let mut m: BTreeMap<String, Vec<[f64; 3]>> = BTreeMap::new();
        for line in include_str!("golden/gf_live_bridge_windows.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            assert_eq!(f.len(), 4);
            m.entry(f[0].into()).or_default().push([
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
                f[3].parse().unwrap(),
            ]);
        }
        m
    };
    assert_eq!(expected.len(), 12);
    assert_eq!(expected.values().map(Vec::len).sum::<usize>(), 18);
    let fks = include_str!("golden/wgc_kernel_fk_order.txt")
        .lines()
        .collect::<Vec<_>>()
        .join(",");
    let queries: Vec<_> = include_str!("golden/gf_live_bridge_queries.csv")
        .lines()
        .collect();
    assert_eq!(queries.len(), 12);
    let mut checked = 0;
    for mission in [false, true] {
        let selected: Vec<_> = queries
            .iter()
            .copied()
            .filter(|line| line.contains("_FOV_SEARCH") == mission)
            .collect();
        assert!(!selected.is_empty());
        let mut command = Command::new(env!("CARGO_BIN_EXE_wgc_gf"));
        command
            .env_remove("STKCRAFT_CK")
            .env_remove("STKCRAFT_SCLK")
            .env_remove("STKCRAFT_LSK");
        if mission {
            command.args(["target/kernels/cassini/pck00010.tpc,target/kernels/cassini/enceladus_ssd_250728_v2.tpc",&(fks.clone()+",target/kernels/cassini/cas_v44.tf"),"target/kernels/cassini/cas_iss_v10.ti","-82360"])
    .env("STKCRAFT_SPK","target/kernels/cassini/200128RU_SCPSE_12280_12328.bsp,target/kernels/de442.bsp")
    .env("STKCRAFT_CK","target/kernels/cassini/12289_12294ra.bc").env("STKCRAFT_SCLK","-82|target/kernels/cassini/cas00172.tsc").env("STKCRAFT_LSK","target/kernels/naif0012.tls");
        } else {
            command.args(["target/kernels/pck00011.tpc,target/kernels/mars_iau2000_v1.tpc,target/kernels/gm_de431.tpc,target/kernels/moon_pa_de440_200625.bpc",&fks]).env("STKCRAFT_SPK","target/kernels/de442.bsp");
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all((selected.join("\n") + "\n").as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let output = String::from_utf8(result.stdout).unwrap();
        let mut lines = output.lines();
        for query in selected {
            let id = query.split(',').next().unwrap();
            let reference = &expected[id];
            let begin: Vec<_> = lines.next().unwrap().split(',').collect();
            assert_eq!(begin, ["BEGIN", id, &reference.len().to_string()]);
            for (i, values) in reference.iter().enumerate() {
                let row: Vec<_> = lines.next().unwrap().split(',').collect();
                assert_eq!(row.len(), 6);
                assert_eq!(&row[..3], ["ROW", id, &i.to_string()]);
                for j in 0..3 {
                    let actual: f64 = row[3 + j].parse().unwrap();
                    assert!(actual.is_finite());
                    assert_eq!(actual.to_bits(), values[j].to_bits(), "{id} row{i} col{j}");
                    checked += 1;
                }
            }
            assert_eq!(lines.next().unwrap(), format!("END,{id}"));
        }
        assert!(lines.next().is_none(), "unexpected extra GF output");
    }
    assert_eq!(checked, 54);
}
