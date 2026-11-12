//! Independent native calculations after text-kernel append operations.
use stkcraft_geocalc::{Fov, Frames, Pck};

fn rows(case: &str) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
    include_str!("golden/kernel_pool.tsv")
        .lines()
        .skip(1)
        .filter_map(move |line| {
            let mut fields = line.split('\t');
            let name = fields.next().unwrap();
            let command = fields.next().unwrap();
            let native = fields.next().unwrap();
            (name == case).then_some((command, native))
        })
}

fn kernel(name: &str) -> String {
    std::fs::read_to_string(format!("tests/golden/kernel_pool/{name}")).unwrap()
}

fn check_numbers(actual: &[f64], native: &str, label: &str) {
    let values: Vec<_> = native.split('|').collect();
    assert_eq!(values[0], "N", "{label}");
    let count: usize = values[1].parse().unwrap();
    assert_eq!(actual.len(), count, "{label}");
    assert_eq!(values.len(), count + 2);
    for (index, value) in actual.iter().enumerate() {
        let expected = u64::from_str_radix(values[index + 2], 16).unwrap();
        assert_eq!(value.to_bits(), expected, "{label} component {index}");
    }
}

#[test]
fn native_text_kernel_frame_appends_exact() {
    let mut checked = 0;
    for case in [
        "frame_append",
        "frame_partial_update",
        "frame_cross_file_append",
        "frame_integer_notation",
    ] {
        let mut frames = Frames::new();
        for (command, native) in rows(case) {
            let fields: Vec<_> = command.split('|').collect();
            match fields[0] {
                "L" => frames
                    .furnsh(&kernel(fields[1]))
                    .unwrap_or_else(|e| panic!("{case}: {e}")),
                "R" | "S" => {
                    let et = f64::from_bits(u64::from_str_radix(fields[3], 16).unwrap());
                    let actual = if fields[0] == "R" {
                        frames.pxform(fields[1], fields[2], et).unwrap().concat()
                    } else {
                        frames.sxform(fields[1], fields[2], et).unwrap().concat()
                    };
                    check_numbers(&actual, native, case);
                    checked += 1;
                }
                _ => {}
            }
        }
    }
    assert_eq!(checked, 10);
}

#[test]
fn native_text_kernel_pck_appends_exact() {
    let mut pck = Pck::default();
    let mut checked = 0;
    for (command, native) in rows("pck_append") {
        let fields: Vec<_> = command.split('|').collect();
        match fields[0] {
            "L" => pck.furnsh(&kernel(fields[1])).unwrap(),
            "B" => {
                let actual = if fields[2] == "RADII" {
                    pck.radii(9999).unwrap().to_vec()
                } else {
                    vec![pck.gm(9999).unwrap()]
                };
                check_numbers(&actual, native, command);
                checked += 1;
            }
            _ => {}
        }
    }
    assert_eq!(checked, 4);
}

#[test]
fn native_text_kernel_fov_appends_exact() {
    let text = kernel("fov_append-0.tf");
    let fov = Fov::from_text(&text, -96301).unwrap();
    let native = rows("fov_append")
        .find(|(command, _)| command.starts_with("F|"))
        .unwrap()
        .1;
    let fields: Vec<_> = native.split('|').collect();
    assert_eq!(fields[0], "F");
    assert_eq!(fov.frame, "J2000");
    assert_eq!(fields[1], "52454354414e474c45");
    assert_eq!(fields[2], "4a32303030");
    assert_eq!(fov.boundary.len(), fields[3].parse::<usize>().unwrap());
    let actual = [fov.original_boresight().to_vec(), fov.boundary.concat()].concat();
    let expected = format!("N|{}|{}", actual.len(), fields[4..].join("|"));
    check_numbers(&actual, &expected, "GETFOV appended vectors");
}

#[test]
fn native_text_kernel_quoted_numeric_axes_rejected() {
    let native = rows("frame_quoted_axes")
        .find(|(command, _)| command.starts_with("R|"))
        .unwrap()
        .1;
    assert_eq!(native, "E|SPICE(BADVARIABLETYPE)");
    let result = Frames::from_text(&kernel("frame_quoted_axes-0.tf"))
        .and_then(|frames| frames.pxform("POOL_FRAME", "J2000", 0.0));
    let error = result
        .expect_err("Native rejects character-valued axes, including numeric-looking strings");
    assert!(
        error.to_string().contains("SPICE(BADVARIABLETYPE)"),
        "{error}"
    );
}
