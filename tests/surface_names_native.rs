use stkcraft_geocalc::{Pck, SurfaceNameResolver};

#[test]
fn native_surface_name_resolution_respects_body_precedence_and_numeric_fallback() {
    let pool = Pck::from_text("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME = ( 'Alpha  Surface', 'ALPHA SURFACE', 'Other', '100' )\nNAIF_SURFACE_CODE = ( 100, 101, 102, 42 )\nNAIF_SURFACE_BODY = ( 301, 301, 399, 301 )\n\\begintext\n").unwrap();
    let mut resolver = SurfaceNameResolver::new();
    assert_eq!(
        resolver.resolve(&pool, "  Alpha   Surface  ", 301).unwrap(),
        Some(101)
    );
    assert_eq!(resolver.resolve(&pool, "alpha surface", 399).unwrap(), None);
    assert_eq!(resolver.resolve(&pool, "OTHER", 399).unwrap(), Some(102));
    assert_eq!(resolver.resolve(&pool, "100", 301).unwrap(), Some(42));
    assert_eq!(resolver.resolve(&pool, "100", 399).unwrap(), Some(100));
    assert_eq!(resolver.resolve(&pool, "+100", 301).unwrap(), Some(100));
}

#[test]
fn native_surface_name_rejects_incomplete_mappings() {
    let pool =
        Pck::from_text("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME = ( 'A' )\n\\begintext\n").unwrap();
    let mut resolver = SurfaceNameResolver::new();
    assert_eq!(
        resolver.resolve(&pool, "A", 301).unwrap_err().to_string(),
        "SPICE(BADSURFACEMAP)"
    );
}

#[test]
fn native_surface_cache_retains_prior_mapping_after_bad_type_until_next_update() {
    let mut pool = Pck::from_text("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME = ( 'A' )\nNAIF_SURFACE_CODE = ( 1 )\nNAIF_SURFACE_BODY = ( 301 )\n\\begintext\n").unwrap();
    let mut resolver = SurfaceNameResolver::new();
    assert_eq!(resolver.resolve(&pool, "A", 301).unwrap(), Some(1));
    pool.furnsh("KPL/FK\n\\begindata\nNAIF_SURFACE_BODY = ( 'bad' )\n\\begintext\n")
        .unwrap();
    assert_eq!(
        resolver.resolve(&pool, "A", 301).unwrap_err().to_string(),
        "SPICE(BADVARIABLETYPE)"
    );
    assert_eq!(resolver.resolve(&pool, "A", 301).unwrap(), Some(1));
    pool.furnsh("KPL/FK\n\\begindata\nNAIF_SURFACE_BODY = ( 'bad' )\n\\begintext\n")
        .unwrap();
    assert_eq!(
        resolver.resolve(&pool, "A", 301).unwrap_err().to_string(),
        "SPICE(BADVARIABLETYPE)"
    );
    pool.furnsh("KPL/FK\n\\begindata\nNAIF_SURFACE_BODY = ( 301 )\n\\begintext\n")
        .unwrap();
    assert_eq!(resolver.resolve(&pool, "A", 301).unwrap(), Some(1));
}

