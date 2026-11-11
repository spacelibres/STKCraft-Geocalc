//! Point locations and the topocentric frame against CSPICE.
//!
//! `location_coordinates.csv` holds the eight WebGeocalc location
//! coordinate systems converted with the CSPICE inverse routines.
//! `location_states.csv` holds apparent states with a point target or
//! observer from `spkcpt`, `spkcvo`, and `spkcvt`, over every `refloc`
//! locus and aberration correction. `location_topocentric.csv` holds
//! `azlcpo` range, azimuth, elevation, and range rate, which is the same
//! frame WebGeocalc builds for `TOPOCENTRIC_AT_POINT`.

use std::f64::consts::TAU;
use stkcraft_geocalc::location::state_between_in;

use stkcraft_geocalc::{
    point_state, state_columns_with_pck, Aberration, Endpoint, FrameLocus, Frames,
    LocationCoordinates, OutputFrame, Pck, PointLocation, SpkKernel, StateRepresentation,
};

const COORDINATES: &str = include_str!("golden/location_coordinates.csv");
const STATES: &str = include_str!("golden/location_states.csv");
const TOPOCENTRIC: &str = include_str!("golden/location_topocentric.csv");

const PCK_PATHS: [&str; 3] = [
    "target/kernels/pck00011.tpc",
    "target/kernels/mars_iau2000_v1.tpc",
    "target/kernels/gm_de431.tpc",
];

const DEG: f64 = std::f64::consts::PI / 180.0;

fn load_pck() -> Pck {
    let mut pck = Pck::default();
    for path in PCK_PATHS {
        pck.furnsh(&std::fs::read_to_string(path).expect("kernel readable"))
            .expect("kernel parsed");
    }
    pck
}

fn body_id(name: &str) -> i32 {
    match name {
        "EARTH" => 399,
        "MOON" => 301,
        "MARS" => 499,
        "SUN" => 10,
        other => other.parse().expect("body id"),
    }
}

/// Maps a CSPICE body-fixed frame name to the name this crate uses.
fn frame_name(name: &str) -> String {
    match name {
        "IAU_EARTH" => "IAU_399".to_string(),
        "IAU_MOON" => "IAU_301".to_string(),
        "IAU_MARS" => "IAU_499".to_string(),
        other => other.to_string(),
    }
}

fn coordinates(label: &str) -> LocationCoordinates {
    match label {
        "latrec" => LocationCoordinates::Latitudinal {
            radius: 4000.0,
            longitude: 30.0 * DEG,
            latitude: 45.0 * DEG,
        },
        "sphrec" => LocationCoordinates::Spherical {
            radius: 5000.0,
            colatitude: 50.0 * DEG,
            longitude: 30.0 * DEG,
        },
        "cylrec" => LocationCoordinates::Cylindrical {
            radius: 4000.0,
            longitude: 30.0 * DEG,
            z: 500.0,
        },
        "radrec" => LocationCoordinates::RightAscensionDeclination {
            range: 4000.0,
            right_ascension: 30.0 * DEG,
            declination: 45.0 * DEG,
        },
        "georec_earth" => LocationCoordinates::Planetodetic {
            longitude: 30.0 * DEG,
            latitude: 45.0 * DEG,
            altitude: 0.5,
        },
        "pgrrec_earth" => LocationCoordinates::Planetographic {
            longitude: 30.0 * DEG,
            latitude: 45.0 * DEG,
            altitude: 0.5,
        },
        "georec_mars" => LocationCoordinates::Planetodetic {
            longitude: 30.0 * DEG,
            latitude: 45.0 * DEG,
            altitude: 0.5,
        },
        "pgrrec_mars" => LocationCoordinates::Planetographic {
            longitude: 30.0 * DEG,
            latitude: 45.0 * DEG,
            altitude: 0.5,
        },
        "pgrrec_mars_west" => LocationCoordinates::Planetographic {
            longitude: -30.0 * DEG,
            latitude: -20.0 * DEG,
            altitude: 12.0,
        },
        "azlrec_0_0" => LocationCoordinates::AzimuthElevation {
            range: 4000.0,
            azimuth: 30.0 * DEG,
            elevation: 20.0 * DEG,
            azimuth_counterclockwise: false,
            elevation_positive_up: false,
        },
        "azlrec_0_1" => LocationCoordinates::AzimuthElevation {
            range: 4000.0,
            azimuth: 30.0 * DEG,
            elevation: 20.0 * DEG,
            azimuth_counterclockwise: false,
            elevation_positive_up: true,
        },
        "azlrec_1_0" => LocationCoordinates::AzimuthElevation {
            range: 4000.0,
            azimuth: 30.0 * DEG,
            elevation: 20.0 * DEG,
            azimuth_counterclockwise: true,
            elevation_positive_up: false,
        },
        "azlrec_1_1" => LocationCoordinates::AzimuthElevation {
            range: 4000.0,
            azimuth: 30.0 * DEG,
            elevation: 20.0 * DEG,
            azimuth_counterclockwise: true,
            elevation_positive_up: true,
        },
        other => panic!("unknown coordinate case {other}"),
    }
}

