//! A narrow JSON bridge to the checked STKCraft-time NAIF ET contract.
use serde_json::{json, Value};
use std::io::{self, Read};
use stkcraft_time::{
    contract::{NaifEt, TimeContext},
    Calendar, LeapSecondTable, UtcDateTime,
};

fn integer(value: &Value, field: &str) -> Result<i64, String> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{field} must be an integer"))
}
fn run() -> Result<Value, String> {
    let path = std::env::args().nth(1).ok_or("LSK path is required")?;
    let contents = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lsk = LeapSecondTable::from_lsk(&contents).map_err(|e| e.to_string())?;
    let context = TimeContext::new(&lsk);
    let mut input = String::new();
    io::stdin()
        .take(65536)
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    let request: Value = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    let calendar = match request
        .get("calendar")
        .and_then(Value::as_str)
        .unwrap_or("gregorian")
    {
        "gregorian" => Calendar::Gregorian,
        "julian" => Calendar::Julian,
        "mixed" => Calendar::Mixed,
        _ => return Err("unsupported calendar".into()),
    };
    match request.get("direction").and_then(Value::as_str) {
        Some("utc_to_et") => {
            let utc = request.get("utc").ok_or("utc is required")?;
            let year = i32::try_from(integer(utc, "year")?).map_err(|_| "invalid year")?;
            let field = |name: &str| -> Result<u8, String> {
                u8::try_from(integer(utc, name)?).map_err(|_| format!("invalid {name}"))
            };
            let second = utc
                .get("second")
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite())
                .ok_or("second must be finite")?;
            let date = UtcDateTime::new(
                year,
                field("month")?,
                field("day")?,
                field("hour")?,
                field("minute")?,
                second,
            );
            let et = context
                .utc_to_et(&date, calendar)
                .map_err(|e| e.to_string())?;
            Ok(json!({"et":et.seconds(), "time_system":"NAIF ET/TDB", "lsk":"configured"}))
        }
        Some("et_to_utc") => {
            let seconds = request
                .get("et")
                .and_then(Value::as_f64)
                .ok_or("et must be a number")?;
            let et = NaifEt::from_seconds(seconds).map_err(|e| e.to_string())?;
            let date = context.et_to_utc(et, calendar).map_err(|e| e.to_string())?;
            Ok(
                json!({"utc":{"year":date.year,"month":date.month,"day":date.day,"hour":date.hour,"minute":date.minute,"second":date.second},"time_system":"UTC"}),
            )
        }
        _ => Err("unsupported direction".into()),
    }
}
fn main() {
    match run() {
        Ok(value) => println!("{value}"),
        Err(error) => {
            println!("{}", json!({"error":error}));
            std::process::exit(1);
        }
    }
}