#[test]
fn native_surface_mapping_validation_and_numeric_fallback() {
    let cases = [
        ("NAIF_SURFACE_NAME=( ' ' )\nNAIF_SURFACE_CODE=( 2147483648 )\nNAIF_SURFACE_BODY=( 301 )", "SPICE(INTOUTOFRANGE)"),
        ("NAIF_SURFACE_NAME=( ' ' )\nNAIF_SURFACE_CODE=( 1 )\nNAIF_SURFACE_BODY=( 301 )", "SPICE(BLANKNAMEASSIGNED)"),
        ("NAIF_SURFACE_NAME=( 'A' 'B' )\nNAIF_SURFACE_CODE=( 1 )\nNAIF_SURFACE_BODY=( 301 )", "SPICE(ARRAYSIZEMISMATCH)"),
        ("NAIF_SURFACE_NAME=( 'A' )\nNAIF_SURFACE_CODE=( 1 )\nNAIF_SURFACE_BODY=( 'bad' )", "SPICE(BADVARIABLETYPE)"),
    ];
    for (data, expected) in cases {
        let pool = Pck::from_text(&format!("KPL/FK\n\\begindata\n{data}\n\\begintext\n")).unwrap();
        let error = SurfaceNameResolver::new()
            .resolve(&pool, "A", 301)
            .unwrap_err();
        assert_eq!(error.to_string(), expected);
    }
    let pool = Pck::default();
    let mut resolver = SurfaceNameResolver::new();
    for (text, expected) in [
        ("1 0", Some(10)),
        ("+ 1", Some(1)),
        ("1.5", Some(1)),
        ("-.5", Some(0)),
        ("1d2", Some(100)),
        ("PI", Some(3)),
        ("1,0", Some(10)),
        ("1..0", None),
        ("2147483648", None),
    ] {
        assert_eq!(
            resolver.resolve(&pool, text, 301).unwrap(),
            expected,
            "{text}"
        );
    }
}

#[test]
fn surface_name_numeric_fallback_never_parses_kernel_dates() {
    let pool = Pck::default();
    let mut resolver = SurfaceNameResolver::new();
    assert_eq!(resolver.resolve(&pool, "@2000-JAN-1", 301).unwrap(), None);
    assert_eq!(
        resolver
            .resolve(&pool, "@2000-JAN-1/12:00:00", 301)
            .unwrap(),
        None
    );
}

#[test]
fn native_surface_cache_keeps_partial_code_writes_after_conversion_failure() {
    let mut pool=Pck::from_text("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME = ( 'A' 'B' )\nNAIF_SURFACE_CODE = ( 1 2 )\nNAIF_SURFACE_BODY = ( 301 301 )\n\\begintext\n").unwrap();
    let mut resolver = SurfaceNameResolver::new();
    assert_eq!(resolver.resolve(&pool, "A", 301).unwrap(), Some(1));
    pool.furnsh("KPL/FK\n\\begindata\nNAIF_SURFACE_CODE = ( 9 2147483648 )\n\\begintext\n")
        .unwrap();
    assert_eq!(
        resolver.resolve(&pool, "A", 301).unwrap_err().to_string(),
        "SPICE(INTOUTOFRANGE)"
    );
    assert_eq!(resolver.resolve(&pool, "A", 301).unwrap(), Some(9));
    assert_eq!(resolver.resolve(&pool, "B", 301).unwrap(), Some(2));
    assert_eq!(resolver.resolve(&pool, "C", 301).unwrap(), None);
}
#[test]
fn native_surface_cache_keeps_new_codes_after_blank_name_failure() {
    let mut pool=Pck::from_text("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME = ( 'A' 'B' )\nNAIF_SURFACE_CODE = ( 1 2 )\nNAIF_SURFACE_BODY = ( 301 301 )\n\\begintext\n").unwrap();
    let mut resolver = SurfaceNameResolver::new();
    assert_eq!(resolver.resolve(&pool, "A", 301).unwrap(), Some(1));
    pool.furnsh("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME = ( 'A' ' ' )\nNAIF_SURFACE_CODE = ( 9 10 )\n\\begintext\n").unwrap();
    assert_eq!(
        resolver.resolve(&pool, "A", 301).unwrap_err().to_string(),
        "SPICE(BLANKNAMEASSIGNED)"
    );
    assert_eq!(resolver.resolve(&pool, "A", 301).unwrap(), Some(9));
    assert_eq!(resolver.resolve(&pool, "B", 301).unwrap(), Some(10));
    assert_eq!(resolver.resolve(&pool, "C", 301).unwrap(), None);
}