#[test]
fn location_coordinates_match_cspice() {
    let pck = load_pck();
    let mut worst: f64 = 0.0;
    let mut checked = 0;

    for line in COORDINATES.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        let case = fields[0];
        let expected: Vec<f64> = fields[1..4]
            .iter()
            .map(|value| value.parse().expect("value"))
            .collect();

        // `pgrrec` needs the body whose longitude sense applies; the other
        // forms are body independent.
        let body = match case {
            "georec_earth" | "pgrrec_earth" => 399,
            "georec_mars" | "pgrrec_mars" | "pgrrec_mars_west" => 499,
            _ => 399,
        };
        let rectangular = coordinates(case)
            .to_rectangular(&pck, body)
            .unwrap_or_else(|error| panic!("{case}: {error}"));

        for index in 0..3 {
            assert_eq!(
                rectangular[index].to_bits(),
                expected[index].to_bits(),
                "{case}: coordinate axis{index} {:?} vs {:?}",
                rectangular[index],
                expected[index]
            );
            worst = worst.max((rectangular[index] - expected[index]).abs());
        }
        checked += 1;
    }

    println!("coordinates: {checked} cases, worst {worst:.3e} km");
    assert_eq!(checked, 13, "every coordinate case must be exercised");
    assert!(
        worst == 0.0,
        "coordinate residual {worst:e} exceeds 1e-9 km"
    );
}

struct Row {
    endpoint_kind: String,
    object: String,
    center: String,
    frame: String,
    position: [f64; 3],
    velocity: [f64; 3],
    epoch: f64,
}

fn parse_row(fields: &[&str], offset: usize) -> Row {
    let number = |index: usize| fields[index].parse::<f64>().expect("value");
    Row {
        endpoint_kind: fields[offset].to_string(),
        object: fields[offset + 1].to_string(),
        center: fields[offset + 2].to_string(),
        frame: fields[offset + 3].to_string(),
        position: [number(offset + 4), number(offset + 5), number(offset + 6)],
        velocity: [number(offset + 7), number(offset + 8), number(offset + 9)],
        epoch: number(offset + 10),
    }
}

impl Row {
    /// A placeholder for an object endpoint, which carries no location.
    fn location_or_placeholder(&self) -> PointLocation {
        if self.endpoint_kind == "OBJECT" {
            PointLocation::fixed(0, "J2000", [0.0; 3])
        } else {
            self.location()
        }
    }

    fn location(&self) -> PointLocation {
        let center = body_id(&self.center);
        if self.velocity == [0.0; 3] {
            PointLocation::fixed(center, frame_name(&self.frame), self.position)
        } else {
            PointLocation::constant_velocity(
                center,
                frame_name(&self.frame),
                self.position,
                self.velocity,
                self.epoch,
            )
        }
    }
}

fn endpoint<'a>(row: &'a Row, location: &'a PointLocation) -> Endpoint<'a> {
    match row.endpoint_kind.as_str() {
        "OBJECT" => Endpoint::Object(body_id(&row.object)),
        "POINT" | "CV_POINT" => Endpoint::Point(location),
        other => panic!("unknown endpoint kind {other}"),
    }
}

