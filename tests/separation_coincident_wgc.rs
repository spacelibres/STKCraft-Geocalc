//! WGC point-coincidence observations and independent CSPICE VSEP references.
use stkcraft_geocalc::{
    angular_separation, angular_separation_shaped, find_angular_separation, Aberration,
    AngularShape, Condition, FinderOptions, Interval, Pck, SpkKernel,
};

#[test]
fn wgc_coincident_point_targets_and_sphere_limbs() {
    let kernel = SpkKernel::open("target/kernels/de442.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    for correction in [Aberration::None, Aberration::LightTimeStellar] {
        let value = angular_separation_shaped(
            &kernel,
            &pck,
            301,
            301,
            399,
            0.0,
            correction,
            AngularShape::Point,
            AngularShape::Point,
        )
        .unwrap();
        assert_eq!(value.to_bits(), 0.0_f64.to_bits());
        assert_eq!(
            angular_separation(&kernel, 301, 301, 399, 0.0, correction)
                .unwrap()
                .to_bits(),
            0.0_f64.to_bits()
        );
    }
    for (first, second, s1, s2) in [
        (301, 399, AngularShape::Point, AngularShape::Sphere),
        (399, 301, AngularShape::Sphere, AngularShape::Point),
    ] {
        let value = angular_separation_shaped(
            &kernel,
            &pck,
            301,
            first,
            second,
            0.0,
            Aberration::None,
            s1,
            s2,
        )
        .unwrap();
        assert_eq!(value.to_bits(), 0xbf903ab793085c7f);
    }
    assert!(angular_separation_shaped(
        &kernel,
        &pck,
        301,
        301,
        399,
        0.0,
        Aberration::None,
        AngularShape::Sphere,
        AngularShape::Point
    )
    .is_err());
    let search = find_angular_separation(
        &kernel,
        &pck,
        301,
        301,
        399,
        Aberration::None,
        AngularShape::Point,
        AngularShape::Point,
        &[Interval {
            start: 0.0,
            end: 1000.0,
        }],
        Condition::LessThan,
        90.0_f64.to_radians(),
        FinderOptions {
            step: 100.0,
            tolerance: 1.0e-6,
            adjustment: 0.0,
        },
    );
    assert_eq!(
        search.unwrap_err().to_string(),
        "SPICE(BODIESNOTDISTINCT)",
        "WGC GFSEP identifies non-distinct bodies"
    );
}