#[test]
fn native_surface_cache_retries_failed_initialization() {
    for (data,expected) in [
        ("NAIF_SURFACE_NAME=( 'A' ' ' )\nNAIF_SURFACE_CODE=( 9 10 )\nNAIF_SURFACE_BODY=( 301 301 )", "SPICE(BLANKNAMEASSIGNED)"),
        ("NAIF_SURFACE_NAME=( 'A' 'B' )\nNAIF_SURFACE_CODE=( 9 2147483648 )\nNAIF_SURFACE_BODY=( 301 301 )", "SPICE(INTOUTOFRANGE)"),
    ] {
        let pool=Pck::from_text(&format!("KPL/FK\n\\begindata\n{data}\n\\begintext\n")).unwrap();
        let mut resolver=SurfaceNameResolver::new();
        for name in ["A","A","10"] {
            assert_eq!(resolver.resolve(&pool,name,301).unwrap_err().to_string(),expected);
        }
    }
}

#[test]
fn native_failed_rebuild_preserves_prior_hash_buckets() {
    let mut pool=Pck::from_text("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME=( 'A' 'B' )\nNAIF_SURFACE_CODE=( 1 2 )\nNAIF_SURFACE_BODY=( 301 301 )\n\\begintext\n").unwrap();
    let mut resolver = SurfaceNameResolver::new();
    assert_eq!(resolver.resolve(&pool, "A", 301).unwrap(), Some(1));
    pool.furnsh("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME=( 'C' ' ' )\nNAIF_SURFACE_CODE=( 9 10 )\n\\begintext\n").unwrap();
    assert_eq!(
        resolver.resolve(&pool, "A", 301).unwrap_err().to_string(),
        "SPICE(BLANKNAMEASSIGNED)"
    );
    assert_eq!(resolver.resolve(&pool, "A", 301).unwrap(), None);
    assert_eq!(resolver.resolve(&pool, "B", 301).unwrap(), Some(10));
    assert_eq!(resolver.resolve(&pool, "C", 301).unwrap(), None);
}

#[test]
fn native_surface_name_fixed_width_trailing_space_semantics() {
    let a = "A".repeat(36) + "Z";
    let b = "A".repeat(35) + " B";
    let c = "A".repeat(34) + "  B";
    let text=format!("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME=(\n '{a}'\n '{b}'\n '{c}' )\nNAIF_SURFACE_CODE=( 1 2 3 )\nNAIF_SURFACE_BODY=( 301 301 301 )\n\\begintext\n");
    let pool = Pck::from_text(&text).unwrap();
    let mut resolver = SurfaceNameResolver::new();
    for (query, expected) in [
        (a, Some(1)),
        (b, Some(2)),
        (c, None),
        ("A".repeat(36), Some(1)),
        ("A".repeat(35) + " ", Some(2)),
        ("A".repeat(34), Some(3)),
    ] {
        assert_eq!(
            resolver.resolve(&pool, &query, 301).unwrap(),
            expected,
            "{query}"
        );
    }
}

#[test]
fn native_surface_name_captured_static_sequence() {
    let mut pool = Pck::default();
    let mut resolver = SurfaceNameResolver::new();
    let mut assigned = false;
    let mut count = 0;
    for row in include_str!("../docs/review-2026-11-12/surface-names/surface-name-reference.tsv")
        .lines()
        .skip(1)
    {
        let f: Vec<_> = row.split('\t').collect();
        assert_eq!(f.len(), 4);
        if f[0] == "assigned" && !assigned {
            pool.furnsh("KPL/FK\n\\begindata\nNAIF_SURFACE_NAME=( 'Alpha  Surface' 'ALPHA SURFACE' 'Other' )\nNAIF_SURFACE_CODE=( 100 101 102 )\nNAIF_SURFACE_BODY=( 301 301 399 )\n\\begintext\n").unwrap();
            assigned = true;
        }
        let result = resolver.resolve(&pool, f[1], f[2].parse().unwrap());
        let actual = match result {
            Ok(Some(value)) => format!("I:{value}"),
            Ok(None) => "N".to_string(),
            Err(error) => format!("E:{error}"),
        };
        assert_eq!(actual, f[3], "phase={} name={:?} body={}", f[0], f[1], f[2]);
        count += 1;
    }
    assert!(assigned);
    assert_eq!(count, 17);
}