#[test]
fn point_states_match_cspice() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").expect("SPK readable");
    let pck = load_pck();
    let frames = Frames::new();
    let et = 1.0e8;

    let mut worst_position: f64 = 0.0;
    let mut worst_velocity: f64 = 0.0;
    let mut worst_light_time: f64 = 0.0;
    let mut checked = 0;

    for line in STATES.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        let case = fields[0];

        let target_row = parse_row(&fields, 1);
        let observer_row = parse_row(&fields, 12);
        let outref = fields[23];
        let refloc = fields[24];
        let abcorr = Aberration::parse(fields[25]).expect("aberration");
        let expected_state: Vec<f64> = fields[26..32]
            .iter()
            .map(|value| value.parse().expect("value"))
            .collect();
        let expected_light_time: f64 = fields[32].parse().expect("light time");

        let target_location = target_row.location_or_placeholder();
        let observer_location = observer_row.location_or_placeholder();
        let target = endpoint(&target_row, &target_location);
        let observer = endpoint(&observer_row, &observer_location);

        let expressed = state_between_in(
            &kernel,
            &frames,
            &pck,
            target,
            observer,
            OutputFrame::Spice(&frame_name(outref)),
            FrameLocus::parse(refloc).unwrap(),
            et,
            abcorr,
        )
        .unwrap_or_else(|error| panic!("{case}: {error}"));

        for index in 0..3 {
            assert_eq!(
                expressed.state.position[index].to_bits(),
                expected_state[index].to_bits(),
                "{case}: position axis{index} {:?} vs {:?}",
                expressed.state.position[index],
                expected_state[index]
            );
            assert_eq!(
                expressed.state.velocity[index].to_bits(),
                expected_state[3 + index].to_bits(),
                "{case}: velocity axis{index} {:?} vs {:?}",
                expressed.state.velocity[index],
                expected_state[3 + index]
            );
            worst_position =
                worst_position.max((expressed.state.position[index] - expected_state[index]).abs());
            worst_velocity = worst_velocity
                .max((expressed.state.velocity[index] - expected_state[3 + index]).abs());
        }
        assert_eq!(
            expressed.light_time.to_bits(),
            expected_light_time.to_bits(),
            "{case}: light_time"
        );
        worst_light_time = worst_light_time.max((expressed.light_time - expected_light_time).abs());
        checked += 1;
    }

    println!(
        "states: {checked} cases, position {worst_position:.3e} km, velocity {worst_velocity:.3e} km/s, light time {worst_light_time:.3e} s"
    );
    assert_eq!(checked, 27, "every state case must be exercised");
    // The position bound is a millimetre at lunar distance; the velocity
    // bound is a micrometre per second.
    assert!(
        worst_position == 0.0,
        "position residual {worst_position:e} km exceeds 1e-3"
    );
    assert!(
        worst_velocity == 0.0,
        "velocity residual {worst_velocity:e} km/s exceeds 1e-6"
    );
    assert!(
        worst_light_time == 0.0,
        "light time residual {worst_light_time:e} s exceeds 1e-9"
    );
}

fn wrap(angle: f64) -> f64 {
    let mut difference = angle.rem_euclid(TAU);
    if difference > std::f64::consts::PI {
        difference -= TAU;
    }
    difference.abs()
}

