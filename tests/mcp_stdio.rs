#![cfg(feature = "mcp")]
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
fn run(lines: &[String]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_stkcraft-geocalc-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for line in lines {
        writeln!(input, "{line}").unwrap();
    }
    drop(input);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn init() -> String {
    json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"native-test","version":"1"}}}).to_string()
}
#[test]
fn mcp_stdio_initialize_discovery_and_notification_silence() {
    let response = run(&[
        init(),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string(),
        json!({"jsonrpc":"2.0","id":3,"method":"ping"}).to_string(),
    ]);
    assert_eq!(response.len(), 3);
    assert_eq!(response[0]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(response[1]["result"]["tools"].as_array().unwrap().len(), 25);
    assert!(response[2].get("result").is_some());
}
#[test]
fn mcp_stdio_rejects_bad_requests_and_unknown_tools() {
    let response=run(&["not-json".into(),init(),json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"run_shell","arguments":{}}}).to_string()]);
    assert_eq!(response[0]["error"]["code"], -32700);
    assert_eq!(response[2]["error"]["code"], -32602);
}
fn kernels() -> Value {
    json!({"spk":[format!("{}/target/kernels/de442s.bsp",env!("CARGO_MANIFEST_DIR"))],"pck":[format!("{}/target/kernels/pck00011.tpc",env!("CARGO_MANIFEST_DIR"))]})
}
#[test]
fn mcp_state_and_gf_preserve_native_backend_bits() {
    let state=json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"state_vector","arguments":{"kernels":kernels(),"target":301,"observer":399,"epoch_et":0.0,"correction":"CN+S"}}}).to_string();
    let gf=json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"gf_distance_search","arguments":{"kernels":kernels(),"target":301,"observer":399,"confinement":[{"start":0.0,"end":4000000.0}],"step_seconds":3600.0,"relation":"RANGE","reference":380000.0,"upper":400000.0}}}).to_string();
    let result = run(&[
        init(),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
        state,
        gf,
    ]);
    assert_eq!(result[1]["result"]["isError"], false, "{}", result[1]);
    assert_eq!(result[2]["result"]["isError"], false, "{}", result[2]);
    let dir = std::path::Path::new(env!("CARGO_BIN_EXE_wgc_geometry"))
        .parent()
        .unwrap();
    let mut command =
        Command::new(dir.join(format!("wgc_geometry{}", std::env::consts::EXE_SUFFIX)));
    command
        .arg("state")
        .env("STKCRAFT_SPK", kernels()["spk"][0].as_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = command.spawn().unwrap();
    writeln!(child.stdin.take().unwrap(), "301,399,0.0,CN+S").unwrap();
    let out = child.wait_with_output().unwrap();
    let raw = String::from_utf8(out.stdout).unwrap();
    assert_eq!(result[1]["result"]["structuredContent"]["rawCsv"], raw);
    let row = result[1]["result"]["structuredContent"]["rows"][0]
        .as_array()
        .unwrap();
    let tokens: Vec<_> = raw.trim().split(',').collect();
    for i in 4..tokens.len() {
        assert_eq!(
            row[i].as_f64().unwrap().to_bits(),
            tokens[i].parse::<f64>().unwrap().to_bits()
        );
    }
    let expected = include_str!("golden/gf_constraints_precision.csv")
        .lines()
        .nth(1)
        .unwrap()
        .split(',')
        .next_back()
        .unwrap();
    let windows: Vec<_> = expected
        .split(';')
        .map(|p| {
            p.split(':')
                .map(|v| v.parse::<f64>().unwrap())
                .collect::<Vec<_>>()
        })
        .collect();
    let rows = result[2]["result"]["structuredContent"]["rows"]
        .as_array()
        .unwrap();
    assert_eq!(rows.len(), windows.len());
    for (a, e) in rows.iter().zip(windows) {
        assert_eq!(a[0].as_f64().unwrap().to_bits(), e[0].to_bits());
        assert_eq!(a[1].as_f64().unwrap().to_bits(), e[1].to_bits());
    }
}
#[test]
fn mcp_modern_discovery_and_finite_validation() {
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});
    let result=run(&[json!({"jsonrpc":"2.0","id":"discover","method":"server/discover","params":{"_meta":meta}}).to_string(),json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"_meta":meta,"name":"state_vector","arguments":{"kernels":kernels(),"target":301,"observer":399,"epoch_et":"NaN"}}}).to_string(),json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"1900-01-01"}}}).to_string()]);
    assert_eq!(result[0]["result"]["resultType"], "complete");
    assert_eq!(result[0]["result"]["supportedVersions"][0], "2026-07-28");
    assert_eq!(result[1]["error"]["code"], -32602);
    assert_eq!(result[2]["error"]["code"], -32022);
}
#[test]
fn mcp_csv_adapter_validation_and_exact_output() {
    let requests=vec![init(),json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"calculate_csv","arguments":{"kernels":kernels(),"calculator":"state","queries":["301,399,0.0,NONE"]}}}).to_string(),json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"calculate_csv","arguments":{"kernels":kernels(),"calculator":"state","queries":["301,399,0.0,NONE","301,399,1.0,NONE"]}}}).to_string(),json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"state_vector","arguments":{"kernels":{"spk":[],"command":"cmd.exe"},"target":301,"observer":399,"epoch_et":0.0}}}).to_string(),json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"calculate_csv","arguments":{"kernels":kernels(),"calculator":"state","queries":["301,399,NaN,NONE"]}}}).to_string()];
    let result = run(&requests);
    assert_eq!(result[1]["result"]["isError"], false);
    assert_eq!(
        result[1]["result"]["structuredContent"]["rows"][0]
            .as_array()
            .unwrap()
            .len(),
        14
    );
    for r in &result[2..] {
        assert_eq!(r["error"]["code"], -32602, "{r}");
    }
}
#[test]
fn mcp_protocol_recovery_and_backend_error_isolation() {
    let result=run(&["[]".into(),json!({"jsonrpc":"2.0","id":null,"method":"ping"}).to_string(),init(),json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"state_vector","arguments":{"kernels":{"spk":["missing-kernel.bsp"]},"target":301,"observer":399,"epoch_et":0.0}}}).to_string(),json!({"jsonrpc":"2.0","id":3,"method":"ping"}).to_string(),json!({"jsonrpc":"2.0","id":4,"method":"resources/list"}).to_string(),json!({"jsonrpc":"2.0","id":5,"method":"resources/read","params":{"uri":"geocalc://manual"}}).to_string()]);
    assert_eq!(result.len(), 7);
    assert_eq!(result[0]["error"]["code"], -32600);
    assert_eq!(result[1]["error"]["code"], -32600);
    assert_eq!(result[3]["result"]["isError"], true);
    assert!(result[4].get("result").is_some());
    assert_eq!(
        result[5]["result"]["resources"][0]["uri"],
        "geocalc://manual"
    );
    assert!(result[6]["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("MCP Server"));
}
#[test]
fn mcp_ray_tools_require_an_explicit_direction() {
    let result=run(&[init(),json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"gf_surface_intercept_point_search","arguments":{"kernels":kernels(),"target":301,"observer":399,"confinement":[{"start":0.0,"end":1000.0}],"step_seconds":100.0}}}).to_string()]);
    assert_eq!(result[1]["error"]["code"], -32602, "{}", result[1]);
}
#[test]
fn mcp_preserves_custom_frame_names_containing_locus_words() {
    let path = std::env::temp_dir().join(format!("geocalc-mcp-frame-{}.tf", std::process::id()));
    std::fs::write(&path,"KPL/FK\n\\begindata\nFRAME_TANGENT_POINT=1999998\nFRAME_1999998_NAME='TANGENT_POINT'\nFRAME_1999998_CLASS=4\nFRAME_1999998_CLASS_ID=1999998\nFRAME_1999998_CENTER=0\nTKFRAME_1999998_RELATIVE='J2000'\nTKFRAME_1999998_SPEC='MATRIX'\nTKFRAME_1999998_MATRIX=(1 0 0 0 1 0 0 0 1)\n\\begintext").unwrap();
    let mut context = kernels();
    context["fk"] = json!([path]);
    let result=run(&[init(),json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"frame_transformation","arguments":{"kernels":context,"frame1":"TANGENT_POINT","frame2":"J2000","epoch_et":0.0,"orientation":"MATRIX_ALL_ONE_ROW"}}}).to_string()]);
    std::fs::remove_file(path).unwrap();
    assert_eq!(result[1]["result"]["isError"], false, "{}", result[1]);
    let row = result[1]["result"]["structuredContent"]["rows"][0]
        .as_array()
        .unwrap();
    assert_eq!(row.len(), 9);
    for (i, value) in row.iter().enumerate() {
        assert_eq!(value.as_f64().unwrap(), if i % 4 == 0 { 1.0 } else { 0.0 });
    }
}
#[test]
fn mcp_frame_tool_default_orientation_is_supported() {
    let result=run(&[init(),json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"frame_transformation","arguments":{"kernels":kernels(),"frame1":"J2000","frame2":"ECLIPJ2000","epoch_et":0.0}}}).to_string()]);
    assert_eq!(result[1]["result"]["isError"], false, "{}", result[1]);
}