#[test]
fn native_surface_name_malformed_pool_cases() {
    let fixture =
        include_str!("../docs/review-2026-11-12/surface-names/surface-malformed-reference.tsv");
    let mut count = 0;
    let mut previous = "";
    let mut pool = Pck::default();
    let mut resolver = SurfaceNameResolver::new();
    for row in fixture.lines().skip(1) {
        let f: Vec<_> = row.split('\t').collect();
        assert_eq!(f.len(), 3);
        if f[0] != previous {
            let data=match f[0] {
                "none"=>"",
                "names_only"=>"NAIF_SURFACE_NAME=( 'A' )",
                "missing_body"=>"NAIF_SURFACE_NAME=( 'A' )\nNAIF_SURFACE_CODE=( 1 )",
                "wrong_lengths"=>"NAIF_SURFACE_NAME=( 'A' 'B' )\nNAIF_SURFACE_CODE=( 1 )\nNAIF_SURFACE_BODY=( 301 )",
                "float_codes"=>"NAIF_SURFACE_NAME=( 'A' )\nNAIF_SURFACE_CODE=( 1.5 )\nNAIF_SURFACE_BODY=( 301 )",
                "numeric_name"=>"NAIF_SURFACE_NAME=( '100' )\nNAIF_SURFACE_CODE=( 42 )\nNAIF_SURFACE_BODY=( 301 )",
                "dup_name"=>"NAIF_SURFACE_NAME=( 'A' 'A' )\nNAIF_SURFACE_CODE=( 1 2 )\nNAIF_SURFACE_BODY=( 301 301 )",
                "dup_code"=>"NAIF_SURFACE_NAME=( 'A' 'B' )\nNAIF_SURFACE_CODE=( 1 1 )\nNAIF_SURFACE_BODY=( 301 301 )",
                other=>panic!("unexpected native case {other}"),
            };
            pool = Pck::from_text(&format!("KPL/FK\n\\begindata\n{data}\n\\begintext\n")).unwrap();
            resolver = SurfaceNameResolver::new();
            previous = f[0];
        }
        let actual = match resolver.resolve(&pool, f[1], 301) {
            Ok(Some(value)) => format!("I:{value}"),
            Ok(None) => "N".to_string(),
            Err(error) => format!("E:{error}"),
        };
        assert_eq!(actual, f[2], "case={} name={}", f[0], f[1]);
        count += 1;
    }
    assert_eq!(count, 40);
}

#[test]
fn native_surface_numeric_fallback_corpus() {
    let pool = Pck::default();
    let mut resolver = SurfaceNameResolver::new();
    let mut count = 0;
    for row in include_str!("../docs/review-2026-11-12/surface-names/numeric-reference.tsv")
        .lines()
        .skip(1)
    {
        let (name, expected) = row.split_once('\t').unwrap();
        let actual = match resolver.resolve(&pool, name, 301) {
            Ok(Some(value)) => format!("I:{value}"),
            Ok(None) => "N".to_string(),
            Err(error) => format!("E:{error}"),
        };
        assert_eq!(actual, expected, "input {name:?}");
        count += 1;
    }
    assert_eq!(count, 522);
}

#[test]
fn native_surface_numeric_extreme_exponents() {
    let pool = Pck::default();
    let mut resolver = SurfaceNameResolver::new();
    let mut count = 0;
    for row in include_str!("../docs/review-2026-11-12/surface-names/numeric-extreme-reference.tsv")
        .lines()
        .skip(1)
    {
        let (name, expected) = row.split_once('\t').unwrap();
        let actual = match resolver.resolve(&pool, name, 301) {
            Ok(Some(value)) => format!("I:{value}"),
            Ok(None) => "N".to_string(),
            Err(error) => format!("E:{error}"),
        };
        assert_eq!(actual, expected, "input {name:?}");
        count += 1;
    }
    assert_eq!(count, 2019);
}

