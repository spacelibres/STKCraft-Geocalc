//! Fixed sibling-executable bridge; numerical work remains in existing CLIs.
use super::catalog::{self};
use serde_json::{json, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn text<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    match value.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(Some)
            .ok_or_else(|| format!("{key} must be a string")),
    }
}
fn path(value: &str) -> Result<&str, String> {
    if value.is_empty() || value.contains([',', '\r', '\n', '\0', '|']) {
        Err("kernel paths must be non-empty and cannot contain comma, pipe, newline or NUL".into())
    } else {
        Ok(value)
    }
}
fn paths(value: &Value, key: &str) -> Result<Vec<String>, String> {
    match value.get(key) {
        None => Ok(vec![]),
        Some(v) => v
            .as_array()
            .ok_or_else(|| format!("kernels.{key} must be an array"))?
            .iter()
            .map(|p| Ok(path(p.as_str().ok_or("kernel path must be a string")?)?.to_string()))
            .collect(),
    }
}
fn validate_rows(rows: &[String]) -> Result<(), String> {
    if rows.is_empty() || rows.len() > 1 {
        return Err("provide exactly one CSV query per call".into());
    }
    for row in rows {
        if row.len() > 65536 || row.contains(['\n', '\r', '\0']) {
            return Err("query must be one bounded CSV row".into());
        }
        for field in row.split(',').flat_map(|s| s.split([';', ':'])) {
            let f = field.trim();
            if matches!(
                f.to_ascii_lowercase().as_str(),
                "nan" | "inf" | "+inf" | "-inf" | "infinity" | "+infinity" | "-infinity"
            ) || f.parse::<f64>().is_ok_and(|n| !n.is_finite())
            {
                return Err("non-finite query numbers are not supported".into());
            }
        }
    }
    Ok(())
}
fn csv_args(args: &Value) -> Result<(String, Vec<String>, Vec<String>), String> {
    let object = args.as_object().ok_or("arguments must be an object")?;
    if object
        .keys()
        .any(|k| !["kernels", "calculator", "queries"].contains(&k.as_str()))
    {
        return Err("unknown calculate_csv argument".into());
    }
    let mode = text(args, "calculator")?.ok_or("calculator is required")?;
    if !catalog::raw_modes().iter().any(|s| s == mode) {
        return Err("unsupported calculator".into());
    }
    let rows = args
        .get("queries")
        .and_then(Value::as_array)
        .ok_or("queries must be an array")?
        .iter()
        .map(|s| {
            s.as_str()
                .map(str::to_string)
                .ok_or("queries must contain strings".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let columns = catalog::specifications()
        .into_iter()
        .find(|s| s.mode == mode)
        .map(|s| s.columns)
        .unwrap_or_default();
    Ok((mode.into(), rows, columns))
}
pub fn prepare(name: &str, args: &Value) -> Result<(String, Vec<String>, Vec<String>), String> {
    let result = if name == "calculate_csv" {
        csv_args(args)?
    } else {
        let spec = catalog::specifications()
            .into_iter()
            .find(|s| s.name == name)
            .ok_or("unknown tool")?;
        let row = catalog::typed_query(&spec, args)?;
        (spec.mode, vec![row], spec.columns)
    };
    validate_rows(&result.1)?;
    build_command(&result.0, args)?;
    Ok(result)
}
fn build_command(mode: &str, args: &Value) -> Result<(Command, &'static str), String> {
    let kernels = args
        .get("kernels")
        .filter(|v| v.is_object())
        .ok_or("kernels must be an object")?;
    let allowed = [
        "spk",
        "pck",
        "fk",
        "ck",
        "lsk",
        "sclk",
        "ik",
        "instrument_id",
    ];
    if kernels
        .as_object()
        .unwrap()
        .keys()
        .any(|k| !allowed.contains(&k.as_str()))
    {
        return Err("unknown kernel context field".into());
    }
    let spk = paths(kernels, "spk")?;
    if spk.is_empty() {
        return Err("kernels.spk must contain at least one SPK path".into());
    }
    let pck = paths(kernels, "pck")?;
    let fk = paths(kernels, "fk")?;
    let ck = paths(kernels, "ck")?;
    let joined = |paths: &[String]| {
        if paths.is_empty() {
            "-".to_string()
        } else {
            paths.join(",")
        }
    };
    let is_gf = catalog::GF_FAMILIES.contains(&mode);
    let backend = if is_gf { "wgc_gf" } else { "wgc_geometry" };
    let directory = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("server executable has no parent directory")?
        .to_path_buf();
    let executable = directory.join(format!("{backend}{}", std::env::consts::EXE_SUFFIX));
    let mut command = Command::new(executable);
    if !is_gf {
        command.arg(mode);
    }
    match mode {
        "frame_rep" => {
            command.arg(joined(&fk)).arg(joined(&pck));
        }
        "frame" | "pointing" => {
            command.arg(joined(&fk));
        }
        _ => {
            command.arg(joined(&pck)).arg(joined(&fk));
        }
    }
    let instrument = kernels
        .get("instrument_id")
        .map(|v| v.as_i64().ok_or("instrument_id must be an integer"))
        .transpose()?;
    if let Some(ik) = text(kernels, "ik")? {
        let id = instrument.ok_or("IK requires instrument_id")?;
        if id < i32::MIN as i64 || id > i32::MAX as i64 {
            return Err("instrument_id exceeds NAIF integer range".into());
        }
        command.arg(path(ik)?).arg(id.to_string());
    } else if instrument.is_some() {
        return Err("instrument_id requires an IK path".into());
    }
    for key in [
        "STKCRAFT_SPK",
        "STKCRAFT_LSK",
        "STKCRAFT_CK",
        "STKCRAFT_SCLK",
    ] {
        command.env_remove(key);
    }
    command.env("STKCRAFT_SPK", spk.join(","));
    if !ck.is_empty() {
        command.env("STKCRAFT_CK", ck.join(","));
    }
    let lsk = text(kernels, "lsk")?;
    if let Some(lsk) = lsk {
        command.env("STKCRAFT_LSK", path(lsk)?);
    }
    if let Some(sclk) = kernels.get("sclk") {
        let clocks = sclk.as_array().ok_or("sclk must be an array")?;
        if !clocks.is_empty() && lsk.is_none() {
            return Err("SCLK context requires an explicit LSK path".into());
        }
        let mut packed = Vec::new();
        for entry in clocks {
            let object = entry.as_object().ok_or("SCLK entry must be an object")?;
            if object.len() != 2 {
                return Err("SCLK entries require spacecraft_id and path only".into());
            }
            let id = entry
                .get("spacecraft_id")
                .and_then(Value::as_i64)
                .filter(|i| *i >= i32::MIN as i64 && *i <= i32::MAX as i64)
                .ok_or("invalid spacecraft_id")?;
            let p = text(entry, "path")?.ok_or("missing SCLK path")?;
            packed.push(format!("{id}|{}", path(p)?));
        }
        if !packed.is_empty() {
            command.env("STKCRAFT_SCLK", packed.join(","));
        }
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok((command, backend))
}
pub fn call(name: &str, args: &Value) -> Result<Value, String> {
    let (mode, rows, columns) = prepare(name, args)?;
    let (mut command, backend) = build_command(&mode, args)?;
    let mut child = command.spawn().map_err(|e| {
        format!("cannot start sibling {backend}: {e}; install all three binaries together")
    })?;
    let write_result = (|| -> std::io::Result<()> {
        let mut stdin = child.stdin.take().unwrap();
        for row in &rows {
            writeln!(stdin, "{row}")?;
        }
        Ok(())
    })();
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let raw = String::from_utf8(output.stdout).map_err(|_| "backend output is not UTF-8")?;
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "{backend} failed: {}{}",
            diagnostic.trim(),
            raw.trim()
        ));
    }
    write_result.map_err(|e| format!("cannot send query: {e}"))?;
    if raw.lines().any(|l| l.starts_with("ERROR,")) {
        return Err(raw.trim().into());
    }
    table(&mode, backend, columns, &raw)
}
fn cell(s: &str) -> Result<Value, String> {
    if s.is_empty() {
        return Ok(Value::Null);
    }
    if let Ok(b) = s.parse::<bool>() {
        return Ok(json!(b));
    }
    if let Ok(n) = s.parse::<f64>() {
        if !n.is_finite() {
            return Err(format!("backend emitted non-finite value {s}"));
        }
        return Ok(json!(n));
    }
    Ok(json!(s))
}
fn table(mode: &str, backend: &str, mut columns: Vec<String>, raw: &str) -> Result<Value, String> {
    let gf = backend == "wgc_gf";
    let mut rows = Vec::new();
    let mut advertised = None;
    let mut ended = false;
    for line in raw.lines() {
        let fields: Vec<_> = line.split(',').collect();
        if fields.first() == Some(&"COLS") {
            columns = fields[1..].iter().map(|s| s.to_string()).collect();
            continue;
        }
        let values = if gf {
            match fields.first().copied() {
                Some("BEGIN") if fields.len() == 3 => {
                    advertised = Some(fields[2].parse::<usize>().map_err(|_| "invalid GF count")?);
                    continue;
                }
                Some("END") => {
                    ended = true;
                    continue;
                }
                Some("ROW") if fields.len() == 6 => &fields[3..],
                _ => return Err("invalid GF backend output".into()),
            }
        } else if fields.first() == Some(&"ROW") {
            &fields[1..]
        } else {
            &fields[..]
        };
        rows.push(
            values
                .iter()
                .map(|s| cell(s))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    if gf && (advertised != Some(rows.len()) || !ended) {
        return Err("incomplete GF output".into());
    }
    if rows.is_empty() && !gf {
        return Err("backend returned no rows".into());
    }
    let width = rows.first().map(Vec::len).unwrap_or(columns.len());
    if rows.iter().any(|r| r.len() != width) {
        return Err("backend table rows have inconsistent widths".into());
    }
    if columns.len() != width {
        columns = (1..=width).map(|i| format!("field_{i}")).collect();
    }
    Ok(json!({"calculator":mode,"backend":backend,"columns":columns,"rows":rows,"rawCsv":raw}))
}
