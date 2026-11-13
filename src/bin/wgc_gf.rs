//! Exact GF bridge. One 33-field query yields BEGIN/id/count, ROW/id/index/ET/ET/duration, END/id.
//! Fields: id,family,target,observer,second,illuminator,frame,target_frame,method,
//! system,coordinate,angle,relation,lower,upper,correction,confinement,step,adjustment,
//! complement,window_adjustment,amount,amount_units,filter,threshold,threshold_units,
//! duration_units,x,y,z,shape1,shape2,occultation_type.
use std::io::{self, BufRead};
use stkcraft_geocalc::finder::*;
use stkcraft_geocalc::fov::{
    find_ray_in_fov_in, find_target_in_fov_shaped_in, Fov, FovTargetShape, RayAberration,
};
use stkcraft_geocalc::gf_constraints::*;
use stkcraft_geocalc::occult::{find_occultation, OccultShape, OccultationType};
use stkcraft_geocalc::{
    Aberration, CoordinateSystem, Frames, GeoError, Pck, RayDirection, SpkKernel, SubPointMethod,
};
fn main() {
    if let Err(error) = run() {
        eprintln!("wgc-gf: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--rotation-profile") {
        println!("{}", stkcraft_geocalc::rotation_math_profile());
        return Ok(());
    }
    let spk = std::env::var("STKCRAFT_SPK").unwrap_or_else(|_| "target/kernels/de442.bsp".into());
    let paths: Vec<_> = spk.split(',').collect();
    let mut kernel = SpkKernel::open_all(&paths)?;
    if let Ok(path) = std::env::var("STKCRAFT_LSK") {
        kernel.attach_lsk(stkcraft_time::LeapSecondTable::from_lsk(
            &std::fs::read_to_string(path)?,
        )?);
    }
    let kernel = std::sync::Arc::new(kernel);
    let pck = match args.get(1).map(String::as_str).unwrap_or("-") {
        "-" => Pck::default(),
        spec => Pck::open_all(&spec.split(',').collect::<Vec<_>>())?,
    };
    let mut frames = Frames::new();
    frames.attach_spk(std::sync::Arc::clone(&kernel));
    for path in args
        .get(2)
        .map(String::as_str)
        .unwrap_or("-")
        .split(',')
        .filter(|s| *s != "-")
    {
        frames.furnsh(&std::fs::read_to_string(path)?)?;
    }
    if let Ok(spec) = std::env::var("STKCRAFT_CK") {
        frames.attach_ck(stkcraft_geocalc::ck::CkKernel::open_all(
            &spec.split(',').collect::<Vec<_>>(),
        )?);
    }
    if let Ok(spec) = std::env::var("STKCRAFT_SCLK") {
        let lsk = stkcraft_time::LeapSecondTable::from_lsk(&std::fs::read_to_string(
            std::env::var("STKCRAFT_LSK").unwrap_or_else(|_| "target/kernels/naif0012.tls".into()),
        )?)?;
        for entry in spec.split(',') {
            let (id, path) = entry
                .split_once('|')
                .ok_or("SCLK specification must be spacecraft_id|path")?;
            frames.attach_sclk(
                stkcraft_time::SclkKernel::parse(&std::fs::read_to_string(path)?, id.parse()?)?,
                lsk.clone(),
            );
        }
    }
    let fov = match args.get(3).filter(|x| x.as_str() != "-") {
        Some(path) => Some(Fov::open(
            path,
            args.get(4).ok_or("IK requires instrument ID")?.parse()?,
        )?),
        None => None,
    };
    for line in io::stdin().lock().lines() {
        let line = line?;
        let f: Vec<_> = line.trim().split(',').collect();
        let id = f.first().copied().unwrap_or("");
        if f.len() != 33 {
            println!("ERROR,{id},expected33fields");
            continue;
        }
        match evaluate(&kernel, &pck, &frames, fov.as_ref(), &f) {
            Ok((windows, units)) => {
                let durations = window_durations(&windows, units)?;
                println!("BEGIN,{id},{}", windows.len());
                for (i, (w, d)) in windows.iter().zip(durations).enumerate() {
                    println!("ROW,{id},{i},{:?},{:?},{d:?}", w.start, w.end);
                }
                println!("END,{id}");
            }
            Err(error) => println!("ERROR,{id},{error}"),
        }
    }
    Ok(())
}
fn evaluate(
    k: &SpkKernel,
    p: &Pck,
    frames: &Frames,
    fov: Option<&Fov>,
    f: &[&str],
) -> Result<(Vec<Interval>, DurationUnits), Box<dyn std::error::Error>> {
    let target = f[2].parse()?;
    let observer = f[3].parse()?;
    let second = f[4].parse()?;
    let illuminator = f[5].parse()?;
    // Ray searches have their own NONE/S/XS correction grammar. The
    // general body-correction value is unused in that branch.
    let corr = if f[1] == "GF_RAY_IN_FOV_SEARCH" {
        Aberration::None
    } else {
        Aberration::parse(f[15])?
    };
    let system = CoordinateSystem::parse(f[9]).ok_or("unknown coordinate system")?;
    let coordinate = system.coordinate_index(f[10]).ok_or("unknown coordinate")?;
    let lo: f64 = f[13].parse()?;
    let hi: f64 = f[14].parse()?;
    let step = f[17].parse()?;
    let adjustment: f64 = f[18].parse()?;
    let cf = f[16]
        .split(';')
        .map(|s| {
            let (a, b) = s.split_once(':').ok_or("invalid confinement")?;
            Ok(Interval {
                start: a.parse()?,
                end: b.parse()?,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let point = [f[27].parse()?, f[28].parse()?, f[29].parse()?];
    let shape = |value: &str| match value {
        "POINT" => Ok(AngularShape::Point),
        "SPHERE" | "ELLIPSOID" => Ok(AngularShape::Sphere),
        _ => Err(GeoError::BadSearch("unsupported separation shape")),
    };
    let coord_angular = matches!(
        f[10],
        "LONGITUDE"
            | "LATITUDE"
            | "RIGHT ASCENSION"
            | "DECLINATION"
            | "COLATITUDE"
            | "AZIMUTH"
            | "ELEVATION"
    );
    let angular = matches!(
        f[1],
        "GF_ANGULAR_SEPARATION_SEARCH" | "GF_PHASE_ANGLE_SEARCH" | "GF_ILLUMINATION_ANGLES_SEARCH"
    ) || (matches!(
        f[1],
        "GF_COORDINATE_SEARCH" | "GF_SUB_POINT_SEARCH" | "GF_SURFACE_INTERCEPT_POINT_SEARCH"
    ) && coord_angular);
    let reference = |v: f64| if angular { v.to_radians() } else { v };
    let options = FinderOptions {
        step,
        tolerance: 1e-6,
        adjustment: reference(adjustment),
    };
    let native = |condition: Condition,
                  value: f64,
                  windows: &[Interval]|
     -> Result<Vec<Interval>, GeoError> {
        let v = reference(value);
        match f[1] {
            "GF_DISTANCE_SEARCH" => {
                find_distance(k, target, observer, corr, windows, condition, v, options)
            }
            "GF_RANGE_RATE_SEARCH" => {
                find_range_rate(k, target, observer, corr, windows, condition, v, options)
            }
            "GF_PHASE_ANGLE_SEARCH" => find_phase_angle(
                k,
                target,
                illuminator,
                observer,
                corr,
                windows,
                condition,
                v,
                options,
            ),
            "GF_ANGULAR_SEPARATION_SEARCH" => find_angular_separation(
                k,
                p,
                observer,
                target,
                second,
                corr,
                shape(f[30])?,
                shape(f[31])?,
                windows,
                condition,
                v,
                options,
            ),
            "GF_COORDINATE_SEARCH" => find_position_coordinate_with_radii(
                k, p, target, observer, corr, frames, f[6], system, coordinate, windows, condition,
                v, options,
            ),
            "GF_SUB_POINT_SEARCH" => find_sub_point_coordinate_in(
                k,
                p,
                frames,
                f[7],
                target,
                observer,
                corr,
                SubPointKind::Observer,
                SubPointMethod::parse(f[8])
                    .ok_or(GeoError::BadSearch("unknown subpoint method"))?,
                system,
                coordinate,
                windows,
                condition,
                v,
                options,
            ),
            "GF_ILLUMINATION_ANGLES_SEARCH" => find_illumination_angle_in(
                k,
                p,
                frames,
                f[7],
                target,
                illuminator,
                observer,
                corr,
                point,
                match f[11] {
                    "INCIDENCE" => IlluminationAngle::Incidence,
                    "EMISSION" => IlluminationAngle::Emission,
                    "PHASE" => IlluminationAngle::Phase,
                    _ => return Err(GeoError::BadSearch("unknown illumination angle")),
                },
                windows,
                condition,
                v,
                options,
            ),
            "GF_SURFACE_INTERCEPT_POINT_SEARCH" => find_surface_intercept_direction_coordinate_in(
                k,
                p,
                fov,
                frames,
                f[7],
                target,
                observer,
                corr,
                &RayDirection::VectorInFrame {
                    frame: f[6].into(),
                    vector: point,
                    anti: false,
                    stellar_aberration: false,
                },
                system,
                coordinate,
                windows,
                condition,
                v,
                options,
            ),
            _ => Err(GeoError::BadSearch("unknown scalar GF family")),
        }
    };
    let windows = match f[1] {
        "GF_OCCULTATION_SEARCH" => find_occultation(
            k,
            p,
            target,
            if f[30] == "POINT" {
                OccultShape::Point
            } else {
                OccultShape::Ellipsoid
            },
            second,
            if f[31] == "POINT" {
                OccultShape::Point
            } else {
                OccultShape::Ellipsoid
            },
            observer,
            corr,
            OccultationType::parse(f[32]).ok_or("unknown occultation type")?,
            &cf,
            options,
        )?,
        "GF_RAY_IN_FOV_SEARCH" => find_ray_in_fov_in(
            k,
            Some(p),
            fov.ok_or("FOV required")?,
            observer,
            RayAberration::parse(f[15])?,
            point,
            f[6],
            frames,
            &cf,
            options,
        )?,
        "GF_TARGET_IN_INSTRUMENT_FOV_SEARCH" => find_target_in_fov_shaped_in(
            k,
            p,
            fov.ok_or("FOV required")?,
            target,
            observer,
            corr,
            frames,
            &cf,
            options,
            if f[30] == "POINT" {
                FovTargetShape::Point
            } else {
                FovTargetShape::Ellipsoid
            },
            f[7],
        )?,
        _ => {
            let periodic = matches!(f[10], "LONGITUDE" | "RIGHT ASCENSION")
                && matches!(
                    f[1],
                    "GF_COORDINATE_SEARCH"
                        | "GF_SUB_POINT_SEARCH"
                        | "GF_SURFACE_INTERCEPT_POINT_SEARCH"
                );
            let domain = if periodic {
                ScalarDomain::Periodic {
                    origin: if matches!(
                        system,
                        CoordinateSystem::Cylindrical
                            | CoordinateSystem::RightAscensionDeclination
                            | CoordinateSystem::Planetographic
                    ) {
                        0.
                    } else {
                        -180.
                    },
                    period: 360.,
                    upper_inclusive: !matches!(
                        system,
                        CoordinateSystem::Cylindrical
                            | CoordinateSystem::RightAscensionDeclination
                            | CoordinateSystem::Planetographic
                    ),
                }
            } else {
                ScalarDomain::Linear
            };
            let constraint = if f[12] == "RANGE" {
                ScalarConstraint::Range {
                    lower: lo,
                    upper: hi,
                }
            } else {
                ScalarConstraint::Relation {
                    condition: Condition::parse(f[12]).ok_or("unknown relation")?,
                    reference: lo,
                }
            };
            apply_scalar_constraint(native, &cf, constraint, domain)?
        }
    };
    let duration =
        |amount: &str, units: &str| -> Result<WindowDuration, Box<dyn std::error::Error>> {
            Ok(WindowDuration {
                amount: amount.parse()?,
                units: DurationUnits::parse(units).ok_or("unknown duration units")?,
            })
        };
    let processing = WindowPostprocessing {
        complement: f[19].parse()?,
        adjustment: match f[20] {
            "NONE" => IntervalAdjustment::None,
            "EXPAND" => IntervalAdjustment::Expand(duration(f[21], f[22])?),
            "CONTRACT" => IntervalAdjustment::Contract(duration(f[21], f[22])?),
            _ => return Err("unknown interval adjustment".into()),
        },
        filtering: if f[23].parse()? {
            IntervalFiltering::Filter(duration(f[24], f[25])?)
        } else {
            IntervalFiltering::None
        },
        output_duration_units: DurationUnits::parse(f[26])
            .ok_or("unknown output duration units")?,
    };
    Ok((
        postprocess_windows(&windows, &cf, processing)?,
        processing.output_duration_units,
    ))
}
