use stkcraft_geocalc::location::state_between_in;
use stkcraft_geocalc::*;
#[test]
fn local_matrix_and_body_state_match_cspice_intermediates() {
    let rows: Vec<Vec<&str>> = include_str!("golden/location_topocentric_intermediate.csv")
        .lines()
        .map(|line| line.split(',').collect())
        .collect();
    let get = |name: &str| -> [f64; 3] {
        let row = rows.iter().find(|row| row[0] == name).unwrap();
        [
            row[1].parse().unwrap(),
            row[2].parse().unwrap(),
            row[3].parse().unwrap(),
        ]
    };
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::new();
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let input = get("position");
    let actual = PointLocation::from_coordinates(
        399,
        "IAU_EARTH",
        LocationCoordinates::Planetodetic {
            longitude: -118. * (std::f64::consts::PI / 180.),
            latitude: 34. * (std::f64::consts::PI / 180.),
            altitude: 0.3,
        },
        &pck,
    )
    .unwrap();
    for (i, expected) in input.iter().enumerate() {
        assert_eq!(
            actual.position[i].to_bits(),
            expected.to_bits(),
            "station axis{i}"
        );
    }
    let dummy = PointLocation::fixed(399, "J2000", input);
    let (local, _) = topocentric_rotation(&frames, &pck, &dummy, 1e8).unwrap();
    for (i, row) in local.iter().enumerate() {
        let e = get(&format!("local{i}"));
        for (j, expected) in e.iter().enumerate() {
            assert_eq!(
                row[j].to_bits(),
                expected.to_bits(),
                "local{i}{j} actual{:?} expected{:?}",
                row[j],
                expected
            );
        }
    }
    let observer = Endpoint::Point(&actual);
    let body = state_between_in(
        &kernel,
        &frames,
        &pck,
        Endpoint::Object(301),
        observer,
        OutputFrame::Spice("IAU_EARTH"),
        FrameLocus::Observer,
        1e8,
        Aberration::None,
    )
    .unwrap();
    for (name, a) in [
        ("body_position", body.state.position),
        ("body_velocity", body.state.velocity),
    ] {
        let e = get(name);
        for i in 0..3 {
            assert_eq!(
                a[i].to_bits(),
                e[i].to_bits(),
                "{name} axis{i} actual{:?} expected{:?}",
                a[i],
                e[i]
            );
        }
    }
    let topo = state_between_in(
        &kernel,
        &frames,
        &pck,
        Endpoint::Object(301),
        observer,
        OutputFrame::Topocentric(&actual),
        FrameLocus::Observer,
        1e8,
        Aberration::None,
    )
    .unwrap();
    for (name, a) in [
        ("local_position", topo.state.position),
        ("local_velocity", topo.state.velocity),
    ] {
        let e = get(name);
        for i in 0..3 {
            assert_eq!(
                a[i].to_bits(),
                e[i].to_bits(),
                "{name} axis{i} actual{:?} expected{:?}",
                a[i],
                e[i]
            );
        }
    }
}
#[test]
fn topocentric_output_requires_its_fixed_point_observer() {
    let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
    let frames = Frames::new();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let moving =
        PointLocation::constant_velocity(399, "IAU_EARTH", [7000., 0., 0.], [1., 0., 0.], 0.);
    assert!(state_between_in(
        &kernel,
        &frames,
        &pck,
        Endpoint::Object(301),
        Endpoint::Point(&moving),
        OutputFrame::Topocentric(&moving),
        FrameLocus::Observer,
        0.,
        Aberration::None
    )
    .is_err());
    let fixed = PointLocation::fixed(399, "IAU_EARTH", [7000., 0., 0.]);
    assert!(state_between_in(
        &kernel,
        &frames,
        &pck,
        Endpoint::Object(301),
        Endpoint::Object(399),
        OutputFrame::Topocentric(&fixed),
        FrameLocus::Observer,
        0.,
        Aberration::None
    )
    .is_err());
}