#[test]
fn topocentric_frame_matches_azlcpo() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").expect("SPK readable");
    let pck = load_pck();
    let frames = Frames::new();
    let et = 1.0e8;

    // The location the golden generator placed on the Earth.
    let location = PointLocation::from_coordinates(
        399,
        "IAU_399",
        LocationCoordinates::Planetodetic {
            longitude: -118.0 * DEG,
            latitude: 34.0 * DEG,
            altitude: 0.3,
        },
        &pck,
    )
    .expect("location");

    let mut worst_angle: f64 = 0.0;
    let mut worst_range: f64 = 0.0;
    let mut worst_rate: f64 = 0.0;
    let mut checked = 0;

    for line in TOPOCENTRIC.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case,") {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        let case = fields[0];
        let az_ccw = fields[1] == "1";
        let el_plus_z = fields[2] == "1";
        let abcorr = Aberration::parse(fields[3]).expect("aberration");
        let expected: Vec<f64> = fields[4..9]
            .iter()
            .map(|value| value.parse().expect("value"))
            .collect();

        let observer = Endpoint::Point(&location);
        let expressed = state_between_in(
            &kernel,
            &frames,
            &pck,
            Endpoint::Object(301),
            observer,
            OutputFrame::Topocentric(&location),
            FrameLocus::Observer,
            et,
            abcorr,
        )
        .unwrap_or_else(|error| panic!("{case}: {error}"));

        let columns = state_columns_with_pck(
            et,
            expressed.state,
            StateRepresentation::azimuth_elevation(az_ccw, el_plus_z),
            expressed.light_time,
            abcorr,
            &pck,
            399,
        )
        .expect("columns");
        // WGC AZ_EL order: azimuth, elevation, range, then their rates.
        let row = columns.numeric_row();

        assert_eq!(
            row[2].to_bits(),
            expected[0].to_bits(),
            "{case}: range {:?} vs {:?}",
            row[2],
            expected[0]
        );
        assert_eq!(
            row[0].to_bits(),
            expected[1].to_degrees().to_bits(),
            "{case}: azimuth"
        );
        assert_eq!(
            row[1].to_bits(),
            expected[2].to_degrees().to_bits(),
            "{case}: elevation"
        );
        assert_eq!(
            row[5].to_bits(),
            expected[3].to_bits(),
            "{case}: range rate"
        );
        worst_range = worst_range.max((row[2] - expected[0]).abs());
        // The row is in degrees, the golden file in radians.
        worst_angle = worst_angle.max(wrap((row[0] - expected[1].to_degrees()).to_radians()));
        worst_angle = worst_angle.max(wrap((row[1] - expected[2].to_degrees()).to_radians()));
        worst_rate = worst_rate.max((row[5] - expected[3]).abs());
        checked += 1;
    }

    println!(
        "topocentric: {checked} cases, range {worst_range:.3e} km, angle {worst_angle:.3e} rad, range rate {worst_rate:.3e} km/s"
    );
    assert_eq!(checked, 8, "every topocentric case must be exercised");
    assert!(worst_range == 0.0, "range residual {worst_range:e} km");
    assert!(worst_angle == 0.0, "angle residual {worst_angle:e} rad");
    assert!(worst_rate == 0.0, "range-rate residual {worst_rate:e} km/s");
}

/// A point fixed at the centre of its centre body must reproduce the
/// object's own state, which is the sanity check that the rotating-frame
/// velocity term is not double counted.
#[test]
fn point_at_centre_is_the_object() {
    let kernel = SpkKernel::open("target/kernels/de440s.bsp").expect("SPK readable");
    let pck = load_pck();
    let frames = Frames::new();
    let location = PointLocation::fixed(301, "IAU_301", [0.0, 0.0, 0.0]);

    let point = point_state(&kernel, &frames, &pck, &location, 1.0e8).expect("point state");
    let object = kernel.state(301, 0, 1.0e8).expect("object state");

    for index in 0..3 {
        assert!((point.position[index] - object.position[index]).abs() < 1e-9);
        assert!((point.velocity[index] - object.velocity[index]).abs() < 1e-12);
    }
}

/// The topocentric frame's `+Z` axis must be the outward surface normal.
#[test]
fn topocentric_zenith_is_the_surface_normal() {
    let pck = load_pck();
    let frames = Frames::new();
    let location = PointLocation::from_coordinates(
        399,
        "IAU_399",
        LocationCoordinates::Planetodetic {
            longitude: 1.0,
            latitude: 0.5,
            altitude: 0.0,
        },
        &pck,
    )
    .expect("location");

    let (rotation, _) =
        stkcraft_geocalc::topocentric_rotation(&frames, &pck, &location, 0.0).expect("rotation");

    // The outward unit normal of the reference spheroid at the location.
    let radii = pck.radii(399).expect("radii");
    let mut normal = [
        location.position[0] / (radii[0] * radii[0]),
        location.position[1] / (radii[1] * radii[1]),
        location.position[2] / (radii[2] * radii[2]),
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    for value in &mut normal {
        *value /= length;
    }

    // Express it in J2000 with the body-fixed rotation from the PCK.
    let to_body = pck.body_rotation(399, 0.0).expect("body rotation");
    let to_j2000 = stkcraft_geocalc::frame::transpose(&to_body);
    for row in 0..3 {
        let expected = (0..3)
            .map(|column| to_j2000[row][column] * normal[column])
            .sum::<f64>();
        let difference = (rotation[2][row] - expected).abs();
        assert!(difference < 1e-12, "axis {row} differs by {difference:e}");
    }
}
