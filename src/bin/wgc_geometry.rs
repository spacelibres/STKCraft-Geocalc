//! Command-line bridge used by `tools/wgc2_geometry_compare.py`.
//!
//! Reads one query per line from stdin and prints one result line per
//! query, with full `f64` precision. The SPK path comes from
//! `STKCRAFT_SPK`, defaulting to `target/kernels/de442.bsp`.
//!
//! Modes:
//!
//! ```text
//! wgc-geometry state                          # target,observer,et,abcorr
//! wgc-geometry separation                     # observer,t1,t2,et,abcorr
//! wgc-geometry state_rep <pck|->              # target,observer,et,abcorr,frame,representation,azccw,elplsz
//! wgc-geometry frame_rep <fk|-> <pck|->    # f1,f2,et,orientation,f1f2f3,av,avaxes,units
//! wgc-geometry location <pck|->              # point targets and observers
//! wgc-geometry pointing_full <pck|-> <fk|-> [ik instrument] # full typed direction
//! # Full pointing CSV: kind,target,observer,et,correction,input_frame,output_frame,
//! # magnitude,representation,anti,ray_kind,x,y,z,axis,azccw,elplsz
//! # Surface modes accept a final target_frame field and an FK path list after PCK.
//! wgc-geometry frame <fk-path-or-dash>        # from,to,et
//! ```
//!
//! In `state_rep`, `frame` is `0` for J2000, `-1` for ECLIPJ2000, and a
//! positive NAIF id for `IAU_<body>`.

use std::io::{self, BufRead};
use std::path::PathBuf;
use stkcraft_geocalc::pointing::{
    pointing_calculation, PointingContext, PointingDirection, PointingMagnitude,
    PointingRepresentation, PointingRequest, PointingVectorCorrection,
};
use stkcraft_geocalc::tangent::CorrectionLocus;

