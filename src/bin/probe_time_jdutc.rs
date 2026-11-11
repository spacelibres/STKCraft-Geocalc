//! Diagnostic only: public time-context UTC labels versus native JDUTC.
use stkcraft_time::{
    contract::{NaifEt, TimeContext},
    Calendar,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string("target/time-jdutc-bridge.csv")?;
    let lsk =
        stkcraft_time::lsk::parse_lsk(&std::fs::read_to_string("target/kernels/naif0012.tls")?)?;
    let context = TimeContext::new(&lsk);
    let mut failures = Vec::new();
    let mut count = 0;
    for line in text.lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let et = f[0].parse()?;
        let expected = f[1].parse::<f64>()?;
        let utc = context.et_to_utc(NaifEt::from_seconds(et)?, Calendar::Gregorian)?;
        let actual = stkcraft_time::calendar::calendar_to_jd(
            utc.year,
            utc.month,
            utc.day,
            utc.hour,
            utc.minute,
            utc.second,
            Calendar::Gregorian,
        )?;
        if actual.to_bits() != expected.to_bits() {
            failures.push(format!("{et},{expected},{actual}"));
        }
        count += 1;
    }
    std::fs::write(
        "target/time-jdutc-bridge-differences.csv",
        failures.join("\n"),
    )?;
    println!("{} mismatches/{count} native JDUTC values", failures.len());
    if !failures.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
