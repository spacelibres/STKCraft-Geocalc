//! Independent native historical SPK10 states, positions and light times.
use stkcraft_geocalc::{Aberration, FrameSource, Frames, Pck, SpkKernel};
fn number(text: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(text, 16).unwrap())
}

#[test]
fn native_historical_spk10_corrected_state_position_cells_exact() {
    let mut kernel = SpkKernel::open_all(&[
        "tests/golden/spk10_historical.bsp",
        "target/kernels/de440s.bsp",
    ])
    .unwrap();
    kernel.attach_lsk(
        stkcraft_time::LeapSecondTable::from_lsk(
            &std::fs::read_to_string("target/kernels/naif0012.tls").unwrap(),
        )
        .unwrap(),
    );
    let frames = Frames::new();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let mut failures = Vec::new();
    let (mut count, mut cells, mut errors) = (0, 0, 0);
    for row in include_str!("golden/spk10_historical.txt").lines() {
        let f: Vec<_> = row.split('|').collect();
        assert_eq!(f.len(), 18);
        assert_eq!(&f[5..7], &["N", "11"]);
        let target = f[0].parse().unwrap();
        let et = number(f[1]);
        let correction = Aberration::parse(f[3]).unwrap();
        let observer = f[4].parse().unwrap();
        let state = kernel.apparent_state_in(
            target,
            observer,
            et,
            correction,
            &FrameSource::Named {
                frames: &frames,
                pck: &pck,
                frame_name: f[2],
            },
        );
        let position = kernel.position_corrected_in_frame(
            target,
            observer,
            et,
            correction,
            &frames,
            Some(&pck),
            f[2],
        );
        match (state, position) {
            (Ok(state), Ok((position, light_time))) => {
                let actual = [
                    state.state.position[0],
                    state.state.position[1],
                    state.state.position[2],
                    state.state.velocity[0],
                    state.state.velocity[1],
                    state.state.velocity[2],
                    state.light_time,
                    position[0],
                    position[1],
                    position[2],
                    light_time,
                ];
                for (index, (value, expected)) in actual.iter().zip(&f[7..]).enumerate() {
                    if !value.is_finite() || value.to_bits() != number(expected).to_bits() {
                        failures.push(format!(
                            "{}: column {index}, actual {:016x}, native {expected}",
                            f[..5].join("|"),
                            value.to_bits()
                        ));
                    }
                    cells += 1;
                }
            }
            (state, position) => {
                errors += 1;
                failures.push(format!(
                    "{}: state {state:?}; position {position:?}",
                    f[..5].join("|")
                ));
            }
        }
        count += 1;
    }
    assert_eq!(count, 33696);
    assert!(
        failures.is_empty(),
        "{} mismatches, {errors} rejected native-valid queries, {cells} compared cells:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!((cells, errors), (370656, 0));
}
