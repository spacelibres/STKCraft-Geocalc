use stkcraft_geocalc::{coords::{to_coordinates, to_coordinates_with_direction}, CoordinateSystem, Pck};
fn main() {
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let system = CoordinateSystem::Planetographic;
    let vector = [100000000.0, 100000000.0, 100000000.0];
    let radii = Some(pck.radii(299).unwrap());
    let east = pck.planetographic_east_positive(299).unwrap();
    let gf_path = to_coordinates(vector, system, radii, 299).unwrap();
    let pck_path = to_coordinates_with_direction(vector, system, radii, east).unwrap();
    println!("Venus east_positive={east}; GF coordinate_value path longitude={} deg; PCK-aware path longitude={} deg", gf_path[0].to_degrees(), pck_path[0].to_degrees());
}
