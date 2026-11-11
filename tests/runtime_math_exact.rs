//! Raw binary64 outputs captured by the independent /MT C oracle.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::hint::black_box;

#[test]
fn native_windows_math_primitives_exact() {
    let data = include_str!("golden/runtime_math.csv");
    let mut counts = BTreeMap::new();
    let mut checked = 0;
    let mut mismatches = 0;
    let mut examples = Vec::new();
    for (index, row) in data.lines().skip(1).enumerate() {
        let cells: Vec<_> = row.split(',').collect();
        assert_eq!(cells.len(), 5, "fixture row {index}");
        let parse = |value: &str| u64::from_str_radix(value, 16).expect("binary64 fixture bits");
        let x = black_box(f64::from_bits(parse(cells[1])));
        let y = black_box(f64::from_bits(parse(cells[2])));
        assert!(x.is_finite() && y.is_finite());
        let (actual, width) = match cells[0] {
            "asin" => ([x.asin(), 0.], 1),
            "acos" => ([x.acos(), 0.], 1),
            "atan2" => ([x.atan2(y), 0.], 1),
            "cos" => ([x.cos(), 0.], 1),
            "cosh" => ([x.cosh(), 0.], 1),
            "exp" => ([x.exp(), 0.], 1),
            "log" => ([x.ln(), 0.], 1),
            "pow" => ([x.powf(y), 0.], 1),
            "sin" => ([x.sin(), 0.], 1),
            "sinh" => ([x.sinh(), 0.], 1),
            "sqrt" => ([x.sqrt(), 0.], 1),
            "tan" => ([x.tan(), 0.], 1),
            "sin_cos" => {
                let (sin, cos) = x.sin_cos();
                ([sin, cos], 2)
            }
            other => panic!("unknown native operation: {other}"),
        };
        *counts.entry(cells[0]).or_insert(0usize) += 1;
        assert_eq!(cells[4].is_empty(), width == 1);
        for component in 0..width {
            let expected = parse(cells[3 + component]);
            assert!(f64::from_bits(expected).is_finite());
            let actual = actual[component].to_bits();
            if actual != expected {
                mismatches += 1;
                if examples.len() < 16 {
                    examples.push(format!(
                        "row {index} {}({}, {}) component {component}: {actual:016x} != {expected:016x}",
                        cells[0], cells[1], cells[2]
                    ));
                }
            }
            checked += 1;
        }
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("asin", 1415),
            ("acos", 1415),
            ("atan2", 1036),
            ("cos", 1005),
            ("cosh", 1000),
            ("exp", 1005),
            ("log", 1005),
            ("pow", 1008),
            ("sin", 1005),
            ("sinh", 1000),
            ("sqrt", 1005),
            ("tan", 1005),
            ("sin_cos", 1005),
        ])
    );
    assert_eq!(checked, 14914);
    println!("Native math primitive cells {checked}; mismatches {mismatches}; {counts:?}");
    assert_eq!(mismatches, 0, "{}", examples.join("\n"));
}
