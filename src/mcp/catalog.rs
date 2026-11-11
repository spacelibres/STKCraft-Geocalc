//! Scientific argument schemas and mappings to the existing CSV contracts.
use serde_json::{json, Map, Value};
pub const GF_FAMILIES: &[&str] = &[
    "GF_COORDINATE_SEARCH",
    "GF_ANGULAR_SEPARATION_SEARCH",
    "GF_DISTANCE_SEARCH",
    "GF_SUB_POINT_SEARCH",
    "GF_OCCULTATION_SEARCH",
    "GF_SURFACE_INTERCEPT_POINT_SEARCH",
    "GF_TARGET_IN_INSTRUMENT_FOV_SEARCH",
    "GF_RAY_IN_FOV_SEARCH",
    "GF_RANGE_RATE_SEARCH",
    "GF_PHASE_ANGLE_SEARCH",
    "GF_ILLUMINATION_ANGLES_SEARCH",
];
#[derive(Clone)]
pub struct Spec {
    pub name: String,
    pub mode: String,
    pub fields: Vec<Field>,
    pub columns: Vec<String>,
}
#[derive(Clone)]
pub struct Field {
    pub name: String,
    pub kind: char,
    pub default: String,
    pub required: bool,
    pub fixed: bool,
}
fn fields(dsl: &str) -> Vec<Field> {
    dsl.split_whitespace()
        .map(|part| {
            let p: Vec<_> = part.splitn(3, ':').collect();
            let tag = p[1];
            Field {
                name: p[0].into(),
                kind: tag.chars().next().unwrap(),
                default: p.get(2).copied().unwrap_or("").into(),
                required: tag.contains('!'),
                fixed: tag.contains('='),
            }
        })
        .collect()
}
fn spec(name: &str, mode: &str, dsl: &str, columns: &str) -> Spec {
    Spec {
        name: name.into(),
        mode: mode.into(),
        fields: fields(dsl),
        columns: columns
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
    }
}
pub fn specifications() -> Vec<Spec> {
    let mut s=vec![
 spec("state_vector","state","target:i! observer:i! epoch_et:n! correction:s:NONE","target,observer,epoch_et,correction,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s,light_time_s,range_km,speed_km_s,time_at_target_et"),
 spec("angular_separation","separation","observer:i! target1:i! target2:i! epoch_et:n! correction:s:NONE shape1:s:POINT shape2:s:POINT","observer,target1,target2,epoch_et,correction,separation_rad,separation_deg"),
 spec("phase_angle","phase","target:i! illuminator:i:10 observer:i! epoch_et:n! correction:s:NONE","target,illuminator,observer,epoch_et,correction,phase_rad,phase_deg"),
 spec("angular_size","size","observer:i! target:i! epoch_et:n! correction:s:NONE method:s:MAX_RADIUS","observer,target,epoch_et,correction,angular_size_rad,angular_size_deg"),
 spec("osculating_elements","elements","target:i! center:i! epoch_et:n! correction:s:NONE gm_source:s:CENTER reference_frame:s:J2000","target,center,epoch_et,correction,perifocal_distance_km,eccentricity,inclination_deg,ascending_node_deg,argument_of_periapsis_deg,mean_anomaly_deg,true_anomaly_deg,semi_major_axis_km,period_s,range_km,speed_km_s,gm_km3_s2"),
 spec("sub_solar_point","subpoint","kind:s=:solar target:i! observer:i! epoch_et:n! correction:s:NONE method:s:NEAR_POINT/ELLIPSOID target_frame:s:@target_frame","kind,target,observer,epoch_et,correction,x_km,y_km,z_km,range_km,surface_epoch_et,light_time_s,surface_vector_x_km,surface_vector_y_km,surface_vector_z_km,radius_km,incidence_deg,emission_deg,phase_deg,local_solar_time"),
 spec("sub_observer_point","subpoint","kind:s=:observer target:i! observer:i! epoch_et:n! correction:s:NONE method:s:NEAR_POINT/ELLIPSOID target_frame:s:@target_frame","kind,target,observer,epoch_et,correction,x_km,y_km,z_km,range_km,surface_epoch_et,light_time_s,surface_vector_x_km,surface_vector_y_km,surface_vector_z_km,radius_km,incidence_deg,emission_deg,phase_deg,local_solar_time"),
 spec("planetocentric_solar_longitude","longitude","target:i! epoch_et:n! correction:s:NONE","target,epoch_et,correction,longitude_deg"),
 spec("illumination_angles","illum","target:i! observer:i! longitude_deg:n! latitude_deg:n! altitude_km:n:0 epoch_et:n! correction:s:NONE illuminator:i:10 target_frame:s:@target_frame","target,observer,longitude_deg,latitude_deg,altitude_km,epoch_et,correction,incidence_deg,emission_deg,phase_deg,surface_epoch_et,light_time_s,surface_vector_x_km,surface_vector_y_km,surface_vector_z_km,visible,illuminated,range_km,local_solar_time"),
 spec("surface_intercept_point","intercept_vec","target:i! observer:i! epoch_et:n! correction:s:NONE direction_frame:s:J2000 x:n! y:n! z:n! target_frame:s:@target_frame","target,observer,epoch_et,correction,x_km,y_km,z_km,range_km"),
 spec("tangent_point","tangent","target:i! observer:i! epoch_et:n! correction:s:NONE x:n:0 y:n:0 z:n:0 correction_locus:s:TANGENT_POINT target_frame:s:@target_frame","target,observer,epoch_et,correction,tangent_x_km,tangent_y_km,tangent_z_km,tangent_altitude_km,range_km,surface_x_km,surface_y_km,surface_z_km,surface_epoch_et,light_time_s,tangent_longitude_deg,tangent_latitude_deg,tangent_altitude_pgr_km,surface_longitude_deg,surface_latitude_deg,surface_altitude_pgr_km,surface_vector_x_km,surface_vector_y_km,surface_vector_z_km,geometry,incidence_deg,emission_deg,phase_deg,local_solar_time"),
 spec("pointing_direction","pointing_full","direction_type:s:POSITION target:i:0 observer:i! epoch_et:n! correction:s:NONE direction_frame:s:J2000 output_frame:s:J2000 magnitude:s:UNIT representation:s:RECTANGULAR anti:b:false vector_type:s:VECTOR x:n:0 y:n:0 z:n:0 axis:s:X azimuth_counterclockwise:b:true elevation_positive_z:b:true","direction_type,target,observer,epoch_et,boundary_point_number,frame_epoch_et,value1,value2,value3"),
 spec("frame_transformation","frame_rep","frame1:s! frame2:s! epoch_et:n! orientation:s:MATRIX_ALL_ONE_ROW axes:s:ZYZ angular_velocity:s:NOT_INCLUDED angular_velocity_axes:s:XYZ angular_units:s:rad correction:s:NONE time_location:s:FRAME1 angular_velocity_units:s:rad/s","")];
    for &family in GF_FAMILIES {
        let name = family.to_ascii_lowercase();
        let mut value=spec(&name,family,"id:s=:mcp family:s= target:i:0 observer:i! second:i:10 illuminator:i:10 reference_frame:s:J2000 target_frame:s:@target_frame method:s:NEAR_POINT/ELLIPSOID coordinate_system:s:RECTANGULAR coordinate:s:X angle:s:PHASE relation:s:> reference:n:0 upper:n:0 correction:s:NONE confinement:w! step_seconds:n! adjustment:n:0 complement:b:false interval_adjustment:s:NONE interval_amount:n:0 interval_units:s:SECONDS filter:b:false filter_threshold:n:0 filter_units:s:SECONDS duration_units:s:SECONDS x:n:0 y:n:0 z:n:0 shape1:s:POINT shape2:s:POINT occultation_type:s:ANY","start_et,end_et,duration");
        value.fields[1].default = family.into();
        value.fields[2].required = family != "GF_RAY_IN_FOV_SEARCH";
        if matches!(
            family,
            "GF_SURFACE_INTERCEPT_POINT_SEARCH"
                | "GF_RAY_IN_FOV_SEARCH"
                | "GF_ILLUMINATION_ANGLES_SEARCH"
        ) {
            for field in &mut value.fields {
                if matches!(field.name.as_str(), "x" | "y" | "z") {
                    field.required = true;
                }
            }
        }
        if family == "GF_OCCULTATION_SEARCH" {
            value.fields[30].default = "ELLIPSOID".into();
        }

        s.push(value);
    }
    s
}
pub fn kernel_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{
 "spk":{"type":"array","minItems":1,"items":{"type":"string"},"description":"SPK paths in load order; required by the CLI backends."},
 "pck":{"type":"array","items":{"type":"string"}},"fk":{"type":"array","items":{"type":"string"}},"ck":{"type":"array","items":{"type":"string"}},"lsk":{"type":"string"},"sclk":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["spacecraft_id","path"],"properties":{"spacecraft_id":{"type":"integer"},"path":{"type":"string"}}}},"ik":{"type":"string"},"instrument_id":{"type":"integer"}},"required":["spk"]})
}
fn field_schema(f: &Field) -> Value {
    let mut v = match f.kind {
        'i' => json!({"type":"integer","minimum":i32::MIN,"maximum":i32::MAX}),
        'n' => json!({"type":"number"}),
        'b' => json!({"type":"boolean"}),
        'w' => {
            json!({"type":"array","minItems":1,"items":{"type":"object","additionalProperties":false,"required":["start","end"],"properties":{"start":{"type":"number"},"end":{"type":"number"}}}})
        }
        _ => json!({"type":"string"}),
    };
    if f.name == "correction" {
        v["description"]=json!("SPICE correction; VECTOR pointing accepts NONE, LT, CN, XLT, XCN, S or XS. Other families use NONE/LT/LT+S/CN/CN+S/XLT/XLT+S/XCN/XCN+S as supported by that native routine.");
    }
    if f.name == "step_seconds" {
        v["exclusiveMinimum"] = json!(0);
    }
    if !f.required && !f.default.starts_with('@') {
        v["default"] = match f.kind {
            'i' => json!(f.default.parse::<i32>().unwrap_or(0)),
            'n' => json!(f.default.parse::<f64>().unwrap_or(0.)),
            'b' => json!(f.default == "true"),
            _ => json!(f.default),
        };
    }
    v
}
pub fn definitions() -> Vec<Value> {
    let mut tools = Vec::new();
    for spec in specifications() {
        let mut properties = Map::new();
        properties.insert("kernels".into(), kernel_schema());
        let mut required = vec![json!("kernels")];
        for f in spec.fields.iter().filter(|f| !f.fixed) {
            properties.insert(f.name.clone(), field_schema(f));
            if f.required {
                required.push(json!(f.name));
            }
        }
        tools.push(json!({"name":spec.name,"description":format!("{} through the existing {} backend. Epochs and GF confinement are ET seconds, distances km, GF angular references degrees. See geocalc://manual for the exact contract and supported subset.",spec.name,spec.mode),"inputSchema":{"type":"object","additionalProperties":false,"properties":properties,"required":required},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false},"outputSchema":output_schema()}));
    }
    tools.push(json!({"name":"calculate_csv","description":"Full allowlisted wgc_geometry/wgc_gf CSV bridge; no arbitrary executable, arguments, shell or environment. Returns exact raw CSV and typed table cells. See geocalc://manual for per-mode fields.","inputSchema":{"type":"object","additionalProperties":false,"required":["calculator","queries","kernels"],"properties":{"calculator":{"type":"string","enum":raw_modes()},"queries":{"type":"array","minItems":1,"maxItems":1,"items":{"type":"string"}},"kernels":kernel_schema()}},"outputSchema":output_schema(),"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}}));
    tools
}
fn output_schema() -> Value {
    json!({"type":"object","required":["calculator","backend","columns","rows","rawCsv"],"properties":{"calculator":{"type":"string"},"backend":{"type":"string"},"columns":{"type":"array","items":{"type":"string"}},"rows":{"type":"array","items":{"type":"array","items":{"type":["number","string","boolean","null"]}}},"rawCsv":{"type":"string"}}})
}
pub fn raw_modes() -> Vec<String> {
    let mut modes:Vec<String>="state separation phase size state_rep elements subpoint longitude illum intercept_vec intercept tangent pointing_full pointing location frame_rep frame".split_whitespace().map(str::to_string).collect();
    modes.extend(GF_FAMILIES.iter().map(|s| s.to_string()));
    modes
}
pub fn typed_query(spec: &Spec, args: &Value) -> Result<String, String> {
    let object = args.as_object().ok_or("arguments must be an object")?;
    for key in object.keys() {
        if key != "kernels" && !spec.fields.iter().any(|f| !f.fixed && &f.name == key) {
            return Err(format!("unknown argument {key}"));
        }
    }
    let mut row = Vec::new();
    for f in &spec.fields {
        let default = if f.default == "@target_frame" {
            format!(
                "IAU_{}",
                object.get("target").and_then(Value::as_i64).unwrap_or(0)
            )
        } else {
            f.default.clone()
        };
        let text = if f.fixed {
            default
        } else if let Some(value) = object.get(&f.name) {
            match f.kind {
                'i' => {
                    let n = value
                        .as_i64()
                        .ok_or_else(|| format!("{} must be an integer", f.name))?;
                    if n < i32::MIN as i64 || n > i32::MAX as i64 {
                        return Err(format!("{} exceeds NAIF integer range", f.name));
                    }
                    n.to_string()
                }
                'n' => {
                    let n = value
                        .as_f64()
                        .filter(|v| v.is_finite())
                        .ok_or_else(|| format!("{} must be a finite number", f.name))?;
                    if f.name == "step_seconds" && n <= 0. {
                        return Err("step_seconds must be positive".into());
                    }
                    format!("{n:?}")
                }
                'b' => value
                    .as_bool()
                    .ok_or_else(|| format!("{} must be boolean", f.name))?
                    .to_string(),
                'w' => {
                    let windows = value
                        .as_array()
                        .filter(|a| !a.is_empty())
                        .ok_or("confinement must be a non-empty array")?;
                    let mut packed = Vec::new();
                    for w in windows {
                        let w = w.as_object().ok_or("window must be an object")?;
                        if w.len() != 2 {
                            return Err("window requires only start and end".into());
                        }
                        let a = w
                            .get("start")
                            .and_then(Value::as_f64)
                            .filter(|v| v.is_finite())
                            .ok_or("invalid start ET")?;
                        let b = w
                            .get("end")
                            .and_then(Value::as_f64)
                            .filter(|v| v.is_finite())
                            .ok_or("invalid end ET")?;
                        if a > b {
                            return Err("window start exceeds end".into());
                        }
                        packed.push(format!("{a:?}:{b:?}"));
                    }
                    packed.join(";")
                }
                _ => value
                    .as_str()
                    .ok_or_else(|| format!("{} must be a string", f.name))?
                    .to_string(),
            }
        } else if f.required {
            return Err(format!("missing argument {}", f.name));
        } else {
            default
        };
        if text.contains([',', '\r', '\n']) {
            return Err(format!("{} contains a CSV delimiter", f.name));
        }
        row.push(if f.name == "correction_locus" {
            text.replace("TANGENT_POINT", "TANGENT POINT")
                .replace("SURFACE_POINT", "SURFACE POINT")
        } else {
            text
        });
    }
    Ok(row.join(","))
}
