//! Native PCK replacement, failed-load state and orientation regressions.
use stkcraft_geocalc::{Frames, GeoError, Pck};

fn native_result(actual: Result<Vec<f64>, GeoError>) -> String {
    match actual {
        Ok(values) => format!(
            "N|{}|{}",
            values.len(),
            values
                .iter()
                .map(|x| format!("{:016x}", x.to_bits()))
                .collect::<Vec<_>>()
                .join("|")
        ),
        Err(error) => {
            let text = error.to_string();
            if let Some(start) = text.find("SPICE(") {
                if let Some(end) = text[start..].find(')') {
                    return format!("E|{}", &text[start..=start + end]);
                }
            }
            format!("UNMAPPED_ERROR|{text}")
        }
    }
}

#[test]
fn native_pck_replacements_and_failed_updates_exact() {
    let frames = Frames::new();
    let mut pck = Pck::default();
    let mut failures = Vec::new();
    let (mut resets, mut loads, mut constants, mut transforms) = (0, 0, 0, 0);
    for line in include_str!("golden/pck_pool_updates.tsv").lines().skip(1) {
        let fields: Vec<_> = line.split('\t').collect();
        let (case, command, native) = (fields[0], fields[1], fields[2]);
        let args: Vec<_> = command.split('|').collect();
        let actual = match args[0] {
            "C" => {
                pck = Pck::default();
                resets += 1;
                "OK".into()
            }
            "L" => {
                loads += 1;
                let text =
                    std::fs::read_to_string(format!("tests/golden/pck_pool_updates/{}", args[1]))
                        .unwrap();
                match pck.furnsh(&text) {
                    Ok(()) => "OK".into(),
                    Err(error) => native_result(Err(error)),
                }
            }
            "B" if matches!(args[2], "RADII" | "GM") => {
                constants += 1;
                let actual = if args[2] == "RADII" {
                    pck.radii(499).map(|x| x.to_vec())
                } else {
                    pck.gm(499).map(|x| vec![x])
                };
                if native.starts_with("E|") {
                    // These existing convenience APIs return Option, not a native error.
                    // Assert absence only; the reference error is retained, not claimed matched.
                    if actual.is_some() {
                        failures.push(format!(
                            "{case} {command}: stale {actual:?}; native {native}"
                        ));
                    }
                    continue;
                }
                match actual {
                    Some(values) => native_result(Ok(values)),
                    None => "ABSENT".into(),
                }
            }
            "R" | "S" => {
                transforms += 1;
                let et = f64::from_bits(u64::from_str_radix(args[3], 16).unwrap());
                let result = if args[0] == "R" {
                    frames
                        .pxform_in(args[1], args[2], et, Some(&pck))
                        .map(|x| x.concat())
                } else {
                    frames
                        .pxform_with_derivative(args[1], args[2], et, Some(&pck))
                        .map(|x| {
                            let mut state = vec![0.0; 36];
                            for row in 0..3 {
                                for column in 0..3 {
                                    state[row * 6 + column] = x.rotation[row][column];
                                    state[(row + 3) * 6 + column] = x.rotation_dot[row][column];
                                    state[(row + 3) * 6 + column + 3] = x.rotation[row][column];
                                }
                            }
                            state
                        })
                };
                native_result(result)
            }
            "B" => continue,
            other => panic!("Unexpected oracle command: {other}"),
        };
        if actual != native {
            failures.push(format!("{case} {command}: {actual}; native {native}"));
        }
    }
    assert_eq!((resets, loads, constants, transforms), (8, 16, 32, 32));
    assert!(
        failures.is_empty(),
        "{} native mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
