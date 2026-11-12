//! Native frame metadata and fixed-offset kernel parameter lookup.
use super::{build_rotation, parse_f64, FrameDef, FrameRotation, Frames, Value, J2000_ID, ZERO};
use crate::error::GeoError;
use std::collections::HashMap;

fn fail(symbol: &'static str) -> GeoError {
    GeoError::BadSearch(symbol)
}

impl Frames {
    /// Source FRINFO: built-ins precede pool metadata; NAME is read first.
    pub(super) fn checked_definition(&self, def: &FrameDef) -> Result<FrameDef, GeoError> {
        if let Some(builtin) = crate::builtin_frames::FRAMES
            .iter()
            .find(|item| item.id == def.id)
        {
            return Ok(FrameDef {
                id: builtin.id,
                name: builtin.name.to_string(),
                class: builtin.class,
                class_id: builtin.class_id,
                center: builtin.center,
                relative: (builtin.class == 1 && builtin.id != J2000_ID).then_some(J2000_ID),
                rotation: builtin.rotation,
            });
        }
        let names = self
            .dynamic_pool
            .get(&format!("FRAME_{}_NAME", def.id))
            .and_then(Value::as_strings)
            .ok_or(fail("SPICE(UNKNOWNFRAME)"))?;
        if names.len() != 1 {
            return Err(fail("SPICE(INVALIDDIMENSION)"));
        }
        let mut checked = def.clone();
        checked.name = names[0].trim_start_matches(' ').chars().take(32).collect();
        let center = self
            .dynamic_value(&checked, "CENTER")
            .ok_or(fail("SPICE(KERNELVARNOTFOUND)"))?;
        if center.values().len() != 1 {
            return Err(fail("SPICE(BADVARIABLESIZE)"));
        }
        checked.center = if let Some(value) = center.as_i32() {
            value
        } else if let Some(name) = center.as_string() {
            self.body_id(&name)?
        } else {
            return Err(fail("SPICE(BADVARIABLETYPE)"));
        };
        checked.class = self.dynamic_numbers(&checked, "CLASS", 1)?[0].round() as i32;
        checked.class_id = self.dynamic_numbers(&checked, "CLASS_ID", 1)?[0].round() as i32;
        Ok(checked)
    }

    fn tk_value<'a>(
        &'a self,
        class_id: i32,
        name: &str,
        item: &str,
    ) -> Result<Option<&'a Value>, GeoError> {
        let id_key = format!("TKFRAME_{class_id}_{item}");
        let name_key = format!("TKFRAME_{name}_{item}");
        let by_id = (id_key.len() <= 32)
            .then(|| self.dynamic_pool.get(&id_key))
            .flatten();
        let by_name = (name_key.len() <= 32)
            .then(|| self.dynamic_pool.get(&name_key))
            .flatten();
        if by_id.is_some() && by_name.is_some() && id_key != name_key {
            return Err(fail("SPICE(COMPETINGFRAMESPEC)"));
        }
        Ok(by_id.or(by_name))
    }

    fn tk_checked<'a>(
        &'a self,
        class_id: i32,
        name: &str,
        item: &str,
        size: usize,
        character: bool,
    ) -> Result<&'a Value, GeoError> {
        let value = self
            .tk_value(class_id, name, item)?
            .ok_or(fail("SPICE(VARIABLENOTFOUND)"))?;
        // BADKPV checks dimension before type, unlike ZZDYNVA*.
        if value.values().len() != size {
            return Err(fail("SPICE(BADVARIABLESIZE)"));
        }
        if character != value.as_strings().is_some() {
            return Err(fail("SPICE(BADVARIABLETYPE)"));
        }
        Ok(value)
    }

    pub(super) fn fixed_kernel_step(
        &self,
        def: &FrameDef,
    ) -> Result<(i32, FrameRotation), GeoError> {
        let class_id = def.class_id;
        let name = if let Some(builtin) = crate::builtin_frames::FRAMES
            .iter()
            .find(|item| item.id == class_id)
        {
            builtin.name.to_string()
        } else {
            self.dynamic_pool
                .get(&format!("FRAME_{class_id}_NAME"))
                .and_then(Value::as_string)
                .ok_or(fail("SPICE(INCOMPLETEFRAME)"))?
        };
        let spec = self
            .tk_checked(class_id, &name, "SPEC", 1, true)?
            .as_string()
            .unwrap();
        let relative = self
            .tk_checked(class_id, &name, "RELATIVE", 1, true)?
            .as_string()
            .unwrap();
        let parent = self
            .names
            .get(&relative.trim().to_uppercase())
            .copied()
            .ok_or(fail("SPICE(BADFRAMESPEC)"))?;
        if parent == class_id {
            return Err(fail("SPICE(BADFRAMESPEC2)"));
        }
        let mut values = HashMap::new();
        values.insert(
            "SPEC".to_string(),
            self.tk_checked(class_id, &name, "SPEC", 1, true)?.clone(),
        );
        let fields: &[(&str, usize)] = match spec.to_uppercase().as_str() {
            "MATRIX" => &[("MATRIX", 9)],
            "ANGLES" => &[("ANGLES", 3), ("AXES", 3)],
            "QUATERNION" => &[("Q", 4)],
            _ => return Err(fail("SPICE(UNKNOWNFRAMESPEC)")),
        };
        for (field, count) in fields {
            values.insert(
                (*field).to_string(),
                self.tk_checked(class_id, &name, field, *count, false)?
                    .clone(),
            );
        }
        if spec.eq_ignore_ascii_case("ANGLES") {
            // TKFRAM uses GCPOOL(room=1), defaulting to radians when the
            // optional UNITS variable has no character-valued result.
            if let Some(units) = self
                .tk_value(class_id, &name, "UNITS")?
                .and_then(Value::as_strings)
                .and_then(|items| items.first())
            {
                values.insert("UNITS".into(), Value::character(units.clone()));
            } else {
                values.insert("UNITS".into(), Value::character("RADIANS".into()));
            }
        }
        let rotation = build_rotation(&class_id, &values)?;
        Ok((
            parent,
            FrameRotation {
                rotation,
                rotation_dot: ZERO,
            },
        ))
    }

    pub(super) fn dynamic_optional_string(
        &self,
        def: &FrameDef,
        item: &str,
    ) -> Result<Option<String>, GeoError> {
        match self.dynamic_value(def, item) {
            Some(_) => self.dynamic_string(def, item).map(Some),
            None => Ok(None),
        }
    }

    pub(super) fn dynamic_frame_id(&self, def: &FrameDef, item: &str) -> Result<i32, GeoError> {
        let value = self
            .dynamic_value(def, item)
            .ok_or(fail("SPICE(KERNELVARNOTFOUND)"))?;
        if value.values().len() != 1 {
            return Err(fail("SPICE(BADVARIABLESIZE)"));
        }
        if let Some(number) = value.as_number() {
            return Ok(parse_f64(number)?.round() as i32);
        }
        self.frame_id(&value.as_string().ok_or(fail("SPICE(BADVARIABLETYPE)"))?)
    }
}