#[test]
fn mcp_location_rejects_invalid_values_and_recovers() {
    let base = "OBJECT,301,0,J2000,0,0,0,0,0,0,0,OBJECT,399,0,J2000,0,0,0,0,0,0,0,J2000";
    let inputs = [
        format!("{base},invalid,CENTER,NONE,RECTANGULAR,1,1,0"),
        format!("{base},SPICE_FRAME,invalid,NONE,RECTANGULAR,1,1,0"),
        format!("{base},SPICE_FRAME,CENTER,NONE,RECTANGULAR,invalid,1,0"),
        format!("{},SPICE_FRAME,CENTER,NONE,RECTANGULAR,1,1,0", base.replacen("OBJECT", "invalid", 1)),
        "FIXED_POINT,0,301,J2000,1,0,0,1,0,0,10,OBJECT,399,0,J2000,0,0,0,0,0,0,0,J2000,SPICE_FRAME,CENTER,NONE,RECTANGULAR,1,1,100".to_string(),
        format!("{base},SPICE_FRAME,CENTER,NONE,RECTANGULAR,1,1,0"),
    ];
    let mut requests = vec![
        init(),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
    ];
    for (index, input) in inputs.iter().enumerate() {
        requests.push(json!({"jsonrpc":"2.0","id":index+2,"method":"tools/call","params":{"name":"calculate_csv","arguments":{"kernels":kernels(),"calculator":"location","queries":[input]}}}).to_string());
    }
    let result = run(&requests);
    assert_eq!(result.len(), 7);
    let expected = [
        "ERROR,unknown frame type invalid",
        "ERROR,unknown frame locus invalid",
        "ERROR,azccw must be 0 or 1",
        "ERROR,unknown target location type invalid",
        "ERROR,fixed target point must have zero velocity",
    ];
    for (response, message) in result[1..6].iter().zip(expected) {
        assert_eq!(response["result"]["isError"], true, "{response}");
        assert_eq!(
            response["result"]["content"][0]["text"], message,
            "{response}"
        );
    }
    assert_eq!(result[6]["result"]["isError"], false);
    assert_eq!(
        result[6]["result"]["structuredContent"]["rows"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn mcp_location_preserves_native_moving_point_bits() {
    let cases: Vec<_> = include_str!("../docs/review-2026-11-12/location-cli-native/matrix.csv")
        .lines()
        .skip(1)
        .collect();
    assert_eq!(cases.len(), 18);
    let mut context = kernels();
    context["spk"] = json!([format!(
        "{}/target/kernels/de440s.bsp",
        env!("CARGO_MANIFEST_DIR")
    )]);
    let mut requests = vec![
        init(),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
    ];
    for (index, case) in cases.iter().enumerate() {
        let f: Vec<_> = case.split(',').collect();
        assert_eq!(f.len(), 9);
        let point_target = "CONSTANT_VELOCITY_POINT,0,301,J2000,1,0,0,1,0,0,10";
        let object_target = "OBJECT,301,0,J2000,0,0,0,0,0,0,0";
        let point_observer = "CONSTANT_VELOCITY_POINT,0,399,J2000,1,0,0,1,0,0,10";
        let object_observer = "OBJECT,399,0,J2000,0,0,0,0,0,0,0";
        let (target, observer) = match f[0] {
            "target" => (point_target, object_observer),
            "observer" => (object_target, point_observer),
            other => panic!("unexpected endpoint {other}"),
        };
        let query = format!(
            "{target},{observer},J2000,SPICE_FRAME,CENTER,{},RECTANGULAR,1,1,100",
            f[1]
        );
        requests.push(json!({"jsonrpc":"2.0","id":index+2,"method":"tools/call","params":{"name":"calculate_csv","arguments":{"kernels":context,"calculator":"location","queries":[query]}}}).to_string());
    }
    let responses = run(&requests);
    assert_eq!(responses.len(), cases.len() + 1);
    for (case, response) in cases.iter().zip(responses.iter().skip(1)) {
        let f: Vec<_> = case.split(',').collect();
        assert_eq!(response["result"]["isError"], false, "{response}");
        let rows = response["result"]["structuredContent"]["rows"]
            .as_array()
            .unwrap();
        assert_eq!(rows.len(), 1, "{response}");
        let row = rows[0].as_array().unwrap();
        assert_eq!(row.len(), 10);
        let actual: Vec<_> = row[2..8]
            .iter()
            .chain(std::iter::once(&row[9]))
            .map(|value| value.as_f64().unwrap().to_bits())
            .collect();
        let expected: Vec<_> = f[2..]
            .iter()
            .map(|value| u64::from_str_radix(value, 16).unwrap())
            .collect();
        assert_eq!(actual, expected, "{} {}", f[0], f[1]);
    }
}

#[test]
fn mcp_location_preserves_native_frame_matrix_bits() {
    let matrices = [
        (
            include_str!("../docs/review-2026-11-12/location-cli-native/rotating.csv"),
            true,
            108,
        ),
        (
            include_str!("../docs/review-2026-11-12/location-cli-native/fixed.csv"),
            false,
            162,
        ),
    ];
    let mut context = kernels();
    context["spk"] = json!([format!(
        "{}/target/kernels/de440s.bsp",
        env!("CARGO_MANIFEST_DIR")
    )]);
    for (matrix, moving, expected_count) in matrices {
        let cases: Vec<_> = matrix.lines().skip(1).collect();
        assert_eq!(cases.len(), expected_count);
        for batch in cases.chunks(9) {
            let mut requests = vec![
                init(),
                json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
            ];
            for (index, case) in batch.iter().enumerate() {
                let f: Vec<_> = case.split(',').collect();
                assert_eq!(f.len(), 11);
                let target_point = if moving {
                    "CONSTANT_VELOCITY_POINT,0,301,J2000,1,0,0,1,0,0,10"
                } else {
                    "FIXED_POINT,0,301,J2000,1,0,0,0,0,0,10"
                };
                let observer_point = if moving {
                    "CONSTANT_VELOCITY_POINT,0,399,J2000,1,0,0,1,0,0,10"
                } else {
                    "FIXED_POINT,0,399,J2000,1,0,0,0,0,0,10"
                };
                let object_target = "OBJECT,301,0,J2000,0,0,0,0,0,0,0";
                let object_observer = "OBJECT,399,0,J2000,0,0,0,0,0,0,0";
                let (target, observer) = match f[2] {
                    "target" => (target_point, object_observer),
                    "observer" => (object_target, observer_point),
                    other => panic!("unexpected endpoint {other}"),
                };
                let query = format!(
                    "{target},{observer},{},SPICE_FRAME,{},{},RECTANGULAR,1,1,100",
                    f[0], f[1], f[3]
                );
                requests.push(json!({"jsonrpc":"2.0","id":index+2,"method":"tools/call","params":{"name":"calculate_csv","arguments":{"kernels":context,"calculator":"location","queries":[query]}}}).to_string());
            }
            let responses = run(&requests);
            assert_eq!(responses.len(), batch.len() + 1);
            for (case, response) in batch.iter().zip(responses.iter().skip(1)) {
                let f: Vec<_> = case.split(',').collect();
                assert_eq!(response["result"]["isError"], false, "{response}");
                let rows = response["result"]["structuredContent"]["rows"]
                    .as_array()
                    .unwrap();
                assert_eq!(rows.len(), 1);
                let row = rows[0].as_array().unwrap();
                assert_eq!(row.len(), 10);
                let actual: Vec<_> = row[2..8]
                    .iter()
                    .chain(std::iter::once(&row[9]))
                    .map(|value| value.as_f64().unwrap().to_bits())
                    .collect();
                let expected: Vec<_> = f[4..]
                    .iter()
                    .map(|value| u64::from_str_radix(value, 16).unwrap())
                    .collect();
                assert_eq!(actual, expected, "{} {} {} {}", f[0], f[1], f[2], f[3]);
            }
        }
    }
}