#[test]
fn native_surface_long_name_and_whitespace_corpus() {
    let pool = Pck::from_text(include_str!(
        "../docs/review-2026-11-12/surface-names/mapping-fuzz.tf"
    ))
    .unwrap();
    let mut resolver = SurfaceNameResolver::new();
    let mut count = 0;
    for row in include_str!("../docs/review-2026-11-12/surface-names/mapping-fuzz-reference.tsv")
        .lines()
        .skip(1)
    {
        let (name, expected) = row.split_once('\t').unwrap();
        let actual = match resolver.resolve(&pool, name, 301) {
            Ok(Some(value)) => format!("I:{value}"),
            Ok(None) => "N".to_string(),
            Err(error) => format!("E:{error}"),
        };
        assert_eq!(actual, expected, "input {name:?}");
        count += 1;
    }
    assert_eq!(count, 419);
}

#[test]
fn native_named_surfaces_select_latitudinal_geometry() {
    use stkcraft_geocalc::{
        parse_surface_method, surface_coordinates::latitudinal_surface_points_in, DskSet, Frames,
        SpkKernel, SurfaceModel,
    };
    let spk = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let mut pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    pck.furnsh("\\begindata\nNAIF_SURFACE_NAME=('Example Shape' 'Missing Surface')\nNAIF_SURFACE_CODE=(100 999)\nNAIF_SURFACE_BODY=(301 301)\n\\begintext\n").unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    let mut kernels = DskSet::new();
    kernels.load("tests/golden/dsk/optimized.bds").unwrap();
    let mut resolver = SurfaceNameResolver::new();
    let mut count = 0;
    for row in
        include_str!("../docs/review-2026-11-12/surface-names/named-latsrf-reference.txt").lines()
    {
        let f: Vec<_> = row.split('|').collect();
        let method = if f[0].is_empty() {
            "DSK/UNPRIORITIZED".to_string()
        } else {
            format!(
                "DSK/UNPRIORITIZED/SURFACES={}",
                f[0].split(';')
                    .map(|name| format!("\"{name}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let surfaces = parse_surface_method(&mut resolver, &pck, 301, &method, 100);
        let actual = match surfaces {
            Err(error) => format!("E|{error}"),
            Ok(parsed) => match latitudinal_surface_points_in(
                &spk,
                &pck,
                &frames,
                301,
                0.0,
                "DSK_LOCAL",
                &[[f[1].parse().unwrap(), f[2].parse().unwrap()]],
                SurfaceModel::DskUnprioritized {
                    kernels: &kernels,
                    surfaces: &parsed.surfaces,
                },
            ) {
                Ok(points) => format!(
                    "N|{}",
                    points[0]
                        .iter()
                        .map(|x| format!("{:016x}", x.to_bits()))
                        .collect::<Vec<_>>()
                        .join("|")
                ),
                Err(error) => {
                    let text = error.to_string();
                    let start = text.find("SPICE(").expect(&text);
                    let end = start + text[start..].find(')').unwrap();
                    format!("E|{}", &text[start..=end])
                }
            },
        };
        assert_eq!(actual, f[3..].join("|"), "{row}");
        count += 1;
    }
    assert_eq!(count, 30);
}

#[test]
fn native_latsrf_method_and_frame_validation() {
    use stkcraft_geocalc::{
        surface_coordinates::latitudinal_surface_points_with_method_in, DskSet, Frames, SpkKernel,
    };
    let spk = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let mut pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    pck.furnsh("\\begindata\nNAIF_SURFACE_NAME=('Example Shape' 'Missing Surface')\nNAIF_SURFACE_CODE=(100 999)\nNAIF_SURFACE_BODY=(301 301)\n\\begintext\n").unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    let mut kernels = DskSet::new();
    kernels.load("tests/golden/dsk/optimized.bds").unwrap();
    let mut resolver = SurfaceNameResolver::new();
    let mut count = 0;
    for row in include_str!("../docs/review-2026-11-12/surface-names/latsrf-method-reference.txt")
        .lines()
        .skip(1)
    {
        let fields: Vec<_> = row.split('|').collect();
        let result = latitudinal_surface_points_with_method_in(
            &spk,
            &pck,
            &frames,
            &kernels,
            &mut resolver,
            301,
            0.0,
            fields[1],
            &[[0.5, 0.2]],
            fields[0],
        );
        let actual = match result {
            Ok(points) => format!(
                "N|{}",
                points[0]
                    .iter()
                    .map(|x| format!("{:016x}", x.to_bits()))
                    .collect::<Vec<_>>()
                    .join("|")
            ),
            Err(error) => {
                let text = error.to_string();
                let start = text.find("SPICE(").expect(&text);
                let end = start + text[start..].find(')').unwrap();
                format!("E|{}", &text[start..=end])
            }
        };
        assert_eq!(actual, fields[2..].join("|"), "{row}");
        count += 1;
    }
    assert_eq!(count, 1065);
}

#[test]
fn native_latsrf_without_loaded_dsk() {
    use stkcraft_geocalc::{
        surface_coordinates::latitudinal_surface_points_with_method_in, DskSet, Frames, SpkKernel,
    };
    let spk = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    let kernels = DskSet::new();
    let mut resolver = SurfaceNameResolver::new();
    let mut count = 0;
    for row in include_str!("../docs/review-2026-11-12/surface-names/latsrf-no-dsk-reference.txt")
        .lines()
        .skip(1)
    {
        let fields: Vec<_> = row.split('|').collect();
        let error = latitudinal_surface_points_with_method_in(
            &spk,
            &pck,
            &frames,
            &kernels,
            &mut resolver,
            301,
            0.0,
            fields[1],
            &[[0.5, 0.2]],
            fields[0],
        )
        .unwrap_err()
        .to_string();
        let start = error.find("SPICE(").expect(&error);
        let end = start + error[start..].find(')').unwrap();
        assert_eq!(&error[start..=end], fields[2], "{row}");
        count += 1;
    }
    assert_eq!(count, 24);
}

#[test]
fn native_sincpt_method_and_error_precedence() {
    use stkcraft_geocalc::{
        surface::surface_intercept_geometry_with_method_in, Aberration, DskSet, Frames, SpkKernel,
    };
    let spk = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let mut pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
    pck.furnsh("\\begindata\nNAIF_SURFACE_NAME=('Example Shape' 'Missing Surface')\nNAIF_SURFACE_CODE=(100 999)\nNAIF_SURFACE_BODY=(301 301)\n\\begintext\n").unwrap();
    let frames = Frames::from_text(include_str!("golden/dsk/frames.tf")).unwrap();
    let mut kernels = DskSet::new();
    kernels.load("tests/golden/dsk/optimized.bds").unwrap();
    let mut resolver = SurfaceNameResolver::new();
    let mut count = 0;
    for row in include_str!("../docs/review-2026-11-12/surface-names/sincpt-method-reference.txt")
        .lines()
        .skip(1)
    {
        let fields: Vec<_> = row.split('|').collect();
        let direction = std::array::from_fn(|i| {
            f64::from_bits(u64::from_str_radix(fields[i + 3], 16).unwrap())
        });
        let result = surface_intercept_geometry_with_method_in(
            &spk,
            &pck,
            &frames,
            &kernels,
            &mut resolver,
            fields[1],
            301,
            399,
            0.0,
            Aberration::parse(fields[2]).unwrap(),
            direction,
            fields[0],
        );
        let actual = match result {
            Ok(Some(g)) => format!(
                "N|{}",
                [
                    g.point[0],
                    g.point[1],
                    g.point[2],
                    g.epoch,
                    g.surface_vector[0],
                    g.surface_vector[1],
                    g.surface_vector[2]
                ]
                .iter()
                .map(|x| format!("{:016x}", x.to_bits()))
                .collect::<Vec<_>>()
                .join("|")
            ),
            Ok(None) => "MISS".to_string(),
            Err(error) => {
                let text = error.to_string();
                let start = text.find("SPICE(").expect(&text);
                let end = start + text[start..].find(')').unwrap();
                format!("E|{}", &text[start..=end])
            }
        };
        assert_eq!(actual, fields[6..].join("|"), "{row}");
        count += 1;
    }
    assert_eq!(count, 756);
}

#[test]
fn native_sincpt_inertial_target_and_missing_radii() {
    use stkcraft_geocalc::{
        surface::surface_intercept_geometry_with_method_in, Aberration, DskSet, Frames, SpkKernel,
    };
    let spk = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let frames = Frames::new();
    let kernels = DskSet::new();
    let mut resolver = SurfaceNameResolver::new();
    let mut count = 0;
    for row in include_str!("../docs/review-2026-11-12/surface-names/sincpt-inertial-reference.txt")
        .lines()
        .skip(1)
    {
        let fields: Vec<_> = row.split('|').collect();
        let mut pck = Pck::default();
        if fields[3] == "1" {
            pck.furnsh("\\begindata\nBODY0_RADII=(1 1 1)\n\\begintext\n")
                .unwrap();
        }
        let direction = std::array::from_fn(|i| {
            f64::from_bits(u64::from_str_radix(fields[i + 4], 16).unwrap())
        });
        let result = surface_intercept_geometry_with_method_in(
            &spk,
            &pck,
            &frames,
            &kernels,
            &mut resolver,
            fields[1],
            fields[0].parse().unwrap(),
            399,
            0.0,
            Aberration::None,
            direction,
            fields[2],
        );
        let actual = match result {
            Ok(Some(g)) => format!(
                "N|{}",
                [
                    g.point[0],
                    g.point[1],
                    g.point[2],
                    g.epoch,
                    g.surface_vector[0],
                    g.surface_vector[1],
                    g.surface_vector[2]
                ]
                .iter()
                .map(|x| format!("{:016x}", x.to_bits()))
                .collect::<Vec<_>>()
                .join("|")
            ),
            Ok(None) => "MISS".to_string(),
            Err(error) => {
                let text = error.to_string();
                let start = text.find("SPICE(").expect(&text);
                let end = start + text[start..].find(')').unwrap();
                format!("E|{}", &text[start..=end])
            }
        };
        assert_eq!(actual, fields[7..].join("|"), "{row}");
        count += 1;
    }
    assert_eq!(count, 4);
}

#[test]
fn native_sincpt_rejects_invalid_radii() {
    use stkcraft_geocalc::{
        surface::surface_intercept_geometry_with_method_in, Aberration, DskSet, Frames, SpkKernel,
    };
    let spk = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
    let frames = Frames::new();
    let kernels = DskSet::new();
    let mut resolver = SurfaceNameResolver::new();
    let row = include_str!("../docs/review-2026-11-12/surface-names/sincpt-method-reference.txt")
        .lines()
        .nth(1)
        .unwrap();
    let fields: Vec<_> = row.split('|').collect();
    let direction =
        std::array::from_fn(|i| f64::from_bits(u64::from_str_radix(fields[i + 3], 16).unwrap()));
    let mut count = 0;
    for row in include_str!("../docs/review-2026-11-12/surface-names/sincpt-radii-reference.txt")
        .lines()
        .skip(1)
    {
        let (radii, expected) = row.split_once('|').unwrap();
        let mut pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        pck.furnsh(&format!(
            "\\begindata\nBODY301_RADII=({})\n\\begintext\n",
            radii.replace(';', " ")
        ))
        .unwrap();
        let error = surface_intercept_geometry_with_method_in(
            &spk,
            &pck,
            &frames,
            &kernels,
            &mut resolver,
            "IAU_MOON",
            301,
            399,
            0.0,
            Aberration::None,
            direction,
            "ELLIPSOID",
        )
        .unwrap_err()
        .to_string();
        let start = error.find("SPICE(").expect(&error);
        let end = start + error[start..].find(')').unwrap();
        assert_eq!(&error[start..=end], expected, "{row}");
        count += 1;
    }
    assert_eq!(count, 5);
}
