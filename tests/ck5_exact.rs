use stkcraft_geocalc::ck::CkKernel;
#[test]
fn native_ck5_packet_interpolation_exact() {
    let ck = CkKernel::open("tests/golden/ck5.bc").unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in include_str!("golden/ck5.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let i = f[0].parse().unwrap();
        let t = f[1].parse().unwrap();
        let tol = f[2].parse().unwrap();
        let reference = f[4].parse().unwrap();
        let result = if f[3] == "1" {
            ck.ckgpav(i, t, tol, reference)
        } else {
            ck.ckgp(i, t, tol, reference)
        };
        match result {
            Ok(Some(p)) if f[5] == "FOUND" => {
                let mut actual = vec![p.clock];
                actual.extend(p.rotation.concat());
                if f[3] == "1" {
                    actual.extend(p.angular_velocity.unwrap());
                }
                let expected: Vec<f64> = f[6..if f[3] == "1" { 19 } else { 16 }]
                    .iter()
                    .map(|v| v.parse().unwrap())
                    .collect();
                if actual != expected {
                    failures.push(format!("{}:{actual:?} != {expected:?}", f[..5].join(",")));
                }
            }
            Ok(None) if f[5] == "NONE" => {}
            Err(e) if e.to_string().contains(f[5]) => {}
            other => failures.push(format!("{}:{other:?} native{}", f[..5].join(","), f[5])),
        }
        count += 1;
    }
    assert_eq!(count, 19072);
    assert!(
        failures.is_empty(),
        "{}/{} CK5 failures:\n{}",
        failures.len(),
        count,
        failures
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