use stkcraft_geocalc::{
    angular_separation_shaped, angular_size, osculating_elements_in, phase_angle,
    planetocentric_solar_longitude, planetodetic_to_rectangular, pointing_direction,
    state_between_in, state_columns, state_columns_with_pck, Aberration, AngularShape,
    AngularUnits, AngularVelocityRepresentation, Endpoint, FrameAxis, FrameLocus, FrameSource,
    Frames, GmSource, OrientationRepresentation, OutputFrame, Pck, PointLocation, RayDirection,
    SpkKernel, StateRepresentation, SubPointMethod,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("wgc-geometry: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    let mode = arguments.get(1).map(String::as_str).unwrap_or("");

    let spk_path =
        std::env::var("STKCRAFT_SPK").unwrap_or_else(|_| "target/kernels/de442.bsp".to_string());

    let stdin = io::stdin();
    match mode {
        "state" => {
            let kernel = load_spk(&spk_path)?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if fields.len() != 4 {
                    println!("ERROR,expected target,observer,et,abcorr");
                    continue;
                }
                let target = fields[0].parse::<i32>()?;
                let observer = fields[1].parse::<i32>()?;
                let et = fields[2].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[3])?;
                match kernel.state_corrected(target, observer, et, abcorr) {
                    Ok(corrected) => println!(
                        "{},{},{},{},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?}",
                        target,
                        observer,
                        et,
                        fields[3],
                        corrected.state.position[0],
                        corrected.state.position[1],
                        corrected.state.position[2],
                        corrected.state.velocity[0],
                        corrected.state.velocity[1],
                        corrected.state.velocity[2],
                        corrected.light_time,
                        norm(corrected.state.position),norm(corrected.state.velocity),stkcraft_geocalc::state_rep::time_at_target(et,corrected.light_time,abcorr),
                    ),
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "separation" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if fields.len() != 5 && fields.len() != 7 {
                    println!("ERROR,expected observer,target1,target2,et,abcorr[,shape1,shape2]");
                    continue;
                }
                let observer = fields[0].parse::<i32>()?;
                let target1 = fields[1].parse::<i32>()?;
                let target2 = fields[2].parse::<i32>()?;
                let et = fields[3].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[4])?;
                let (shape1, shape2) = if fields.len() == 7 {
                    let Some(shape1) = AngularShape::parse(fields[5]) else {
                        println!("ERROR,unknown shape {}", fields[5]);
                        continue;
                    };
                    let Some(shape2) = AngularShape::parse(fields[6]) else {
                        println!("ERROR,unknown shape {}", fields[6]);
                        continue;
                    };
                    (shape1, shape2)
                } else {
                    (AngularShape::Point, AngularShape::Point)
                };
                match angular_separation_shaped(
                    &kernel, &pck, observer, target1, target2, et, abcorr, shape1, shape2,
                ) {
                    Ok(radians) => println!(
                        "{observer},{target1},{target2},{et},{},{radians:?},{:?}",
                        fields[4],
                        radians.to_degrees(),
                    ),
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "phase" => {
            let kernel = load_spk(&spk_path)?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if fields.len() != 5 {
                    println!("ERROR,expected target,illuminator,observer,et,abcorr");
                    continue;
                }
                let target = fields[0].parse::<i32>()?;
                let illuminator = fields[1].parse::<i32>()?;
                let observer = fields[2].parse::<i32>()?;
                let et = fields[3].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[4])?;
                match phase_angle(&kernel, target, illuminator, observer, et, abcorr) {
                    Ok(radians) => println!(
                        "{target},{illuminator},{observer},{et},{},{radians:?},{:?}",
                        fields[4],
                        radians.to_degrees(),
                    ),
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "size" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if fields.len() != 4 && fields.len() != 5 {
                    println!("ERROR,expected observer,target,et,abcorr[,method]");
                    continue;
                }
                let observer = fields[0].parse::<i32>()?;
                let target = fields[1].parse::<i32>()?;
                let et = fields[2].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[3])?;
                let method = fields
                    .get(4)
                    .map(|text| {
                        stkcraft_geocalc::AngularSizeMethod::parse(text)
                            .ok_or("unknown angular size method")
                    })
                    .transpose()?;
                let result = match method {
                    Some(method) => stkcraft_geocalc::angular_size_with_method(
                        &kernel, &pck, observer, target, et, abcorr, method,
                    ),
                    None => angular_size(&kernel, &pck, observer, target, et, abcorr),
                };
                match result {
                    Ok(radians) => {
                        if method==Some(stkcraft_geocalc::AngularSizeMethod::LimbExtents) {
                            let [minimum,maximum]=stkcraft_geocalc::finder::angular_size_extents(&kernel,&pck,observer,target,et,abcorr)?;
                            println!("{observer},{target},{et},{},{minimum:?},{:?},{:?}",fields[3],minimum.to_degrees(),maximum.to_degrees());
                        }else{
                            println!("{observer},{target},{et},{},{radians:?},{:?}",fields[3],radians.to_degrees());
                        }
                    }
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "state_rep" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            let frames = Frames::new();
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if fields.len() != 8 {
                    println!(
                        "ERROR,expected target,observer,et,abcorr,frame,representation,azccw,elplsz"
                    );
                    continue;
                }
                let target = fields[0].parse::<i32>()?;
                let observer = fields[1].parse::<i32>()?;
                let et = fields[2].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[3])?;
                let frame_code = fields[4].parse::<i32>()?;
                let Some(parsed) = StateRepresentation::parse(fields[5]) else {
                    println!("ERROR,unknown state representation");
                    continue;
                };
                let representation = match parsed {
                    StateRepresentation::AzimuthElevation { .. } => {
                        StateRepresentation::azimuth_elevation(
                            fields[6].parse::<bool>()?,
                            fields[7].parse::<bool>()?,
                        )
                    }
                    other => other,
                };

                // CSPICE `spkezr` evaluates a non-inertial output frame at the
                // epoch of the target's state, that is at `TIME_AT_TARGET`.
                let ecliptic;
                let frame_source = if frame_code == 0 {
                    FrameSource::J2000
                } else if frame_code == -1 {
                    ecliptic = match frames.pxform("J2000", "ECLIPJ2000", et) {
                        Ok(value) => value,
                        Err(error) => {
                            println!("ERROR,{error}");
                            continue;
                        }
                    };
                    FrameSource::Fixed(&ecliptic)
                } else if frame_code > 0 {
                    FrameSource::BodyFixed {
                        pck: &pck,
                        body: frame_code,
                    }
                } else {
                    println!("ERROR,frame code must be 0, -1, or a NAIF body id");
                    continue;
                };

                let apparent = match kernel.apparent_state_in(
                    target,
                    observer,
                    et,
                    abcorr,
                    &frame_source,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        println!("ERROR,{error}");
                        continue;
                    }
                };
                let frame_state = apparent.state;

                let body = frame_code.max(0);
                let columns = if body > 0 && pck.radii(body).is_some() {
                    state_columns_with_pck(
                        et,
                        frame_state,
                        representation,
                        apparent.light_time,
                        abcorr,
                        &pck,
                        body,
                    )
                } else {
                    state_columns(
                        et,
                        frame_state,
                        representation,
                        apparent.light_time,
                        abcorr,
                        pck.radii(body),
                        body,
                    )
                };

                match columns {
                    Ok(columns) => {
                        let row: Vec<String> = columns
                            .numeric_row()
                            .iter()
                            .map(|value| format!("{value:?}"))
                            .collect();
                        println!("{},{},{},{}{}", target, observer, et, fields[3], comma(&row));
                    }
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "elements" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if !(4..=6).contains(&fields.len()) {
                    println!("ERROR,expected orbiting,center,et,abcorr[,gm|CENTER|CENTER_ORBITER[,frame]]");
                    continue;
                }
                let orbiting = fields[0].parse::<i32>()?;
                let center = fields[1].parse::<i32>()?;
                let et = fields[2].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[3])?;
                let source = match fields.get(4).copied().unwrap_or("CENTER") {
                    "CENTER" => GmSource::Center,
                    "CENTER_ORBITER" => GmSource::CenterOrbiter,
                    value => GmSource::Custom(value.parse::<f64>()?),
                };
                let frame = fields.get(5).copied().unwrap_or("J2000");
                let elements = osculating_elements_in(&kernel, &pck, &Frames::new(),
                    orbiting, center, et, abcorr, frame, source);
                match elements {
                    Ok(elements) => println!(
                        "{orbiting},{center},{et},{},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?}",
                        fields[3],
                        elements.perifocal_distance,
                        elements.eccentricity,
                        elements.inclination.to_degrees(),
                        elements.ascending_node.to_degrees(),
                        elements.argument_of_periapsis.to_degrees(),
                        elements.mean_anomaly.to_degrees(),
                        elements.true_anomaly.to_degrees(),
                        elements.semi_major_axis,
                        elements.period,
                        elements.range,
                        elements.speed,
                        elements.gm,
                    ),
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "subpoint" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            let frames=load_frames(arguments.get(3).map(String::as_str).unwrap_or("-"))?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if !(5..=7).contains(&fields.len()) {
                    println!("ERROR,expected kind,target,observer,et,abcorr[,method,target_frame]");
                    continue;
                }
                let kind = fields[0];
                if !matches!(kind, "solar" | "observer") {
                    println!("ERROR,unknown sub-point kind {kind}");
                    continue;
                }
                let target = fields[1].parse::<i32>()?;
                let observer = fields[2].parse::<i32>()?;
                let default_frame=format!("IAU_{target}");let target_frame=fields.get(6).copied().unwrap_or(&default_frame);
                let et = fields[3].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[4])?;
                let method = if fields.len() >= 6 {
                    match SubPointMethod::parse(fields[5]) {
                        Some(value) => value,
                        None => {
                            println!("ERROR,unknown sub-point method {}", fields[5]);
                            continue;
                        }
                    }
                } else {
                    SubPointMethod::NearPoint
                };
                let result = if kind == "solar" {
                    stkcraft_geocalc::sub_solar_geometry_in(&kernel, &pck, &frames,target_frame,target, observer, et, abcorr, method)
                } else {
                    stkcraft_geocalc::sub_observer_geometry_in(
                        &kernel, &pck, &frames,target_frame,target, observer, et, abcorr, method,
                    )
                };
                match result {
                    Ok(point) => {
                        let illumination=stkcraft_geocalc::illumination_geometry_in(&kernel,&pck,&frames,target_frame,target,10,observer,et,abcorr,point.point)?;
                        let lst=local_solar_time_with_frame(&kernel,&pck,target,point.epoch,point.point,Some((&frames,target_frame)))?;
                        println!(
                        "{kind},{target},{observer},{et},{},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{}",
                        fields[4], point.point[0], point.point[1], point.point[2], point.altitude,
                        point.epoch,point.light_time,point.surface_vector[0],point.surface_vector[1],point.surface_vector[2],
                        norm(point.point),illumination.angles.solar.to_degrees(),illumination.angles.emission.to_degrees(),illumination.angles.phase.to_degrees(),lst)
                    },
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "longitude" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if fields.len() != 3 {
                    println!("ERROR,expected body,et,abcorr");
                    continue;
                }
                let body = fields[0].parse::<i32>()?;
                let et = fields[1].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[2])?;
                match planetocentric_solar_longitude(&kernel, &pck, body, et, abcorr) {
                    Ok(radians) => println!(
                        "{body},{et},{},{:?}",
                        fields[2],
                        radians.to_degrees()
                    ),
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "illum" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            let frames=load_frames(arguments.get(3).map(String::as_str).unwrap_or("-"))?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if !(7..=9).contains(&fields.len()) {
                    println!("ERROR,expected target,observer,lon_deg,lat_deg,alt_km,et,abcorr[,illuminator,target_frame]");
                    continue;
                }
                let target = fields[0].parse::<i32>()?;
                let observer = fields[1].parse::<i32>()?;
                let default_frame=format!("IAU_{target}");let target_frame=fields.get(8).copied().unwrap_or(&default_frame);
                let longitude: f64 = fields[2].parse::<f64>()?;
                let latitude: f64 = fields[3].parse::<f64>()?;
                let altitude: f64 = fields[4].parse::<f64>()?;
                let et = fields[5].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[6])?;

                let radii = match pck.radii(target) {
                    Some(radii) => radii,
                    None => {
                        println!("ERROR,no PCK data for body {target}");
                        continue;
                    }
                };
                let point = match planetodetic_to_rectangular(
                    latitude.to_radians(),
                    longitude.to_radians(),
                    altitude,
                    radii,
                ) {
                    Ok(point) => point,
                    Err(error) => {
                        println!("ERROR,{error}");
                        continue;
                    }
                };
                let illuminator = if fields.len()>=8 { fields[7].parse::<i32>()? } else { 10 };
                match stkcraft_geocalc::illumination_geometry_in(
                    &kernel, &pck, &frames,target_frame,target, illuminator, observer, et, abcorr, point,
                ) {
                    Ok(geometry) => { let angles=geometry.angles;
                        let lst=local_solar_time_with_frame(&kernel,&pck,target,geometry.epoch,point,Some((&frames,target_frame)))?;
                        println!(
                        "{target},{observer},{longitude},{latitude},{altitude},{et},{},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{},{},{:?},{}",
                        fields[6],
                        angles.solar.to_degrees(),
                        angles.emission.to_degrees(),
                        angles.phase.to_degrees(),
                        geometry.epoch,geometry.light_time,geometry.surface_vector[0],geometry.surface_vector[1],geometry.surface_vector[2],geometry.visible,geometry.illuminated,norm(geometry.surface_vector),lst,
                    ) },
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "intercept_vec" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            let fk = arguments.get(3).map(String::as_str).unwrap_or("-");
            let frames=load_frames(fk)?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if !(8..=9).contains(&fields.len()) {
                    println!("ERROR,expected target,observer,et,abcorr,frame,dx,dy,dz[,target_frame]");
                    continue;
                }
                let target = fields[0].parse::<i32>()?;
                let observer = fields[1].parse::<i32>()?;
                let default_frame=format!("IAU_{target}");let target_frame=fields.get(8).copied().unwrap_or(&default_frame);
                let et = fields[2].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[3])?;
                let direction = RayDirection::VectorInFrame {
                    frame: fields[4].to_string(),
                    vector: [
                        fields[5].parse::<f64>()?,
                        fields[6].parse::<f64>()?,
                        fields[7].parse::<f64>()?,
                    ],
                    anti: false,
                    stellar_aberration: false,
                };
                let result = stkcraft_geocalc::direction::surface_intercept_points_in(
                    &kernel,
                    &pck,
                    None,
                    &frames,
                    target_frame,
                    target,
                    observer,
                    et,
                    abcorr,
                    &direction,
                );
                match result {
                    Ok(points) => match points.first() {
                        Some(point) => println!(
                            "{target},{observer},{et},{},{:?},{:?},{:?},{:?}",
                            fields[3], point.point[0], point.point[1], point.point[2], point.altitude
                        ),
                        None => println!("ERROR,no intercept"),
                    },
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "intercept" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            let frames=load_frames(arguments.get(3).map(String::as_str).unwrap_or("-"))?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if !(4..=5).contains(&fields.len()) {
                    println!("ERROR,expected target,observer,et,abcorr[,target_frame]");
                    continue;
                }
                let target = fields[0].parse::<i32>()?;
                let observer = fields[1].parse::<i32>()?;
                let default_frame=format!("IAU_{target}");let target_frame=fields.get(4).copied().unwrap_or(&default_frame);
                let et = fields[2].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[3])?;
                let direction=object_direction(&kernel,&pck,target,observer,et,abcorr)?;
                match stkcraft_geocalc::surface::surface_intercept_geometry_in(&kernel,&pck,&frames,target_frame,target,observer,et,abcorr,direction) {
                    Ok(geometry)=>{
                        let illumination=stkcraft_geocalc::illumination_geometry_in(&kernel,&pck,&frames,target_frame,target,10,observer,et,abcorr,geometry.point)?;
                        let lst=local_solar_time_with_frame(&kernel,&pck,target,geometry.epoch,geometry.point,Some((&frames,target_frame)))?;
                        println!("{target},{observer},{et},{},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{},{:?},{:?},{:?}",fields[3],
                            geometry.point[0],geometry.point[1],geometry.point[2],geometry.altitude,norm(geometry.point),
                            illumination.angles.solar.to_degrees(),illumination.angles.emission.to_degrees(),illumination.angles.phase.to_degrees(),
                            geometry.epoch,geometry.light_time,lst,geometry.surface_vector[0],geometry.surface_vector[1],geometry.surface_vector[2]);
                    }
                    Err(error)=>println!("ERROR,{error}"),
                }
            }
        }
        "tangent" => {
            let kernel = load_spk(&spk_path)?;
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            let frames=load_frames(arguments.get(3).map(String::as_str).unwrap_or("-"))?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if !(7..=9).contains(&fields.len()) {
                    println!("ERROR,expected target,observer,et,abcorr,dx,dy,dz[,correction_locus,target_frame]");
                    continue;
                }
                let target = fields[0].parse::<i32>()?;
                let observer = fields[1].parse::<i32>()?;
                let default_frame=format!("IAU_{target}");let target_frame=fields.get(8).copied().unwrap_or(&default_frame);
                let et = fields[2].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[3])?;
                let mut direction = [
                    fields[4].parse::<f64>()?,
                    fields[5].parse::<f64>()?,
                    fields[6].parse::<f64>()?,
                ];
                if direction == [0.0, 0.0, 0.0] {
                    direction = object_direction(&kernel,&pck,target,observer,et,abcorr)?;
                }
                let locus=if fields.len()>=8 {
                    match CorrectionLocus::parse(fields[7]) {Some(locus)=>locus,None=>{println!("ERROR,unknown correction locus");continue;}}
                } else {CorrectionLocus::TangentPoint};
                match stkcraft_geocalc::tangent_point_in(&kernel, &pck, &frames,target_frame,target, observer, et, abcorr, direction,locus) {
                    Ok(result) => {
                        let illumination=stkcraft_geocalc::illumination_geometry_in(&kernel,&pck,&frames,target_frame,target,10,observer,et,abcorr,result.surface_point)?;
                        let lst=local_solar_time_default_frame(&kernel,&pck,&frames,target,result.epoch,result.surface_point)?;
                        let geometry=if result.altitude==0.0 {"Ray intercepts surface"} else {"Ray above limb"};
                        let radii = pck.radii(target);
                        let tangent_coordinates = stkcraft_geocalc::to_coordinates(
                            result.point,
                            stkcraft_geocalc::CoordinateSystem::Planetographic,
                            radii,
                            target,
                        )
                        .map(|values| [values[0].to_degrees(), values[1].to_degrees(), values[2]])
                        .unwrap_or([f64::NAN; 3]);
                        let surface_coordinates = stkcraft_geocalc::to_coordinates(
                            result.surface_point,
                            stkcraft_geocalc::CoordinateSystem::Planetographic,
                            radii,
                            target,
                        )
                        .map(|values| [values[0].to_degrees(), values[1].to_degrees(), values[2]])
                        .unwrap_or([f64::NAN; 3]);
                        println!(
                            "{target},{observer},{et},{},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{},{:?},{:?},{:?},{}",
                            fields[3],
                            result.point[0],
                            result.point[1],
                            result.point[2],
                            result.altitude,
                            result.range,
                            result.surface_point[0],
                            result.surface_point[1],
                            result.surface_point[2],
                            result.epoch,
                            result.light_time,
                            tangent_coordinates[0],
                            tangent_coordinates[1],
                            tangent_coordinates[2],
                            surface_coordinates[0],
                            surface_coordinates[1],
                            surface_coordinates[2],
                            result.surface_vector[0],result.surface_vector[1],result.surface_vector[2],geometry,
                            illumination.angles.solar.to_degrees(),illumination.angles.emission.to_degrees(),illumination.angles.phase.to_degrees(),lst,
                        )
                    }
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "pointing_full" => {
            let kernel=load_spk(&spk_path)?;
            let pck=load_pck(arguments.get(2).map(String::as_str).unwrap_or("-"))?;
            let frames=load_frames(arguments.get(3).map(String::as_str).unwrap_or("-"))?;
            let fov=match arguments.get(4).filter(|p|p.as_str()!="-") {
                Some(path)=>Some(stkcraft_geocalc::Fov::open(path,arguments.get(5).ok_or("instrument ID required with IK")?.parse()?)?),None=>None,
            };
            let context=PointingContext {kernel:&kernel,frames:&frames,pck:Some(&pck),fov:fov.as_ref()};
            for line in stdin.lock().lines() {
                let line=line?;let f=split(&line);
                if f.len()!=17 {println!("ERROR,expected kind,target,observer,et,correction,input_frame,output_frame,magnitude,representation,anti,ray_kind,x,y,z,axis,azccw,elplsz");continue;}
                let target=f[1].parse()?;let observer=f[2].parse()?;let epoch=f[3].parse()?;let anti=f[9].parse()?;
                let vector=[f[11].parse()?,f[12].parse()?,f[13].parse()?];
                let direction=match f[0] {
                    "POSITION"=>PointingDirection::Position {target,observer,aberration:Aberration::parse(f[4])?,anti},
                    "VELOCITY"=>PointingDirection::Velocity {target,observer,input_frame:f[5].into(),aberration:Aberration::parse(f[4])?,anti},
                    "VECTOR"=> {
                        let ray=match f[10] {
                            "VECTOR"|"VECTOR_IN_REFERENCE_FRAME"=>RayDirection::VectorInFrame {frame:f[5].into(),vector,anti:false,stellar_aberration:false},
                            "AXIS"|"REFERENCE_FRAME_AXIS"=>RayDirection::FrameAxis {frame:f[5].into(),axis:FrameAxis::parse(f[14]).ok_or("unknown axis")?,anti:false},
                            "BORESIGHT"|"INSTRUMENT_BORESIGHT"=>RayDirection::InstrumentBoresight,
                            "FOV_BOUNDARY"|"INSTRUMENT_FOV_BOUNDARY_VECTORS"=>RayDirection::InstrumentFovBoundaryVectors,
                            "VECTOR_FOV"|"VECTOR_IN_INSTRUMENT_FOV"=>RayDirection::VectorInFov {vector},
                            _=>{println!("ERROR,unknown ray kind");continue;}
                        };
                        PointingDirection::Vector {direction:ray,observer,aberration:PointingVectorCorrection::parse(f[4]).ok_or("unknown vector correction")?,anti}
                    },
                    _=>{println!("ERROR,unknown pointing kind");continue;}
                };
                let mut representation=PointingRepresentation::parse(f[8]).ok_or("unknown pointing representation")?;
                if f[8]=="AZ_EL" {representation=PointingRepresentation::AzimuthElevation {az_ccw:f[15].parse()?,el_plus_z:f[16].parse()?};}
                let request=PointingRequest {epoch,direction,output_frame:f[6].into(),magnitude:PointingMagnitude::parse(f[7]).ok_or("unknown magnitude")?,representation};
                match pointing_calculation(&context,&request) {
                    Ok(results)=>for result in results {let row=result.columns.numeric_row();println!("{},{target},{observer},{epoch},{},{:?},{:?},{:?},{:?}",f[0],result.boundary_point_number.unwrap_or(0),result.frame_epoch,row[0],row[1],row[2]);},
                    Err(error)=>println!("ERROR,{error}"),
                }
            }
        }
        "pointing" => {
            let kernel = load_spk(&spk_path)?;
            let fk = arguments.get(2).map(String::as_str).unwrap_or("-");
            let frames = if fk == "-" { None } else { Some(load_frames(fk)?) };
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if fields.len() != 4 && fields.len() != 5 {
                    println!("ERROR,expected target,observer,et,abcorr[,frame]");
                    continue;
                }
                let target = fields[0].parse::<i32>()?;
                let observer = fields[1].parse::<i32>()?;
                let et = fields[2].parse::<f64>()?;
                let abcorr = Aberration::parse(fields[3])?;
                let frame = fields.get(4).copied().unwrap_or("J2000");
                match pointing_direction(
                    &kernel,
                    target,
                    observer,
                    et,
                    abcorr,
                    frames.as_ref(),
                    frame,
                ) {
                    Ok(direction) => println!(
                        "{target},{observer},{et},{},{:?},{:?},{:?}",
                        fields[3], direction[0], direction[1], direction[2]
                    ),
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "location" => {
            let pck_spec = arguments.get(2).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            let frames = Frames::new();
            let kernel = load_spk(&spk_path)?;

            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if fields.len() != 30 {
                    println!("ERROR,expected 30 fields");
                    continue;
                }

                let number = |index: usize| -> Result<f64, Box<dyn std::error::Error>> {
                    Ok(fields[index].parse::<f64>()?)
                };
                let read_position =
                    |base: usize| -> Result<[f64; 3], Box<dyn std::error::Error>> {
                        Ok([number(base)?, number(base + 1)?, number(base + 2)?])
                    };
                let make_location = |base: usize,
                                     kind: &str|
                 -> Result<Option<PointLocation>, Box<dyn std::error::Error>> {
                    if kind == "OBJECT" {
                        return Ok(None);
                    }
                    let center = fields[base + 2].parse::<i32>()?;
                    let frame = fields[base + 3].to_string();
                    let position = read_position(base + 4)?;
                    let velocity = read_position(base + 7)?;
                    let epoch = number(base + 10)?;
                    Ok(Some(if kind == "FIXED_POINT" {
                        PointLocation::fixed(center, frame, position)
                    } else {
                        PointLocation::constant_velocity(center, frame, position, velocity, epoch)
                    }))
                };

                let target_kind = fields[0];
                let observer_kind = fields[11];
                if !matches!(target_kind, "OBJECT" | "FIXED_POINT" | "CONSTANT_VELOCITY_POINT") {
                    println!("ERROR,unknown target location type {target_kind}");
                    continue;
                }
                if !matches!(observer_kind, "OBJECT" | "FIXED_POINT" | "CONSTANT_VELOCITY_POINT") {
                    println!("ERROR,unknown observer location type {observer_kind}");
                    continue;
                }
                if target_kind == "FIXED_POINT" && read_position(7)? != [0.0; 3] {
                    println!("ERROR,fixed target point must have zero velocity");
                    continue;
                }
                if observer_kind == "FIXED_POINT" && read_position(18)? != [0.0; 3] {
                    println!("ERROR,fixed observer point must have zero velocity");
                    continue;
                }
                let target_location = make_location(0, target_kind)?;
                let observer_location = make_location(11, observer_kind)?;

                let target_endpoint = match &target_location {
                    Some(location) => Endpoint::Point(location),
                    None => Endpoint::Object(fields[1].parse::<i32>()?),
                };
                let observer_endpoint = match &observer_location {
                    Some(location) => Endpoint::Point(location),
                    None => Endpoint::Object(fields[12].parse::<i32>()?),
                };

                let outref = fields[22];
                let frame_type = fields[23];
                if !matches!(frame_type, "SPICE_FRAME" | "SPICE" | "TOPOCENTRIC_AT_POINT") {
                    println!("ERROR,unknown frame type {frame_type}");
                    continue;
                }
                let Some(locus) = FrameLocus::parse(fields[24]) else {
                    println!("ERROR,unknown frame locus {}", fields[24]);
                    continue;
                };
                let abcorr = Aberration::parse(fields[25])?;
                let Some(representation) = StateRepresentation::parse(fields[26]) else {
                    println!("ERROR,unknown representation {}", fields[26]);
                    continue;
                };
                if !matches!(fields[27], "0" | "1") {
                    println!("ERROR,azccw must be 0 or 1");
                    continue;
                }
                if !matches!(fields[28], "0" | "1") {
                    println!("ERROR,elplsz must be 0 or 1");
                    continue;
                }
                let az_ccw = fields[27] == "1";
                let el_plus_z = fields[28] == "1";
                let et = number(29)?;

                let representation = match representation {
                    StateRepresentation::AzimuthElevation { .. } => {
                        StateRepresentation::azimuth_elevation(az_ccw, el_plus_z)
                    }
                    other => other,
                };

                let (frame, column_body) = if frame_type == "TOPOCENTRIC_AT_POINT" {
                    let Some(location) = &observer_location else {
                        println!("ERROR,topocentric frame needs a point observer");
                        continue;
                    };
                    (
                        OutputFrame::Topocentric(location),
                        observer_location.as_ref().map(|item| item.center).unwrap_or(0),
                    )
                } else {
                    let center = frames.center(outref)?;
                    (
                        OutputFrame::Spice(outref),
                        center,
                    )
                };

                let expressed = match state_between_in(
                    &kernel,
                    &frames,
                    &pck,
                    target_endpoint,
                    observer_endpoint,
                    frame,
                    locus,
                    et,
                    abcorr,
                ) {
                    Ok(corrected) => corrected,
                    Err(error) => {
                        println!("ERROR,{error}");
                        continue;
                    }
                };

                match state_columns_with_pck(
                    et,
                    expressed.state,
                    representation,
                    expressed.light_time,
                    abcorr,
                    &pck,
                    column_body,
                ) {
                    Ok(columns) => {
                        let mut header = String::from("COLS");
                        // `numeric_row` omits the date column, so the
                        // header does too.
                        for id in columns.representation.output_ids().into_iter().skip(1) {
                            header.push(',');
                            header.push_str(id);
                        }
                        println!("{header}");
                        let mut row = String::from("ROW");
                        for value in &columns.numeric_row() {
                            row.push_str(&format!(",{value:?}"));
                        }
                        println!("{row}");
                    }
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "frame_rep" => {
            let frame_kernel=load_spk(&spk_path)?;
            let fk = arguments.get(2).map(String::as_str).unwrap_or("-");
            let frames=load_frames(fk)?;
            let pck_spec = arguments.get(3).map(String::as_str).unwrap_or("-");
            let pck = load_pck(pck_spec)?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if !(8..=11).contains(&fields.len()) {
                    println!("ERROR,expected f1,f2,et,orientation,axes,av,avaxes,units[,abcorr,timeLocation,avUnits]");
                    continue;
                }
                let et = fields[2].parse::<f64>()?;
                let Some(mut orientation) = OrientationRepresentation::parse(fields[3]) else {
                    println!("ERROR,unknown orientation {}", fields[3]);
                    continue;
                };
                if let OrientationRepresentation::EulerAngles {
                    axis1,
                    axis2,
                    axis3,
                } = &mut orientation
                {
                    let Some(axes) = parse_axes(fields[4]) else {
                        println!("ERROR,expected three axes in {}", fields[4]);
                        continue;
                    };
                    *axis1 = axes.0;
                    *axis2 = axes.1;
                    *axis3 = axes.2;
                }
                let Some(mut angular_velocity) = AngularVelocityRepresentation::parse(fields[5])
                else {
                    println!("ERROR,unknown angular velocity {}", fields[5]);
                    continue;
                };
                if let AngularVelocityRepresentation::EulerAngleDerivatives {
                    axis1,
                    axis2,
                    axis3,
                } = &mut angular_velocity
                {
                    let Some(axes) = parse_axes(fields[6]) else {
                        println!("ERROR,expected three axes in {}", fields[6]);
                        continue;
                    };
                    *axis1 = axes.0;
                    *axis2 = axes.1;
                    *axis3 = axes.2;
                }
                let units = match fields[7].to_ascii_uppercase().as_str() {
                    "DEG" | "DEGREES" => AngularUnits::Degrees,
                    "RAD" | "RADIANS" => AngularUnits::Radians,
                    "RPM" => AngularUnits::RevolutionsPerMinute,
                    other => {
                        println!("ERROR,unknown units {other}");
                        continue;
                    }
                };

                let correction=Aberration::parse(fields.get(8).copied().unwrap_or("NONE"))?;
                let locus=stkcraft_geocalc::FrameTimeLocation::parse(fields.get(9).copied().unwrap_or("FRAME1")).ok_or("unknown frame time location")?;
                let default_velocity_units=match units {AngularUnits::Radians=>"rad/s",AngularUnits::Degrees=>"deg/s",AngularUnits::RevolutionsPerMinute=>"RPM"};
                let velocity_units=stkcraft_geocalc::AngularVelocityUnits::parse(fields.get(10).copied().unwrap_or(default_velocity_units)).ok_or("unknown angular velocity units")?;
                let rotation=match stkcraft_geocalc::frame_transformation(&frame_kernel,&frames,Some(&pck),fields[0],fields[1],et,correction,locus) {
                    Ok(value)=>value,
                    Err(error)=>{println!("ERROR,{error}");continue;}
                };
                match stkcraft_geocalc::describe_with_units(&rotation.transform.rotation,&rotation.transform.rotation_dot,orientation,angular_velocity,units,velocity_units) {
                    Ok(columns) => {
                        let corrected=correction!=Aberration::None;
                        let mut header = if corrected {String::from("COLS,DATE,DATE2,LIGHT_TIME")} else {String::from("COLS")};
                        for id in &columns.ids {
                            header.push(',');
                            header.push_str(id);
                        }
                        println!("{header}");
                        for (index,row) in columns.rows.iter().enumerate() {
                            let mut text = if corrected {
                                if index==0 {format!("ROW,{:?},{:?},{:?}",rotation.frame1_epoch,rotation.frame2_epoch,rotation.light_time)}else{String::from("ROW,,,")}
                            }else{String::from("ROW")};
                            for value in row {
                                text.push_str(&format!(",{value:?}"));
                            }
                            println!("{text}");
                        }
                    }
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        "frame" => {
            let fk = arguments.get(2).map(String::as_str).unwrap_or("-");
            let frames=load_frames(fk)?;
            for line in stdin.lock().lines() {
                let line = line?;
                let fields = split(&line);
                if fields.len() != 3 {
                    println!("ERROR,expected from,to,et");
                    continue;
                }
                let et = fields[2].parse::<f64>()?;
                match frames.pxform(fields[0], fields[1], et) {
                    Ok(matrix) => {
                        let mut values = String::new();
                        for row in matrix.iter() {
                            for value in row.iter() {
                                values.push_str(&format!(",{:?}", value));
                            }
                        }
                        println!("{},{},{et}{values}", fields[0], fields[1]);
                    }
                    Err(error) => println!("ERROR,{error}"),
                }
            }
        }
        _ => {
            return Err(
                "usage: wgc-geometry <state|separation|phase|size <pck|->|state_rep <pck|->|elements <pck|->|subpoint <pck|->|longitude <pck|->|illum <pck|->|intercept <pck|->|tangent <pck|->|pointing [<fk|->]|pointing_full <pck|-> <fk|-> [ik instrument]|frame <fk|->|frame_rep <fk|-> <pck|->|location <pck|->> < queries"
                    .into(),
            )
        }
    }

    Ok(())
}

fn parse_axes(text: &str) -> Option<(FrameAxis, FrameAxis, FrameAxis)> {
    let mut axes = text
        .trim()
        .chars()
        .map(|character| FrameAxis::parse(&character.to_string()));
    Some((axes.next()??, axes.next()??, axes.next()??))
}

fn load_spk(spec: &str) -> Result<SpkKernel, Box<dyn std::error::Error>> {
    let mut kernel = SpkKernel::open_all(&spec.split(',').map(str::trim).collect::<Vec<_>>())?;
    if let Ok(path) = std::env::var("STKCRAFT_LSK") {
        kernel.attach_lsk(stkcraft_time::LeapSecondTable::from_lsk(
            &std::fs::read_to_string(path)?,
        )?);
    }
    Ok(kernel)
}

fn load_frames(spec: &str) -> Result<Frames, Box<dyn std::error::Error>> {
    let mut frames = Frames::new();
    for path in spec
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty() && *p != "-")
    {
        frames.furnsh(&std::fs::read_to_string(path)?)?;
    }
    // Parameterized TWO-VECTOR definitions can evaluate ephemeris vectors.
    // Keep frame-only operations usable without an unneeded default SPK.
    if let Ok(spk) = std::env::var("STKCRAFT_SPK") {
        frames.attach_spk(std::sync::Arc::new(load_spk(&spk)?));
    } else if std::path::Path::new("target/kernels/de442.bsp").is_file() {
        frames.attach_spk(std::sync::Arc::new(load_spk("target/kernels/de442.bsp")?));
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
    Ok(frames)
}

fn load_pck(spec: &str) -> Result<Pck, Box<dyn std::error::Error>> {
    if spec == "-" {
        return Ok(Pck::default());
    }
    let paths: Vec<PathBuf> = spec
        .split(',')
        .map(|path| PathBuf::from(path.trim()))
        .collect();
    Ok(Pck::open_all(&paths)?)
}

fn split(line: &str) -> Vec<&str> {
    line.trim()
        .split(',')
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .collect()
}

fn comma(fields: &[String]) -> String {
    let mut text = String::new();
    for field in fields {
        text.push(',');
        text.push_str(field);
    }
    text
}

fn norm(v: [f64; 3]) -> f64 {
    let big = v[0].abs().max(v[1].abs()).max(v[2].abs());
    if big == 0.0 {
        return 0.0;
    }
    let (x, y, z) = (v[0] / big, v[1] / big, v[2] / big);
    big * (x * x + y * y + z * z).sqrt()
}

fn object_direction(
    kernel: &SpkKernel,
    pck: &Pck,
    target: i32,
    observer: i32,
    et: f64,
    aberration: Aberration,
) -> Result<[f64; 3], stkcraft_geocalc::GeoError> {
    let frames = Frames::new();
    let context = PointingContext {
        kernel,
        frames: &frames,
        pck: Some(pck),
        fov: None,
    };
    let request = PointingRequest {
        epoch: et,
        direction: PointingDirection::Position {
            target,
            observer,
            aberration,
            anti: false,
        },
        output_frame: "J2000".into(),
        magnitude: PointingMagnitude::PreserveOriginal,
        representation: PointingRepresentation::Rectangular,
    };
    Ok(pointing_calculation(&context, &request)?[0].columns.vector)
}

// Source-order ET2LST with an explicit IAU body frame (legacy oracle).
#[cfg(test)]
fn local_solar_time(
    kernel: &SpkKernel,
    pck: &Pck,
    body: i32,
    et: f64,
    point: [f64; 3],
) -> Result<String, Box<dyn std::error::Error>> {
    local_solar_time_with_frame(kernel, pck, body, et, point, None)
}

// WGC tangent metadata follows ET2LST/CIDFRM, passing its requested-frame
// longitude unchanged. OBJECT_<body>_FRAME associations therefore matter.
fn local_solar_time_default_frame(
    kernel: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    body: i32,
    et: f64,
    point: [f64; 3],
) -> Result<String, Box<dyn std::error::Error>> {
    let frame = frames.body_frame(body)?;
    local_solar_time_with_frame(kernel, pck, body, et, point, Some((frames, &frame)))
}
fn local_solar_time_with_frame(
    kernel: &SpkKernel,
    pck: &Pck,
    body: i32,
    et: f64,
    point: [f64; 3],
    frame: Option<(&Frames, &str)>,
) -> Result<String, Box<dyn std::error::Error>> {
    if body == 10 {
        return Ok("12:00:00".into());
    }
    let matrix = frame
        .map(|(frames, name)| frames.pxform_with_derivative("J2000", name, et, Some(pck)))
        .transpose()?;
    let source = match &matrix {
        Some(m) => FrameSource::FixedNonInertial {
            matrix: &m.rotation,
            center: body,
        },
        None => FrameSource::BodyFixed { pck, body },
    };
    let sun = kernel
        .apparent_state_in(10, body, et, Aberration::LightTimeStellar, &source)?
        .state
        .position;
    let lon = stkcraft_geocalc::to_coordinates(
        point,
        stkcraft_geocalc::CoordinateSystem::Latitudinal,
        None,
        body,
    )?[1];
    let slon = stkcraft_geocalc::to_coordinates(
        sun,
        stkcraft_geocalc::CoordinateSystem::Latitudinal,
        None,
        body,
    )?[1];
    let remainder = |num: f64, den: f64| {
        let mut q = (num / den).trunc();
        let mut r = num - q * den;
        if r < 0.0 {
            q -= 1.0;
            r += den;
        }
        (q, r)
    };
    let mut angle = remainder(lon - slon, std::f64::consts::TAU).1;
    if angle > std::f64::consts::PI {
        angle -= std::f64::consts::TAU;
    }
    if pck
        .prime_meridian_rate(body)
        .ok_or("missing PM spin rate")?
        < 0.0
    {
        angle = -angle;
    }
    let seconds = (angle * 86400.0 / std::f64::consts::TAU).clamp(-43200.0, 43200.0);
    let (hours, seconds) = remainder(seconds, 3600.0);
    let (minutes, seconds) = remainder(seconds, 60.0);
    let hours = (hours as i32 + 12) % 24;
    Ok(format!(
        "{:02}:{:02}:{:02}",
        hours, minutes as i32, seconds as i32
    ))
}

#[cfg(test)]
mod clock_tests {
    use super::*;
    #[test]
    fn tangent_clock_uses_cidfrm_default_frame() {
        let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
        let pck = Pck::open_all(&[
            "target/kernels/pck00011.tpc",
            "target/kernels/moon_pa_de440_200625.bpc",
        ])
        .unwrap();
        let mut frames = Frames::new();
        for path in [
            "target/kernels/moon_de440_250416.tf",
            "target/kernels/moon_assoc_me.tf",
        ] {
            frames
                .furnsh(&std::fs::read_to_string(path).unwrap())
                .unwrap();
        }
        let mut checked = 0;
        for line in include_str!("../../tests/golden/tangent_clock_default_frame.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let point = [
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
                f[3].parse().unwrap(),
            ];
            let actual = local_solar_time_default_frame(
                &kernel,
                &pck,
                &frames,
                301,
                f[0].parse().unwrap(),
                point,
            )
            .unwrap();
            assert_eq!(actual, f[4], "{line}");
            assert_eq!(f[4], f[5]);
            checked += 1;
        }
        assert_eq!(checked, 24);
    }
    #[test]
    fn earth_default_frame_clock_matches_cspice_and_nasa() {
        let kernel = SpkKernel::open("target/kernels/de442.bsp").unwrap();
        let pck = Pck::open_all(&[
            "target/kernels/pck00011.tpc",
            "target/kernels/earth_1962_260806_2126_combined.bpc",
            "target/kernels/earth_000101_270205_261110.bpc",
        ])
        .unwrap();
        let frames = load_frames("target/kernels/EARTHFIXEDITRF93.TF").unwrap();
        assert_eq!(frames.body_frame(399).unwrap(), "IAU_EARTH");
        let mut checked = 0;
        for line in include_str!("../../tests/golden/earth_tangent_clock_default_frame.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let point = [
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
                f[3].parse().unwrap(),
            ];
            let actual = local_solar_time_default_frame(
                &kernel,
                &pck,
                &frames,
                399,
                f[0].parse().unwrap(),
                point,
            )
            .unwrap();
            assert_eq!(actual, f[4], "{line}");
            assert_eq!(f[4], f[5]);
            checked += 1;
        }
        assert_eq!(checked, 24);
    }
    #[test]
    fn local_solar_metadata_matches_cspice_et2lst() {
        let kernel = SpkKernel::open("target/kernels/de442s.bsp").unwrap();
        let pck = Pck::open("target/kernels/pck00011.tpc").unwrap();
        let mut checked = 0;
        for line in include_str!("../../tests/golden/local_solar_time_exact.csv").lines() {
            if line.starts_with('#') || line.starts_with("body,") {
                continue;
            }
            let f: Vec<_> = line.split(',').collect();
            assert_eq!(f.len(), 6);
            let actual = local_solar_time(
                &kernel,
                &pck,
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                [
                    f[2].parse().unwrap(),
                    f[3].parse().unwrap(),
                    f[4].parse().unwrap(),
                ],
            )
            .unwrap();
            assert_eq!(actual, f[5], "{line}");
            checked += 1;
        }
        assert_eq!(checked, 36);
    }
}
