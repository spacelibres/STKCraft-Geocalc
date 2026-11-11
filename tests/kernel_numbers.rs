use stkcraft_geocalc::Pck;
#[test]
fn text_numbers_equal_naif_prsdp() {
    let mut failures = Vec::new();
    for line in include_str!("golden/kernel_numbers.csv").lines().skip(1) {
        let (text, expected) = line.split_once(',').unwrap();
        let expected: f64 = expected.parse().unwrap();
        let pck = Pck::from_text(&format!(
            "KPL/PCK\n\\begindata\nBODY123_GM = ( {text} )\n\\begintext\n"
        ));
        match pck {
            Ok(pck) if pck.gm(123).unwrap() == expected => {}
            actual => failures.push(format!("{text} expected {expected:?}, got {actual:?}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} parser mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
