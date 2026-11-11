use stkcraft_geocalc::{LocationCoordinates, Pck};
#[test]
fn geodetic_locations_equal_cspice_georec() {
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    for line in include_str!("golden/georec_precision.csv").lines() {
        let f: Vec<f64> = line.split(',').map(|v| v.parse().unwrap()).collect();
        let body = if f[3] > 6000. { 399 } else { 301 };
        let actual = LocationCoordinates::Planetodetic {
            longitude: f[0],
            latitude: f[1],
            altitude: f[2],
        }
        .to_rectangular(&pck, body)
        .unwrap();
        assert_eq!(
            actual,
            [f[6], f[7], f[8]],
            "body{body} lon{} lat{} alt{}",
            f[0],
            f[1],
            f[2]
        );
    }
}
