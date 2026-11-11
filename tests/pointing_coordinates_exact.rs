use stkcraft_geocalc::pointing::{
    represent_pointing_vector, PointingMagnitude, PointingRepresentation,
};

#[test]
fn every_coordinate_and_magnitude_matches_cspice_bits() {
    let mut rows = 0;
    for line in include_str!("golden/pointing_coordinates_exact.csv").lines() {
        if line.starts_with('#') || line.starts_with("representation") {
            continue;
        }
        let f: Vec<_> = line.split(',').collect();
        assert_eq!(f.len(), 11);
        let representation = match f[0] {
            "AZ_EL_TT" => PointingRepresentation::AzimuthElevation {
                az_ccw: true,
                el_plus_z: true,
            },
            "AZ_EL_TF" => PointingRepresentation::AzimuthElevation {
                az_ccw: true,
                el_plus_z: false,
            },
            "AZ_EL_FT" => PointingRepresentation::AzimuthElevation {
                az_ccw: false,
                el_plus_z: true,
            },
            "AZ_EL_FF" => PointingRepresentation::AzimuthElevation {
                az_ccw: false,
                el_plus_z: false,
            },
            other => PointingRepresentation::parse(other).unwrap(),
        };
        let magnitude = PointingMagnitude::parse(f[1]).unwrap();
        let numbers: Vec<f64> = f[2..].iter().map(|x| x.parse().unwrap()).collect();
        let actual = represent_pointing_vector(
            [numbers[0], numbers[1], numbers[2]],
            magnitude,
            representation,
        )
        .unwrap();
        for (i, value) in actual
            .vector
            .into_iter()
            .chain(actual.coordinates)
            .enumerate()
        {
            assert_eq!(
                value.to_bits(),
                numbers[i + 3].to_bits(),
                "row {rows} {line} column {i}; actual={value:?} expected={:?}",
                numbers[i + 3]
            );
        }
        rows += 1;
    }
    assert_eq!(rows, 180);
}

#[test]
fn nonfinite_directions_are_errors() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(represent_pointing_vector(
            [value, 0., 1.],
            PointingMagnitude::Unit,
            PointingRepresentation::Rectangular
        )
        .is_err());
    }
}
