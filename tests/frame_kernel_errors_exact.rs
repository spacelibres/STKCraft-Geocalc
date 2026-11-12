//! Native fixed/dynamic frame metadata, typed values and identity bypass.
use stkcraft_geocalc::{Frames, GeoError};

fn encode(result: Result<Vec<f64>, GeoError>) -> String {
    match result {
        Ok(values) => {
            let bits = values
                .iter()
                .map(|x| format!("{:016x}", x.to_bits()))
                .collect::<Vec<_>>();
            format!("N|{}|{}", bits.len(), bits.join("|"))
        }
        Err(error) => {
            let text = error.to_string();
            if let Some(start) = text.find("SPICE(") {
                if let Some(end) = text[start..].find(')') {
                    return format!("E|{}", &text[start..=start + end]);
                }
            }
            format!("E|{text}")
        }
    }
}

#[test]
fn native_fixed_and_dynamic_frame_error_matrix_exact() {
    let mut frames = Frames::new();
    let mut failures = Vec::new();
    let (mut loads, mut transforms, mut resets) = (0, 0, 0);
    for line in include_str!("golden/frame_kernel_errors.tsv")
        .lines()
        .skip(1)
    {
        let fields: Vec<_> = line.split('\t').collect();
        let query: Vec<_> = fields[1].split('|').collect();
        let actual = match query[0] {
            "C" => {
                frames = Frames::new();
                resets += 1;
                "OK".to_string()
            }
            "L" => {
                loads += 1;
                let text = std::fs::read_to_string(format!(
                    "tests/golden/frame_kernel_errors/{}",
                    query[1]
                ))
                .unwrap();
                match frames.furnsh(&text) {
                    Ok(()) => "OK".to_string(),
                    Err(error) => encode(Err(error)),
                }
            }
            "R" | "S" => {
                transforms += 1;
                let et = f64::from_bits(u64::from_str_radix(query[3], 16).unwrap());
                if query[0] == "R" {
                    encode(frames.pxform(query[1], query[2], et).map(|x| x.concat()))
                } else {
                    encode(frames.sxform(query[1], query[2], et).map(|x| x.concat()))
                }
            }
            other => panic!("Unexpected native frame operation: {other}"),
        };
        if actual != fields[2] {
            failures.push(format!(
                "{} {}: {actual} != {}",
                fields[0], fields[1], fields[2]
            ));
        }
    }
    assert_eq!((resets, loads, transforms), (120, 120, 480));
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
