// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::excessive_precision, clippy::needless_range_loop)]
//! Text-kernel frame definitions and frame transformations.
//!
//! This module reads the fixed-rotation frames that a NAIF frame kernel
//! (FK) declares with the `TKFRAME` family of variables, and evaluates
//! `pxform`/`sxform` equivalent rotations between frames.
//!
//! # Supported
//!
//! * Frame declarations `FRAME_<id>_{NAME,CLASS,CLASS_ID,CENTER}` and
//!   `FRAME_<name> = <id>` aliases.
//! * `TKFRAME_<id>_{RELATIVE,SPEC}` with `SPEC` equal to `MATRIX`,
//!   `ANGLES` (plus `_AXES` and `_UNITS`), or `QUATERNION` (plus `_Q`).
//! * The built-in NAIF inertial-frame catalog and body-fixed frames
//!   backed by text-PCK orientation models and binary-PCK types 2, 3 and 20.
//!
//! * CK attitude frames with explicitly attached CK and spacecraft-clock data.
//! * Parameterized Euler, two-vector, product and Earth of-date frames,
//!   plus class-6 switch frames. Ephemeris-based definitions require an
//!   explicitly attached SPK context.
//!
//! Numeric epoch fields are supported. Kernel date literals and general
//! kernel-pool append semantics remain outside the verified frame scope.
//! See `docs/reference/DYNAMIC_FRAME_SCOPE.md` for the exact native matrix.
//!
//! # Conventions
//!
//! These were verified against CSPICE (`pxform`, `q2m`) rather than
//! assumed. `TKFRAME_<id>_MATRIX` is listed **column-major**; the Euler
//! angles compose as `[a1]_axis1 * [a2]_axis2 * [a3]_axis3`; and
//! `TKFRAME_<id>_Q` is `(q0, q1, q2, q3)` with the scalar part first.
//! The resulting matrix always maps this frame's coordinates into the
//! `RELATIVE` frame.

use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::path::Path;
use std::sync::Arc;
use stkcraft_time::{sclk::SclkKernel, LeapSecondTable};

use crate::error::GeoError;
pub(crate) use crate::kernels::text_kernel::{apply_assignments, Value};
use crate::pck::Pck;
#[path = "frame_body_defaults.rs"]
mod body_defaults;
#[path = "frame_body_names.rs"]
mod body_names;
#[path = "frame_kernel.rs"]
mod frame_kernel;
#[derive(Clone)]
struct EphemerisContext(Arc<crate::spk::SpkKernel>);
impl std::fmt::Debug for EphemerisContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EphemerisContext")
            .field("segments", &self.0.segments().len())
            .finish()
    }
}

/// A 3x3 rotation matrix in row-major order.
pub type Mat3 = [[f64; 3]; 3];

/// The identity rotation.
pub const IDENTITY: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// The zero matrix, for the derivative of a constant rotation.
const ZERO: Mat3 = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];

const J2000_ID: i32 = 1;
thread_local! {static DYNAMIC_DEPTH:std::cell::Cell<usize>=const {std::cell::Cell::new(0)};}
struct DynamicLevel;
impl DynamicLevel {
    fn enter() -> Result<Self, GeoError> {
        DYNAMIC_DEPTH.with(|depth| {
            if depth.get() >= 2 {
                Err(GeoError::BadSearch("SPICE(RECURSIONTOODEEP)"))
            } else {
                depth.set(depth.get() + 1);
                Ok(Self)
            }
        })
    }
}
impl Drop for DynamicLevel {
    fn drop(&mut self) {
        DYNAMIC_DEPTH.with(|depth| depth.set(depth.get() - 1));
    }
}
const FRAME_ATTRIBUTES: [&str; 4] = ["NAME", "CLASS", "CLASS_ID", "CENTER"];
const TK_ATTRIBUTES: [&str; 7] = ["RELATIVE", "SPEC", "MATRIX", "ANGLES", "AXES", "UNITS", "Q"];

/// A rotation and its time derivative between two frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameRotation {
    /// Rotation mapping `from` into `to`, as `pxform` returns it.
    pub rotation: Mat3,
    /// Time derivative of `rotation`, in inverse seconds.
    pub rotation_dot: Mat3,
}

/// One frame and its rotation into its `RELATIVE` frame.
#[derive(Clone, Debug)]
pub struct FrameDef {
    /// NAIF frame id.
    pub id: i32,
    /// Canonical frame name.
    pub name: String,
    /// NAIF frame class.
    pub class: i32,
    /// NAIF frame class id.
    pub class_id: i32,
    /// NAIF id of the frame center.
    pub center: i32,
    /// NAIF id of the frame this one is defined relative to.
    pub relative: Option<i32>,
    /// Rotation mapping this frame into `relative`, when known.
    pub rotation: Option<Mat3>,
}

/// A set of frame definitions, including the built-in inertial frames.
#[derive(Clone, Debug)]
pub struct Frames {
    frames: HashMap<i32, FrameDef>,
    names: HashMap<String, i32>,
    object_frames: HashMap<String, String>,
    ck: Option<Arc<crate::ck::CkKernel>>,
    clocks: HashMap<i32, Arc<SclkKernel>>,
    clock_lsk: Option<Arc<LeapSecondTable>>,
    ck_clock_ids: HashMap<i32, i32>,
    dynamic_pool: HashMap<String, Value>,
    ephemeris: Option<EphemerisContext>,
}

impl Default for Frames {
    fn default() -> Self {
        Self::new()
    }
}

impl Frames {
    /// Creates a frame set containing only the built-in frames.
    pub fn new() -> Self {
        let mut frames = HashMap::new();
        let mut names = HashMap::new();
        for builtin in crate::builtin_frames::FRAMES {
            names.insert(builtin.name.to_string(), builtin.id);
            frames.insert(
                builtin.id,
                FrameDef {
                    id: builtin.id,
                    name: builtin.name.to_string(),
                    class: builtin.class,
                    class_id: builtin.class_id,
                    center: builtin.center,
                    relative: if builtin.class == 1 && builtin.id != J2000_ID {
                        Some(J2000_ID)
                    } else {
                        None
                    },
                    rotation: builtin.rotation,
                },
            );
        }
        Self {
            frames,
            names,
            object_frames: HashMap::new(),
            ck: None,
            clocks: HashMap::new(),
            clock_lsk: None,
            ck_clock_ids: HashMap::new(),
            dynamic_pool: HashMap::new(),
            ephemeris: None,
        }
    }

    /// Attaches the ephemeris used by parameterized two-vector frames.
    pub fn attach_spk(&mut self, kernel: Arc<crate::spk::SpkKernel>) {
        self.ephemeris = Some(EphemerisContext(kernel));
    }

    /// Attaches encoded-tick CK data used by class-3 frame definitions.
    pub fn attach_ck(&mut self, kernel: crate::ck::CkKernel) {
        self.ck = Some(Arc::new(kernel));
    }
    /// Attaches a clock and the exact leap-second context used by its
    /// SCE2C/SCT2E conversions. No constant-rate ET approximation is used.
    pub fn attach_sclk(&mut self, clock: SclkKernel, lsk: LeapSecondTable) {
        self.clocks.insert(clock.spacecraft_id(), Arc::new(clock));
        self.clock_lsk = Some(Arc::new(lsk));
    }

    /// Opens a frame kernel file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GeoError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_text(&text)
    }

    /// Parses a frame kernel from text.
    pub fn from_text(text: &str) -> Result<Self, GeoError> {
        let mut frames = Self::new();
        frames.furnsh(text)?;
        Ok(frames)
    }

    /// Adds the frame definitions from a frame kernel.
    pub fn furnsh(&mut self, text: &str) -> Result<(), GeoError> {
        let data = extract_data_blocks(text);
        let parsing = apply_assignments(&data, &mut self.dynamic_pool);
        let assignments = self.dynamic_pool.clone();
        let builtins = Self::new();
        self.frames = builtins.frames;
        self.names = builtins.names;
        self.object_frames.clear();
        self.ck_clock_ids.clear();

        let mut raw: HashMap<i32, RawFrame> = HashMap::new();
        let mut aliases: HashMap<String, i32> = HashMap::new();

        for (variable, value) in assignments {
            if variable == "NAIF_BODY_NAME" || variable == "NAIF_BODY_CODE" {
                continue;
            }
            if let Some(id) = variable
                .strip_prefix("CK_")
                .and_then(|x| x.strip_suffix("_SCLK"))
                .and_then(|x| x.parse::<i32>().ok())
            {
                if let Some(clock) = value.as_i32() {
                    self.ck_clock_ids.insert(id, clock);
                }
                continue;
            }
            if let Some(object) = variable
                .strip_prefix("OBJECT_")
                .and_then(|v| v.strip_suffix("_FRAME"))
            {
                let frame = value.values().first().ok_or(GeoError::FrameParse(
                    "OBJECT_FRAME requires a frame name or ID",
                ))?;
                self.object_frames
                    .insert(object.to_string(), frame.to_string());
                continue;
            }
            if let Some(rest) = variable.strip_prefix("TKFRAME_") {
                if let Some((id_text, attribute)) = rest.split_once('_') {
                    if let Ok(id) = id_text.parse::<i32>() {
                        if TK_ATTRIBUTES.contains(&attribute) {
                            raw.entry(id)
                                .or_default()
                                .tk
                                .insert(attribute.to_string(), value.clone());
                            continue;
                        }
                    }
                }
                continue;
            }

            if let Some(rest) = variable.strip_prefix("FRAME_") {
                let mut handled = false;
                if let Some((first, attribute)) = rest.split_once('_') {
                    if let Ok(id) = first.parse::<i32>() {
                        if FRAME_ATTRIBUTES.contains(&attribute) {
                            raw.entry(id)
                                .or_default()
                                .frame
                                .insert(attribute.to_string(), value.clone());
                            handled = true;
                        }
                    }
                }
                if !handled {
                    if let Some(id) = value.as_i32() {
                        aliases.insert(rest.to_uppercase(), id);
                    }
                }
            }
        }

        for (id, raw) in raw {
            if crate::builtin_frames::FRAMES
                .iter()
                .any(|item| item.id == id)
            {
                continue;
            }
            let name = raw
                .frame
                .get("NAME")
                .and_then(Value::as_string)
                .unwrap_or_else(|| id.to_string());
            let class = raw.frame.get("CLASS").and_then(Value::as_i32).unwrap_or(0);
            let class_id = raw
                .frame
                .get("CLASS_ID")
                .and_then(Value::as_i32)
                .unwrap_or(id);
            let center = raw.frame.get("CENTER").and_then(Value::as_i32).unwrap_or(0);

            let relative = None;
            let rotation = None;

            self.names.insert(name.to_uppercase(), id);
            self.frames.insert(
                id,
                FrameDef {
                    id,
                    name,
                    class,
                    class_id,
                    center,
                    relative,
                    rotation,
                },
            );
        }

        for (alias, id) in aliases {
            self.names.entry(alias).or_insert(id);
        }
        let ids: Vec<_> = self.frames.keys().copied().collect();
        for id in &ids {
            if let Ok(checked) = self.checked_definition(&self.frames[id]) {
                self.names.insert(checked.name.to_uppercase(), *id);
                self.frames.insert(*id, checked);
            }
        }
        for id in ids {
            if self.frames[&id].class == 4 {
                if let Ok((parent, step)) = self.fixed_kernel_step(&self.frames[&id]) {
                    let def = self.frames.get_mut(&id).unwrap();
                    def.relative = Some(parent);
                    def.rotation = Some(step.rotation);
                }
            }
        }
        parsing
    }

    /// Looks up a frame by NAIF id.
    pub fn frame(&self, id: i32) -> Option<&FrameDef> {
        self.frames.get(&id)
    }

    /// Resolves a frame name (or numeric id) to its NAIF id.
    pub fn frame_id(&self, name: &str) -> Result<i32, GeoError> {
        if let Ok(id) = name.parse::<i32>() {
            return if self.frames.contains_key(&id) {
                Ok(id)
            } else {
                Err(GeoError::FrameNotFound(name.to_string()))
            };
        }
        if let Some(body) = iau_body_id(name) {
            if let Some(frame) = self
                .frames
                .values()
                .find(|frame| frame.class == 2 && frame.class_id == body)
            {
                return Ok(frame.id);
            }
        }
        self.names
            .get(&name.to_uppercase())
            .copied()
            .ok_or_else(|| GeoError::FrameNotFound(name.to_string()))
    }

    /// Resolves numeric IDs, native NAIF body aliases and FK body aliases.
    pub fn body_id(&self, name: &str) -> Result<i32, GeoError> {
        if let Ok(id) = name.trim().parse::<i32>() {
            return Ok(id);
        }
        let normalize = |s: &str| {
            s.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_uppercase()
        };
        let name = normalize(name);
        if let (Some(names), Some(codes)) = (
            self.dynamic_pool.get("NAIF_BODY_NAME"),
            self.dynamic_pool.get("NAIF_BODY_CODE"),
        ) {
            let names = names
                .as_strings()
                .ok_or(GeoError::BadSearch("SPICE(BADVARIABLETYPE)"))?;
            let codes = codes
                .as_list()
                .ok_or(GeoError::BadSearch("SPICE(BADVARIABLETYPE)"))?;
            if names.len() != codes.len() {
                return Err(GeoError::BadSearch("SPICE(BADDIMENSIONS)"));
            }
            for i in (0..names.len()).rev() {
                if normalize(&names[i]) == name {
                    return Ok(parse_f64(&codes[i])?.round() as i32);
                }
            }
        }
        body_names::NAMES
            .iter()
            .rev()
            .find(|(n, _)| n.eq_ignore_ascii_case(&name))
            .map(|x| x.1)
            .ok_or(GeoError::BadSearch("SPICE(NOTRANSLATION)"))
    }

    fn body_name(&self, body: i32) -> Result<Option<String>, GeoError> {
        if let (Some(names), Some(codes)) = (
            self.dynamic_pool.get("NAIF_BODY_NAME"),
            self.dynamic_pool.get("NAIF_BODY_CODE"),
        ) {
            let names = names
                .as_strings()
                .ok_or(GeoError::BadSearch("SPICE(BADVARIABLETYPE)"))?;
            let codes = codes
                .as_list()
                .ok_or(GeoError::BadSearch("SPICE(BADVARIABLETYPE)"))?;
            if names.len() != codes.len() {
                return Err(GeoError::BadSearch("SPICE(BADDIMENSIONS)"));
            }
            for i in (0..names.len()).rev() {
                if parse_f64(&codes[i])?.round() as i32 == body {
                    return Ok(Some(
                        names[i]
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_uppercase(),
                    ));
                }
            }
        }
        Ok(body_names::NAMES
            .iter()
            .rev()
            .find(|(_, id)| *id == body)
            .map(|(name, _)| name.to_uppercase()))
    }

    /// Default frame associated with a center body, following CIDFRM.
    /// Numeric `OBJECT_<id>_FRAME` overrides take priority over an override
    /// for the body's current preferred name, then the native default is used.
    pub fn body_frame(&self, body: i32) -> Result<String, GeoError> {
        let default = body_defaults::DEFAULTS.iter().find(|v| v.0 == body);
        let explicit = self.object_frames.get(&body.to_string());
        let preferred = if explicit.is_none() {
            self.body_name(body)?
        } else {
            None
        };
        let association = explicit.or_else(|| {
            preferred
                .as_ref()
                .and_then(|name| self.object_frames.get(name))
        });
        let id = if let Some(value) = association {
            self.frame_id(value)?
        } else {
            default
                .map(|v| v.2)
                .ok_or_else(|| GeoError::FrameNotFound(format!("default body frame for {body}")))?
        };
        self.frames
            .get(&id)
            .map(|f| f.name.clone())
            .ok_or_else(|| GeoError::FrameNotFound(id.to_string()))
    }

    /// Center body reported by NAIF FRINFO, including IAU body aliases.
    pub fn center(&self, name: &str) -> Result<i32, GeoError> {
        if let Ok(id) = self.frame_id(name) {
            return Ok(self.frames[&id].center);
        }
        iau_body_id(name).ok_or_else(|| GeoError::FrameNotFound(name.to_string()))
    }

    /// Whether this is a NAIF class-1 inertial frame. A TKFRAME has its
    /// declared class even when its orientation relative to J2000 is fixed.
    pub fn is_inertial(&self, name: &str) -> Result<bool, GeoError> {
        if let Ok(id) = self.frame_id(name) {
            return Ok(self.frames[&id].class == 1);
        }
        if iau_body_id(name).is_some() {
            return Ok(false);
        }
        Err(GeoError::FrameNotFound(name.to_string()))
    }

    /// Rotation mapping vectors from `from` into `to`.
    ///
    /// Mirrors CSPICE `pxform`: the returned matrix `m` satisfies
    /// `v_to = m * v_from`. Time-dependent frames are evaluated at `et`.
    /// This follows the rotation-only REFCHG path, which can differ in
    /// the final bit from the rotation block produced by SXFORM.
    pub fn pxform(&self, from: &str, to: &str, et: f64) -> Result<Mat3, GeoError> {
        Ok(self.transformation(from, to, et, None, false)?.rotation)
    }

    /// The 6x6 state transformation mapping `from` into `to`.
    ///
    /// Mirrors CSPICE `sxform`: the rotation appears in both the upper-left
    /// and lower-right 3x3 blocks.
    pub fn sxform(&self, from: &str, to: &str, et: f64) -> Result<[[f64; 6]; 6], GeoError> {
        let state = self.transformation(from, to, et, None, true)?;
        let rotation = state.rotation;
        let mut matrix = [[0.0f64; 6]; 6];
        for row in 0..3 {
            for column in 0..3 {
                matrix[row][column] = rotation[row][column];
                matrix[row + 3][column + 3] = rotation[row][column];
                matrix[row + 3][column] = state.rotation_dot[row][column];
            }
        }
        Ok(matrix)
    }

    /// Rotation and its time derivative between two frames.
    ///
    /// `from` and `to` are either frames declared in this frame set, the
    /// built-in inertial frames, or a body-fixed frame named
    /// `IAU_<naif-id>` (for example `IAU_301`), which needs `pck`.
    ///
    /// The derivative is analytic, not a finite difference: it is
    /// accumulated through the frame chain with the product rule, so a
    /// body-fixed frame contributes the rotation rate of its IAU
    /// orientation model. Static frames contribute nothing, and a chain of
    /// static frames therefore has a zero derivative.
    ///
    /// Matches CSPICE `sxform`, whose 6x6 state transformation is built
    /// from exactly these two matrices.
    pub fn pxform_with_derivative(
        &self,
        from: &str,
        to: &str,
        et: f64,
        pck: Option<&Pck>,
    ) -> Result<FrameRotation, GeoError> {
        self.transformation(from, to, et, pck, true)
    }

    /// Rotation-only REFCHG/PXFORM with an explicit PCK context. Unlike
    /// SXFORM, this does not require angular velocity from CK segments.
    pub fn pxform_in(
        &self,
        from: &str,
        to: &str,
        et: f64,
        pck: Option<&Pck>,
    ) -> Result<Mat3, GeoError> {
        Ok(self.transformation(from, to, et, pck, false)?.rotation)
    }

    fn transformation(
        &self,
        from: &str,
        to: &str,
        et: f64,
        pck: Option<&Pck>,
        need_av: bool,
    ) -> Result<FrameRotation, GeoError> {
        let (from_id, to_id) = match (self.frame_id(from), self.frame_id(to)) {
            (Ok(a), Ok(b)) => (a, b),
            _ => {
                let (a, ad) = self.j2000_orientation(from, et, pck)?;
                let (b, bd) = self.j2000_orientation(to, et, pck)?;
                return Ok(state_product(
                    FrameRotation {
                        rotation: transpose(&b),
                        rotation_dot: transpose(&bd),
                    },
                    FrameRotation {
                        rotation: a,
                        rotation_dot: ad,
                    },
                ));
            }
        };
        if from_id == to_id {
            return Ok(FrameRotation {
                rotation: IDENTITY,
                rotation_dot: ZERO,
            });
        }
        // FRMCHG retains at most ten from-side frame nodes. Additional
        // transforms are folded into its final buffered matrix.
        let mut nodes = vec![from_id];
        let mut steps: Vec<FrameRotation> = Vec::new();
        let mut current = from_id;
        let mut visited = HashSet::new();
        while current != J2000_ID && current != to_id {
            if !visited.insert(current) {
                return Err(GeoError::FrameParse("cyclic frame definitions"));
            }
            let Some((parent, step)) = self.state_step(current, et, pck, need_av)? else {
                break;
            };
            if nodes.len() < 10 {
                steps.push(step);
                nodes.push(parent);
            } else {
                let last = steps.last_mut().unwrap();
                *last = state_product(step, *last);
                nodes[9] = parent;
            }
            current = parent;
        }
        let fold = |list: &[FrameRotation]| {
            let mut iter = list.iter().copied();
            let mut result = iter.next().unwrap_or(FrameRotation {
                rotation: IDENTITY,
                rotation_dot: ZERO,
            });
            for step in iter {
                result = state_product(step, result);
            }
            result
        };
        if current == to_id {
            return Ok(fold(&steps));
        }
        current = to_id;
        visited.clear();
        let mut to_transform = None;
        while !nodes.contains(&current) {
            if !visited.insert(current) || current == J2000_ID {
                return Err(GeoError::BadSearch("SPICE(FRAMEDATANOTFOUND)"));
            }
            let Some((parent, step)) = self.state_step(current, et, pck, need_av)? else {
                return Err(GeoError::BadSearch("SPICE(FRAMEDATANOTFOUND)"));
            };
            to_transform = Some(match to_transform {
                None => step,
                Some(value) => state_product(step, value),
            });
            current = parent;
        }
        let index = nodes.iter().position(|id| *id == current).unwrap();
        let Some(to_transform) = to_transform else {
            return Ok(fold(&steps[..index]));
        };
        let inverse = FrameRotation {
            rotation: transpose(&to_transform.rotation),
            rotation_dot: transpose(&to_transform.rotation_dot),
        };
        if index == 0 {
            Ok(inverse)
        } else {
            Ok(state_product(inverse, fold(&steps[..index])))
        }
    }

    fn state_step(
        &self,
        id: i32,
        et: f64,
        pck: Option<&Pck>,
        need_av: bool,
    ) -> Result<Option<(i32, FrameRotation)>, GeoError> {
        let Some(original) = self.frames.get(&id) else {
            return Err(GeoError::BadSearch("SPICE(UNKNOWNFRAME)"));
        };
        let checked = self.checked_definition(original)?;
        let def = &checked;
        if !(1..=6).contains(&def.class) {
            return Err(GeoError::BadSearch("SPICE(UNKNOWNFRAMETYPE)"));
        }
        if def.class == 4 {
            return self.fixed_kernel_step(def).map(Some);
        }
        if def.class == 5 {
            return Ok(Some(self.dynamic_step(def, et, pck, need_av)?));
        }
        if def.class == 6 {
            return self.switch_step(def, et, pck, need_av);
        }
        if def.class == 3 {
            let kernel = self
                .ck
                .as_ref()
                .ok_or_else(|| GeoError::FrameNotFound(format!("CK data for {}", def.name)))?;
            let clock_id =
                self.ck_clock_ids
                    .get(&def.class_id)
                    .copied()
                    .unwrap_or(if def.class_id <= -1000 {
                        def.class_id / 1000
                    } else {
                        0
                    });
            let clock = self
                .clocks
                .get(&clock_id)
                .ok_or_else(|| GeoError::FrameNotFound(format!("spacecraft clock {clock_id}")))?;
            let lsk = self.clock_lsk.as_ref().ok_or_else(|| {
                GeoError::FrameNotFound("spacecraft-clock leap-second context".into())
            })?;
            let ticks = clock.et_to_ticks(et, lsk)?;
            let Some(pointing) = kernel.pointing(def.class_id, ticks, 0.0, need_av)? else {
                return Ok(None);
            };
            let rotation = pointing.rotation;
            let derivative = if let Some(av) = pointing.angular_velocity {
                // RAV2XF then INVSTM: transpose each nonzero state block.
                let omega = [
                    [0.0, av[2], -av[1]],
                    [-av[2], 0.0, av[0]],
                    [av[1], -av[0], 0.0],
                ];
                mat_mul(&rotation, &omega)
            } else {
                ZERO
            };
            return Ok(Some((
                pointing.reference_frame,
                FrameRotation {
                    rotation: transpose(&rotation),
                    rotation_dot: transpose(&derivative),
                },
            )));
        }
        if def.class == 2 {
            if pck.is_none() {
                return Ok(None);
            }
            if let Some(pck) = pck {
                if let Some((reference, rotation, derivative)) =
                    pck.binary_state_rotation(def.class_id, et)?
                {
                    // FRMGET class2 always calls TISBOD("J2000"). Even
                    // when the requested frame is the segment reference,
                    // keep this conversion before the outer frame chain.
                    let (rotation, derivative) = if reference == J2000_ID {
                        (rotation, derivative)
                    } else {
                        let change = self.pxform("J2000", &reference.to_string(), et)?;
                        (mat_mul(&rotation, &change), mat_mul(&derivative, &change))
                    };
                    return Ok(Some((
                        J2000_ID,
                        FrameRotation {
                            rotation: transpose(&rotation),
                            rotation_dot: transpose(&derivative),
                        },
                    )));
                }
            }
            let (rotation, rotation_dot) =
                body_j2000_orientation(def.class_id, &def.name, et, pck)?;
            return Ok(Some((
                J2000_ID,
                FrameRotation {
                    rotation,
                    rotation_dot,
                },
            )));
        }
        let rotation = def
            .rotation
            .ok_or_else(|| GeoError::UnsupportedFrameSpec(def.name.clone()))?;
        Ok(Some((
            def.relative.unwrap_or(J2000_ID),
            FrameRotation {
                rotation,
                rotation_dot: ZERO,
            },
        )))
    }

    fn switch_step(
        &self,
        def: &FrameDef,
        et: f64,
        pck: Option<&Pck>,
        need_av: bool,
    ) -> Result<Option<(i32, FrameRotation)>, GeoError> {
        if def.class_id != def.id {
            return Err(GeoError::BadSearch("SPICE(BADFRAMESPEC)"));
        }
        let aligned = self
            .dynamic_pool
            .get(&format!("FRAME_{}_ALIGNED_WITH", def.id))
            .ok_or(GeoError::BadSearch("SPICE(MISSINGFRAMEVAR)"))?;
        let numeric = aligned.as_list().is_some();
        let bases = aligned.values();
        if bases.len() > 15000 {
            return Err(GeoError::BadSearch("SPICE(TOOMANYBASEFRAMES)"));
        }
        let start = self.dynamic_pool.get(&format!("FRAME_{}_START", def.id));
        let stop = self.dynamic_pool.get(&format!("FRAME_{}_STOP", def.id));
        let times = match (start, stop) {
            (None, None) => None,
            (Some(_), None) | (None, Some(_)) => {
                return Err(GeoError::BadSearch("SPICE(PARTIALFRAMESPEC)"))
            }
            (Some(a), Some(b)) => {
                let numbers = |v: &Value| -> Result<Vec<f64>, GeoError> {
                    let v = v
                        .as_list()
                        .ok_or(GeoError::BadSearch("SPICE(BADVARIABLETYPE)"))?;
                    v.iter().map(|x| parse_f64(x)).collect()
                };
                let a = numbers(a)?;
                let b = numbers(b)?;
                if a.len() != bases.len() || b.len() != bases.len() {
                    return Err(GeoError::BadSearch("SPICE(COUNTMISMATCH)"));
                }
                if a.iter().zip(&b).any(|(a, b)| a > b) {
                    return Err(GeoError::BadSearch("SPICE(BADTIMEBOUNDS)"));
                }
                Some((a, b))
            }
        };
        // Resolve and validate every base before beginning selection, as
        // ZZSWFFET does even for entries outside the request interval.
        let mut ids = Vec::with_capacity(bases.len());
        for name in bases {
            let numeric_name;
            let name = if numeric {
                numeric_name = (parse_f64(name)?.round() as i32).to_string();
                &numeric_name
            } else {
                name
            };
            let id = self.frame_id(name).or_else(|_| {
                parse_f64(name)
                    .ok()
                    .map(|x| x.round() as i32)
                    .filter(|id| self.frames.contains_key(id))
                    .ok_or_else(|| GeoError::FrameNotFound(name.clone()))
            })?;
            ids.push(id);
        }
        for i in (0..ids.len()).rev() {
            if let Some((starts, stops)) = &times {
                if et < starts[i] || et > stops[i] {
                    continue;
                }
            }
            let id = ids[i];
            if matches!(self.frames[&id].class, 5 | 6) {
                return Ok(Some((
                    id,
                    FrameRotation {
                        rotation: IDENTITY,
                        rotation_dot: ZERO,
                    },
                )));
            }
            if let Some(result) = self.state_step(id, et, pck, need_av)? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    // ZZDYNOA*: numeric ID spelling takes precedence over name spelling.
    fn dynamic_value(&self, def: &FrameDef, item: &str) -> Option<&Value> {
        let id = format!("FRAME_{}_{}", def.id, item);
        let name = format!("FRAME_{}_{}", def.name, item);
        if id.len() <= 32 {
            if let Some(v) = self.dynamic_pool.get(&id) {
                return Some(v);
            }
        }
        if name.len() <= 32 {
            self.dynamic_pool.get(&name)
        } else {
            None
        }
    }
    fn dynamic_string(&self, def: &FrameDef, item: &str) -> Result<String, GeoError> {
        let value = self
            .dynamic_value(def, item)
            .ok_or(GeoError::BadSearch("SPICE(KERNELVARNOTFOUND)"))?;
        let strings = value
            .as_strings()
            .ok_or(GeoError::BadSearch("SPICE(BADVARIABLETYPE)"))?;
        if strings.len() != 1 {
            return Err(GeoError::BadSearch("SPICE(BADVARIABLESIZE)"));
        }
        Ok(strings[0].clone())
    }
    fn dynamic_numbers(
        &self,
        def: &FrameDef,
        item: &str,
        max: usize,
    ) -> Result<Vec<f64>, GeoError> {
        let value = self
            .dynamic_value(def, item)
            .ok_or(GeoError::BadSearch("SPICE(KERNELVARNOTFOUND)"))?;
        let tokens = value
            .as_list()
            .ok_or(GeoError::BadSearch("SPICE(BADVARIABLETYPE)"))?;
        if tokens.is_empty() || tokens.len() > max {
            return Err(GeoError::BadSearch("SPICE(BADVARIABLESIZE)"));
        }
        tokens.iter().map(|x| parse_f64(x)).collect()
    }
    fn dynamic_step(
        &self,
        def: &FrameDef,
        et: f64,
        pck: Option<&Pck>,
        need_av: bool,
    ) -> Result<(i32, FrameRotation), GeoError> {
        let _level = DynamicLevel::enter()?;
        let base = self.dynamic_frame_id(def, "RELATIVE")?;
        if !self
            .dynamic_string(def, "DEF_STYLE")?
            .eq_ignore_ascii_case("PARAMETERIZED")
        {
            return Err(GeoError::BadSearch("SPICE(NOTSUPPORTED)"));
        }
        let family = self
            .dynamic_string(def, "FAMILY")?
            .replace(' ', "")
            .to_uppercase();
        let frozen = self.dynamic_value(def, "FREEZE_EPOCH").is_some();
        let t0 = if frozen {
            self.dynamic_numbers(def, "FREEZE_EPOCH", 1)?[0]
        } else {
            et
        };
        let rotation_state = self.dynamic_optional_string(def, "ROTATION_STATE")?;
        if frozen && rotation_state.is_some() {
            return Err(GeoError::BadSearch("SPICE(FRAMEDEFERROR)"));
        }
        let of_date = matches!(
            family.as_str(),
            "MEAN_EQUATOR_AND_EQUINOX_OF_DATE"
                | "TRUE_EQUATOR_AND_EQUINOX_OF_DATE"
                | "MEAN_ECLIPTIC_AND_EQUINOX_OF_DATE"
        );
        if of_date && !frozen && rotation_state.is_none() {
            return Err(GeoError::BadSearch("SPICE(FRAMEDEFERROR)"));
        }
        let inert = if let Some(state) = rotation_state {
            if state.eq_ignore_ascii_case("INERTIAL") {
                true
            } else if state.eq_ignore_ascii_case("ROTATING") {
                false
            } else {
                return Err(GeoError::BadSearch("SPICE(NOTSUPPORTED)"));
            }
        } else {
            false
        };
        let mut result = match family.as_str() {
            "MEAN_EQUATOR_AND_EQUINOX_OF_DATE"
            | "TRUE_EQUATOR_AND_EQUINOX_OF_DATE"
            | "MEAN_ECLIPTIC_AND_EQUINOX_OF_DATE" => {
                self.of_date_step(def, base, &family, t0, pck, need_av)?
            }
            "EULER" => {
                let epoch = self.dynamic_numbers(def, "EPOCH", 1)?[0];
                let delta = t0 - epoch;
                let axis_values = self.dynamic_numbers(def, "AXES", 3)?;
                if axis_values
                    .iter()
                    .any(|value| !(1..=3).contains(&(value.round() as i32)))
                {
                    return Err(GeoError::BadSearch("SPICE(BADAXISNUMBERS)"));
                }
                if axis_values.len() != 3 {
                    return Err(GeoError::BadSearch("SPICE(BADVARIABLESIZE)"));
                }
                let mut axes = [0usize; 3];
                for i in 0..3 {
                    let a = axis_values[i].round() as i32;
                    if !(1..=3).contains(&a) {
                        return Err(GeoError::BadSearch("SPICE(BADAXISNUMBERS)"));
                    }
                    axes[i] = a as usize - 1;
                }
                let units = self.dynamic_string(def, "UNITS")?;
                let mut e = [0.0; 6];
                for i in 0..3 {
                    let cp = self.dynamic_numbers(def, &format!("ANGLE_{}_COEFFS", i + 1), 20)?;
                    let mut value = 0.0;
                    let mut derivative = 0.0;
                    for coefficient in cp.iter().rev() {
                        derivative = delta * derivative + 1.0 * value;
                        value = delta * value + coefficient;
                    }
                    e[i] = convert_angle(value, &units)?;
                    e[i + 3] = convert_angle(derivative, &units)?;
                }
                if need_av {
                    crate::orientation::euler_state(e, axes)?
                } else {
                    let first = match axes[2] {
                        0 => rot1(e[2]),
                        1 => rot2(e[2]),
                        _ => rot3(e[2]),
                    };
                    FrameRotation {
                        rotation: rotate_matrix(rotate_matrix(first, e[1], axes[1]), e[0], axes[0]),
                        rotation_dot: ZERO,
                    }
                }
            }
            "TWO-VECTOR" => self.constant_two_vector(def, base, t0, pck, need_av)?,
            "PRODUCT" => {
                let strings = |item: &str| -> Result<Vec<String>, GeoError> {
                    let v = self
                        .dynamic_value(def, item)
                        .ok_or(GeoError::FrameParse("missing product frame factors"))?;
                    let out = v
                        .as_strings()
                        .ok_or(GeoError::BadSearch("SPICE(BADVARIABLETYPE)"))?
                        .to_vec();
                    if out.is_empty() || out.len() > 10 {
                        return Err(GeoError::BadSearch("SPICE(BADVARIABLESIZE)"));
                    }
                    Ok(out)
                };
                let from = strings("FROM_FRAMES")?;
                let to = strings("TO_FRAMES")?;
                if from.len() != to.len() {
                    return Err(GeoError::BadSearch("SPICE(BADFRAMECOUNT)"));
                }
                let mut result = self.transformation(&to[0], &from[0], t0, pck, need_av)?;
                for i in 1..from.len() {
                    let step = self.transformation(&to[i], &from[i], t0, pck, need_av)?;
                    result = if need_av {
                        state_product(step, result)
                    } else {
                        FrameRotation {
                            rotation: mat_mul(&step.rotation, &result.rotation),
                            rotation_dot: ZERO,
                        }
                    };
                }
                result
            }
            _ => return Err(GeoError::BadSearch("SPICE(NOTSUPPORTED)")),
        };
        if need_av && inert {
            if self.frames[&base].class == 1 {
                result.rotation_dot = ZERO;
            } else {
                let xf = self.transformation(&base.to_string(), "J2000", t0, pck, true)?;
                let mut constant = state_product(xf, result);
                constant.rotation_dot = ZERO;
                let inverse = FrameRotation {
                    rotation: transpose(&xf.rotation),
                    rotation_dot: transpose(&xf.rotation_dot),
                };
                result.rotation_dot = state_product(inverse, constant).rotation_dot;
            }
        }
        if frozen {
            result.rotation_dot = ZERO;
        }
        Ok((base, result))
    }

    fn of_date_step(
        &self,
        def: &FrameDef,
        base: i32,
        family: &str,
        t0: f64,
        pck: Option<&Pck>,
        need_av: bool,
    ) -> Result<FrameRotation, GeoError> {
        if !self
            .dynamic_string(def, "PREC_MODEL")?
            .replace(' ', "")
            .eq_ignore_ascii_case("EARTH_IAU_1976")
        {
            return Err(GeoError::BadSearch("SPICE(NOTSUPPORTED)"));
        }
        if def.center != 399 {
            return Err(GeoError::BadSearch("SPICE(INVALIDSELECTION)"));
        }
        let precession = crate::earth_orientation::precession(t0);
        let mut forward = precession;
        if family == "TRUE_EQUATOR_AND_EQUINOX_OF_DATE" {
            if !self
                .dynamic_string(def, "NUT_MODEL")?
                .replace(' ', "")
                .eq_ignore_ascii_case("EARTH_IAU_1980")
            {
                return Err(GeoError::BadSearch("SPICE(NOTSUPPORTED)"));
            }
            let nutation = crate::earth_orientation::nutation(t0);
            forward = if need_av {
                state_product(nutation, precession)
            } else {
                FrameRotation {
                    rotation: mat_mul(&nutation.rotation, &precession.rotation),
                    rotation_dot: ZERO,
                }
            };
        } else if family == "MEAN_ECLIPTIC_AND_EQUINOX_OF_DATE" {
            if !self
                .dynamic_string(def, "OBLIQ_MODEL")?
                .replace(' ', "")
                .eq_ignore_ascii_case("EARTH_IAU_1980")
            {
                return Err(GeoError::BadSearch("SPICE(NOTSUPPORTED)"));
            }
            let t = t0 / 31557600.0 / 100.0;
            let rad = PI / 180.0;
            let mean = rad / 3600.0 * (t * (t * (t * 0.001813 - 5.9e-4) - 46.815) + 84381.448);
            let rate = rad / 3600.0
                * (t * (t * 3.0 * 0.001813 - 0.0011800000000000001) - 46.815)
                * (1.0 / (31557600.0 * 100.0));
            let obliquity = if need_av {
                crate::orientation::euler_state([0.0, 0.0, mean, 0.0, 0.0, rate], [0, 2, 0])?
            } else {
                FrameRotation {
                    rotation: rotate_matrix(rotate_matrix(rot1(mean), 0.0, 2), 0.0, 0),
                    rotation_dot: ZERO,
                }
            };
            forward = if need_av {
                state_product(obliquity, precession)
            } else {
                FrameRotation {
                    rotation: mat_mul(&obliquity.rotation, &precession.rotation),
                    rotation_dot: ZERO,
                }
            };
        }
        let mut result = FrameRotation {
            rotation: transpose(&forward.rotation),
            rotation_dot: if need_av {
                transpose(&forward.rotation_dot)
            } else {
                ZERO
            },
        };
        if base != J2000_ID {
            let change = self.transformation("J2000", &base.to_string(), t0, pck, need_av)?;
            result = if need_av {
                state_product(change, result)
            } else {
                FrameRotation {
                    rotation: mat_mul(&change.rotation, &result.rotation),
                    rotation_dot: ZERO,
                }
            };
        }
        Ok(result)
    }

    fn constant_two_vector(
        &self,
        def: &FrameDef,
        base: i32,
        t0: f64,
        pck: Option<&Pck>,
        need_av: bool,
    ) -> Result<FrameRotation, GeoError> {
        let mut states = [[0.0; 6]; 2];
        let mut axes = [0usize; 2];
        for (i, prefix) in ["PRI", "SEC"].iter().enumerate() {
            let axis = self
                .dynamic_string(def, &format!("{prefix}_AXIS"))?
                .replace(' ', "")
                .to_uppercase();
            let negative = axis.starts_with('-');
            let axis = axis.replace(['-', '+'], "");
            axes[i] = match axis.as_str() {
                "X" => 0,
                "Y" => 1,
                "Z" => 2,
                _ => return Err(GeoError::BadSearch("SPICE(INVALIDAXIS)")),
            };
            let definition = self
                .dynamic_string(def, &format!("{prefix}_VECTOR_DEF"))?
                .replace(' ', "")
                .to_uppercase();
            states[i] = if definition == "CONSTANT" {
                self.constant_vector_state(def, prefix, t0, pck, need_av)?
            } else {
                self.ephemeris_vector_state(def, prefix, &definition, t0, pck, need_av)?
            };
            if negative {
                states[i] = states[i].map(|x| -x);
            }
        }
        let a = [states[0][0], states[0][1], states[0][2]];
        let b = [states[1][0], states[1][1], states[1][2]];
        let minsep = if self.dynamic_value(def, "ANGLE_SEP_TOL").is_some() {
            self.dynamic_numbers(def, "ANGLE_SEP_TOL", 1)?[0]
        } else {
            0.001
        };
        if vector_separation(a, b).sin() < minsep.sin() {
            return Err(GeoError::BadSearch("SPICE(DEGENERATECASE)"));
        }
        if axes[0] == axes[1] {
            return Err(GeoError::BadSearch("SPICE(UNDEFINEDFRAME)"));
        }
        let i1 = axes[0];
        let i2 = (i1 + 1) % 3;
        let i3 = (i1 + 2) % 3;
        let mut columns = [[0.0; 6]; 3];
        if need_av {
            columns[i1] = crate::earth_orientation::normalize(states[0]);
            if axes[1] == i2 {
                columns[i3] = crate::earth_orientation::unit_cross_state(states[0], states[1]);
                columns[i2] = crate::earth_orientation::unit_cross_state(columns[i3], states[0]);
            } else {
                columns[i2] = crate::earth_orientation::unit_cross_state(states[1], states[0]);
                columns[i3] = crate::earth_orientation::unit_cross_state(states[0], columns[i2]);
            }
        } else {
            let mut c = [[0.0; 3]; 3];
            c[i1] = crate::surface::unit(a);
            if axes[1] == i2 {
                c[i3] = crate::surface::unit_cross(a, b);
                c[i2] = crate::surface::unit_cross(c[i3], a);
            } else {
                c[i2] = crate::surface::unit_cross(b, a);
                c[i3] = crate::surface::unit_cross(a, c[i2]);
            }
            for i in 0..3 {
                columns[i][..3].copy_from_slice(&c[i]);
            }
        }
        if columns[i2][..3].iter().all(|x| *x == 0.0) {
            return Err(GeoError::BadSearch("SPICE(DEPENDENTVECTORS)"));
        }
        let mut result = FrameRotation {
            rotation: std::array::from_fn(|i| std::array::from_fn(|j| columns[j][i])),
            rotation_dot: std::array::from_fn(|i| std::array::from_fn(|j| columns[j][i + 3])),
        };
        if base != J2000_ID {
            let change = self.transformation("J2000", &base.to_string(), t0, pck, need_av)?;
            result = if need_av {
                state_product(change, result)
            } else {
                FrameRotation {
                    rotation: mat_mul(&change.rotation, &result.rotation),
                    rotation_dot: ZERO,
                }
            };
        }
        Ok(result)
    }

    fn attached_ephemeris(&self) -> Result<&crate::spk::SpkKernel, GeoError> {
        self.ephemeris
            .as_ref()
            .map(|x| x.0.as_ref())
            .ok_or(GeoError::UnsupportedFrameSpec(
                "dynamic frame requires attached SPK context".into(),
            ))
    }
    fn dynamic_body(&self, def: &FrameDef, item: &str) -> Result<i32, GeoError> {
        let value = self.dynamic_string(def, item)?;
        self.body_id(&value).or_else(|_| {
            parse_f64(&value)
                .map(|x| x.round() as i32)
                .map_err(|_| GeoError::BadSearch("SPICE(NOTRANSLATION)"))
        })
    }
    fn ephemeris_state(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        correction: crate::spk::Aberration,
        frame: &str,
        pck: Option<&Pck>,
    ) -> Result<crate::spk::CorrectedState, GeoError> {
        let empty = Pck::default();
        self.attached_ephemeris()?.apparent_state_in(
            target,
            observer,
            et,
            correction,
            &crate::spk::FrameSource::Named {
                frames: self,
                pck: pck.unwrap_or(&empty),
                frame_name: frame,
            },
        )
    }
    fn ephemeris_position(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        correction: crate::spk::Aberration,
        frame: &str,
        pck: Option<&Pck>,
    ) -> Result<([f64; 3], f64), GeoError> {
        crate::spkapo::corrected_position_in_frame(
            self.attached_ephemeris()?,
            target,
            observer,
            et,
            correction,
            self,
            frame,
            pck,
        )
    }
    fn corrected_frame_epoch(
        &self,
        frame: i32,
        observer: i32,
        et: f64,
        correction: crate::spk::Aberration,
        pck: Option<&Pck>,
    ) -> Result<f64, GeoError> {
        if !correction.flags().0 || self.frames[&frame].class == 1 {
            return Ok(et);
        }
        let center = self.frames[&frame].center;
        let lt = self
            .ephemeris_position(center, observer, et, correction, "J2000", pck)?
            .1;
        Ok(if correction.flags().3 {
            et + lt
        } else {
            et - lt
        })
    }
    fn constant_vector_state(
        &self,
        def: &FrameDef,
        prefix: &str,
        t0: f64,
        pck: Option<&Pck>,
        need_av: bool,
    ) -> Result<[f64; 6], GeoError> {
        use crate::spk::Aberration;
        let frid = self.frame_id(&self.dynamic_string(def, &format!("{prefix}_FRAME"))?)?;
        let correction = self
            .dynamic_optional_string(def, &format!("{prefix}_ABCORR"))?
            .unwrap_or_else(|| "NONE".into())
            .replace(' ', "")
            .to_uppercase();
        let stellar = matches!(correction.as_str(), "S" | "XS");
        let transmission = correction == "XS";
        let ordinary = if stellar {
            Aberration::None
        } else {
            Aberration::parse(&correction)
                .map_err(|_| GeoError::BadSearch("SPICE(INVALIDOPTION)"))?
        };
        if ordinary.flags().0 && ordinary.flags().2 {
            return Err(GeoError::BadSearch("SPICE(INVALIDOPTION)"));
        }
        let fet = if ordinary.flags().0 && self.frames[&frid].class != 1 {
            let observer = self.dynamic_body(def, &format!("{prefix}_OBSERVER"))?;
            self.corrected_frame_epoch(frid, observer, t0, ordinary, pck)?
        } else {
            t0
        };
        let observer_velocity = if stellar {
            let observer = self.dynamic_body(def, &format!("{prefix}_OBSERVER"))?;
            Some(
                self.attached_ephemeris()?
                    .state_in_frame(observer, 0, t0, self, pck, "J2000")?
                    .velocity,
            )
        } else {
            None
        };
        let spec = self
            .dynamic_string(def, &format!("{prefix}_SPEC"))?
            .replace(' ', "")
            .to_uppercase();
        let dir = match spec.as_str() {
            "RECTANGULAR" => {
                let v = self.dynamic_numbers(def, &format!("{prefix}_VECTOR"), 3)?;
                if v.len() != 3 {
                    return Err(GeoError::FrameParse(
                        "constant vector requires three coordinates",
                    ));
                }
                [v[0], v[1], v[2]]
            }
            "LATITUDINAL" | "RA/DEC" => {
                let units = self.dynamic_string(def, &format!("{prefix}_UNITS"))?;
                let attrs = if spec == "LATITUDINAL" {
                    ["LONGITUDE", "LATITUDE"]
                } else {
                    ["RA", "DEC"]
                };
                let lon = convert_angle(
                    self.dynamic_numbers(def, &format!("{prefix}_{}", attrs[0]), 1)?[0],
                    &units,
                )?;
                let lat = convert_angle(
                    self.dynamic_numbers(def, &format!("{prefix}_{}", attrs[1]), 1)?[0],
                    &units,
                )?;
                [
                    1.0 * lon.cos() * lat.cos(),
                    1.0 * lon.sin() * lat.cos(),
                    1.0 * lat.sin(),
                ]
            }
            _ => return Err(GeoError::BadSearch("SPICE(NOTSUPPORTED)")),
        };
        let mut state = [dir[0], dir[1], dir[2], 0.0, 0.0, 0.0];
        if frid != J2000_ID {
            let change = self.transformation(&frid.to_string(), "J2000", fet, pck, need_av)?;
            state = if need_av {
                apply_state(change, state)
            } else {
                let p = apply_rotation(change.rotation, dir);
                [p[0], p[1], p[2], 0.0, 0.0, 0.0]
            };
        }
        if let Some(velocity) = observer_velocity {
            let velocity = if transmission {
                velocity.map(|x| -x)
            } else {
                velocity
            };
            let position = crate::surface::stellar_aberration_position(
                [state[0], state[1], state[2]],
                velocity,
            );
            state[..3].copy_from_slice(&position);
        }
        Ok(state)
    }
    fn ephemeris_vector_state(
        &self,
        def: &FrameDef,
        prefix: &str,
        definition: &str,
        t0: f64,
        pck: Option<&Pck>,
        need_av: bool,
    ) -> Result<[f64; 6], GeoError> {
        use crate::spk::Aberration;
        let target = self.dynamic_body(def, &format!("{prefix}_TARGET"))?;
        let observer = self.dynamic_body(def, &format!("{prefix}_OBSERVER"))?;
        let correction = Aberration::parse(&self.dynamic_string(def, &format!("{prefix}_ABCORR"))?)
            .map_err(|_| GeoError::BadSearch("SPICE(SPKINVALIDOPTION)"))?;
        match definition {
            "OBSERVER_TARGET_POSITION" => {
                if need_av {
                    let state = self
                        .ephemeris_state(target, observer, t0, correction, "J2000", pck)?
                        .state;
                    Ok([
                        state.position[0],
                        state.position[1],
                        state.position[2],
                        state.velocity[0],
                        state.velocity[1],
                        state.velocity[2],
                    ])
                } else {
                    let (p, _) =
                        self.ephemeris_position(target, observer, t0, correction, "J2000", pck)?;
                    Ok([p[0], p[1], p[2], 0.0, 0.0, 0.0])
                }
            }
            "OBSERVER_TARGET_VELOCITY" => {
                let frame = self.dynamic_string(def, &format!("{prefix}_FRAME"))?;
                let frid = self.frame_id(&frame)?;
                let mut state = [0.0; 6];
                if need_av {
                    // Native ZZDYNFRM/QDERIV, including the asymmetric
                    // negative-epoch step rule, not an invented derivative.
                    let delta = 1.0_f64.max(t0 * 7.4505805969238281e-9);
                    let mut vectors = [[0.0; 3]; 3];
                    for (j, t) in [t0 - delta, t0, t0 + delta].iter().enumerate() {
                        vectors[j] = crate::surface::unit(
                            self.ephemeris_state(target, observer, *t, correction, &frame, pck)?
                                .state
                                .velocity,
                        );
                    }
                    state[..3].copy_from_slice(&vectors[1]);
                    for i in 0..3 {
                        state[i + 3] =
                            (0.5 / delta) * vectors[2][i] + (-0.5 / delta) * vectors[0][i];
                    }
                } else {
                    let v = crate::surface::unit(
                        self.ephemeris_state(target, observer, t0, correction, &frame, pck)?
                            .state
                            .velocity,
                    );
                    state[..3].copy_from_slice(&v);
                }
                let vet = self.corrected_frame_epoch(frid, observer, t0, correction, pck)?;
                if frid != J2000_ID {
                    let change = self.transformation(&frame, "J2000", vet, pck, need_av)?;
                    state = if need_av {
                        apply_state(change, state)
                    } else {
                        let p = apply_rotation(change.rotation, [state[0], state[1], state[2]]);
                        [p[0], p[1], p[2], 0.0, 0.0, 0.0]
                    };
                }
                Ok(state)
            }
            "TARGET_NEAR_POINT" => {
                let pck = pck.ok_or(GeoError::MissingPckData(target))?;
                let frame = self.body_frame(target)?;
                let radii = pck.radii(target).ok_or(GeoError::MissingPckData(target))?;
                let (observer_state, lt) = if need_av {
                    let x =
                        self.ephemeris_state(target, observer, t0, correction, &frame, Some(pck))?;
                    (
                        [
                            -x.state.position[0],
                            -x.state.position[1],
                            -x.state.position[2],
                            -x.state.velocity[0],
                            -x.state.velocity[1],
                            -x.state.velocity[2],
                        ],
                        x.light_time,
                    )
                } else {
                    let (p, lt) = self.ephemeris_position(
                        target,
                        observer,
                        t0,
                        correction,
                        &frame,
                        Some(pck),
                    )?;
                    ([-p[0], -p[1], -p[2], 0.0, 0.0, 0.0], lt)
                };
                let p = [observer_state[0], observer_state[1], observer_state[2]];
                let v = [observer_state[3], observer_state[4], observer_state[5]];
                let near = crate::surface::nearest_point_on_ellipsoid(p, radii)?.point;
                let derivative = if need_av {
                    crate::finder::nearpoint_velocity(p, v, near, radii)
                        .map_err(|_| GeoError::BadSearch("SPICE(DEGENERATECASE)"))?
                } else {
                    [0.0; 3]
                };
                let relative = [
                    near[0] - p[0],
                    near[1] - p[1],
                    near[2] - p[2],
                    derivative[0] - v[0],
                    derivative[1] - v[1],
                    derivative[2] - v[2],
                ];
                let fet = if correction.flags().3 {
                    t0 + lt
                } else if correction.flags().0 {
                    t0 - lt
                } else {
                    t0
                };
                let change = self.transformation(&frame, "J2000", fet, Some(pck), need_av)?;
                Ok(if need_av {
                    apply_state(change, relative)
                } else {
                    let p =
                        apply_rotation(change.rotation, [relative[0], relative[1], relative[2]]);
                    [p[0], p[1], p[2], 0.0, 0.0, 0.0]
                })
            }
            _ => Err(GeoError::BadSearch("SPICE(NOTSUPPORTED)")),
        }
    }

    /// Rotation mapping J2000 vectors into `frame`, plus its derivative.
    fn j2000_orientation(
        &self,
        frame: &str,
        et: f64,
        pck: Option<&Pck>,
    ) -> Result<(Mat3, Mat3), GeoError> {
        if let Ok(id) = self.frame_id(frame) {
            return self.frame_j2000_orientation(id, et, pck);
        }
        if let Some(body) = iau_body_id(frame) {
            return body_j2000_orientation(body, frame, et, pck);
        }
        Err(GeoError::FrameNotFound(frame.to_string()))
    }

    /// Rotation mapping J2000 vectors into the frame with NAIF id `id`.
    fn frame_j2000_orientation(
        &self,
        id: i32,
        et: f64,
        pck: Option<&Pck>,
    ) -> Result<(Mat3, Mat3), GeoError> {
        if id == J2000_ID {
            return Ok((IDENTITY, ZERO));
        }

        let Some(def) = self.frames.get(&id) else {
            return Err(GeoError::FrameNotFound(id.to_string()));
        };
        if def.class == 2 {
            let (parent, local) = self.state_step(id, et, pck, true)?.ok_or_else(|| {
                GeoError::FrameNotFound(format!("frame {id} has no transformation at ET {et}"))
            })?;
            let (rotation, rotation_dot) = self.frame_j2000_orientation(parent, et, pck)?;
            let result = state_product(
                FrameRotation {
                    rotation,
                    rotation_dot,
                },
                local,
            );
            return Ok((result.rotation, result.rotation_dot));
        }

        let parent = def.relative.unwrap_or(J2000_ID);
        let (parent_rotation, parent_dot) = self.frame_j2000_orientation(parent, et, pck)?;
        let local = def
            .rotation
            .ok_or_else(|| GeoError::FrameNotFound(format!("frame {id} has no fixed rotation")))?;

        // `local` maps this frame into its parent, so the full rotation is
        // `parent * local` and only the parent can depend on time.
        Ok((
            mat_mul(&parent_rotation, &local),
            mat_mul(&parent_dot, &local),
        ))
    }
}

#[derive(Default)]
struct RawFrame {
    frame: HashMap<String, Value>,
    tk: HashMap<String, Value>,
}

fn build_rotation(id: &i32, tk: &HashMap<String, Value>) -> Result<Mat3, GeoError> {
    for item in ["MATRIX", "ANGLES", "AXES", "Q"] {
        if tk.get(item).is_some_and(|value| value.as_list().is_none()) {
            return Err(GeoError::BadSearch("SPICE(BADVARIABLETYPE)"));
        }
    }
    let spec = tk
        .get("SPEC")
        .and_then(Value::as_string)
        .ok_or(GeoError::FrameParse("TK frame has no SPEC"))?;
    let spec = spec.trim().to_uppercase();

    match spec.as_str() {
        "MATRIX" => {
            let tokens = tk
                .get("MATRIX")
                .and_then(Value::as_list)
                .ok_or(GeoError::FrameParse("MATRIX frame has no MATRIX"))?;
            if tokens.len() != 9 {
                return Err(GeoError::FrameParse("TKFRAME_MATRIX must have 9 values"));
            }
            let mut values = [0.0f64; 9];
            for (index, token) in tokens.iter().enumerate() {
                values[index] = parse_f64(token)?;
            }
            // The kernel lists the matrix column by column.
            let mut matrix = [[0.0f64; 3]; 3];
            for row in 0..3 {
                for column in 0..3 {
                    matrix[row][column] = values[column * 3 + row];
                }
            }
            orthonormalize_columns(matrix)
        }
        "ANGLES" => {
            let angles = tk
                .get("ANGLES")
                .and_then(Value::as_list)
                .ok_or(GeoError::FrameParse("ANGLES frame has no ANGLES"))?;
            let axes = tk
                .get("AXES")
                .and_then(Value::as_list)
                .ok_or(GeoError::FrameParse("ANGLES frame has no AXES"))?;
            let units = tk
                .get("UNITS")
                .and_then(Value::as_string)
                .unwrap_or_else(|| "RADIANS".to_string());
            if angles.len() != 3 || axes.len() != 3 {
                return Err(GeoError::FrameParse("TKFRAME_ANGLES/_AXES need 3 values"));
            }

            let mut rotations = [IDENTITY; 3];
            let mut axis_numbers = [0usize; 3];
            let mut converted = [0.0; 3];
            for index in 0..3 {
                let angle = convert_angle(parse_f64(&angles[index])?, &units)?;
                converted[index] = angle;
                let axis = parse_f64(&axes[index])?.round() as i32;
                let rotation = match axis {
                    1 => rot1(angle),
                    2 => rot2(angle),
                    3 => rot3(angle),
                    _ => return Err(GeoError::BadSearch("SPICE(BADAXISNUMBERS)")),
                };
                rotations[index] = rotation;
                axis_numbers[index] = axis as usize - 1;
            }
            let mut matrix = rotations[2];
            for index in [1, 0] {
                matrix = rotate_matrix(matrix, converted[index], axis_numbers[index]);
            }
            Ok(matrix)
        }
        "QUATERNION" => {
            let tokens = tk
                .get("Q")
                .and_then(Value::as_list)
                .ok_or(GeoError::FrameParse("QUATERNION frame has no Q"))?;
            if tokens.len() != 4 {
                return Err(GeoError::FrameParse("TKFRAME_Q must have 4 values"));
            }
            let mut q = [0.0f64; 4];
            for (index, token) in tokens.iter().enumerate() {
                q[index] = parse_f64(token)?;
            }
            quaternion_matrix(q)
        }
        other => Err(GeoError::UnsupportedFrameSpec(format!(
            "TKFRAME_SPEC '{other}' for frame {id}"
        ))),
    }
}

fn convert_angle(value: f64, units: &str) -> Result<f64, GeoError> {
    let dpr = 180.0 / PI;
    let degrees = match units.trim().to_uppercase().as_str() {
        "RADIANS" => dpr,
        "DEGREES" => 1.0,
        "ARCSECONDS" => 2.7777777777777778e-4,
        "ARCMINUTES" => 0.016666666666666666,
        "HOURANGLE" => 15.0,
        "MINUTEANGLE" => 0.25,
        "SECONDANGLE" => 0.0041666666666666666,
        other => {
            let _ = other;
            return Err(GeoError::BadSearch("SPICE(UNITSNOTREC)"));
        }
    };
    Ok((value * degrees) / dpr)
}

// ROTMAT preserves an unchanged row and evaluates two-term row products.
pub(crate) fn rotate_matrix(matrix: Mat3, angle: f64, axis: usize) -> Mat3 {
    let (sin, cos) = angle.sin_cos();
    let i2 = (axis + 1) % 3;
    let i3 = (axis + 2) % 3;
    let mut out = matrix;
    for column in 0..3 {
        out[i2][column] = cos * matrix[i2][column] + sin * matrix[i3][column];
        out[i3][column] = -sin * matrix[i2][column] + cos * matrix[i3][column];
    }
    out
}

/// SPICE `[A]_1`.
pub(crate) fn rot1(angle: f64) -> Mat3 {
    let (sin, cos) = angle.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, cos, sin], [0.0, -sin, cos]]
}

/// SPICE `[A]_2`.
pub(crate) fn rot2(angle: f64) -> Mat3 {
    let (sin, cos) = angle.sin_cos();
    [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]]
}

/// SPICE `[A]_3`.
pub(crate) fn rot3(angle: f64) -> Mat3 {
    let (sin, cos) = angle.sin_cos();
    [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]]
}

/// Rotation matrix of a quaternion `(q0, q1, q2, q3)`, scalar part first.
fn quaternion_matrix(q: [f64; 4]) -> Result<Mat3, GeoError> {
    // TKFRAM first calls VHATG, then Q2M sharpens the normalized products.
    let maximum = q.iter().map(|x| x.abs()).fold(0.0, f64::max);
    let magnitude = if maximum == 0.0 {
        0.0
    } else {
        let mut sum = 0.0;
        for x in q {
            let a = x / maximum;
            sum += a * a;
        }
        maximum * sum.sqrt()
    };
    let q = if magnitude == 0.0 {
        [0.0; 4]
    } else {
        q.map(|x| x / magnitude)
    };
    let (mut q01, mut q02, mut q03) = (q[0] * q[1], q[0] * q[2], q[0] * q[3]);
    let (mut q12, mut q13, mut q23) = (q[1] * q[2], q[1] * q[3], q[2] * q[3]);
    let (mut q1s, mut q2s, mut q3s) = (q[1] * q[1], q[2] * q[2], q[3] * q[3]);
    let l2 = q[0] * q[0] + q1s + q2s + q3s;
    if l2 != 1.0 && l2 != 0.0 {
        let sharp = 1.0 / l2;
        q01 *= sharp;
        q02 *= sharp;
        q03 *= sharp;
        q12 *= sharp;
        q13 *= sharp;
        q23 *= sharp;
        q1s *= sharp;
        q2s *= sharp;
        q3s *= sharp;
    }
    Ok([
        [
            1.0 - (q2s + q3s) * 2.0,
            (q12 - q03) * 2.0,
            (q13 + q02) * 2.0,
        ],
        [
            (q12 + q03) * 2.0,
            1.0 - (q1s + q3s) * 2.0,
            (q23 - q01) * 2.0,
        ],
        [
            (q13 - q02) * 2.0,
            (q23 + q01) * 2.0,
            1.0 - (q1s + q2s) * 2.0,
        ],
    ])
}

/// Source SHARPR, followed by TKFRAM's input-column handedness checks.
fn orthonormalize_columns(matrix: Mat3) -> Result<Mat3, GeoError> {
    let original1 = [matrix[0][1], matrix[1][1], matrix[2][1]];
    let original2 = [matrix[0][2], matrix[1][2], matrix[2][2]];
    let c0 = crate::surface::unit([matrix[0][0], matrix[1][0], matrix[2][0]]);
    let mut c2 = crate::surface::unit_cross(c0, original1);
    let mut c1 = crate::surface::unit_cross(c2, c0);
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    if dot(c1, original1) < 0.0 {
        c1 = c1.map(|x| -x);
    }
    if dot(c2, original2) < 0.0 {
        c2 = c2.map(|x| -x);
    }
    Ok([
        [c0[0], c1[0], c2[0]],
        [c0[1], c1[1], c2[1]],
        [c0[2], c1[2], c2[2]],
    ])
}

/// Multiplies two rotations.
pub fn mat_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut result = [[0.0f64; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] =
                a[row][0] * b[0][column] + a[row][1] * b[1][column] + a[row][2] * b[2][column];
        }
    }
    result
}

/// Transposes a rotation.
pub fn transpose(matrix: &Mat3) -> Mat3 {
    [
        [matrix[0][0], matrix[1][0], matrix[2][0]],
        [matrix[0][1], matrix[1][1], matrix[2][1]],
        [matrix[0][2], matrix[1][2], matrix[2][2]],
    ]
}

fn apply_rotation(r: Mat3, v: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| r[i][0] * v[0] + r[i][1] * v[1] + r[i][2] * v[2])
}
fn apply_state(r: FrameRotation, v: [f64; 6]) -> [f64; 6] {
    std::array::from_fn(|i| {
        let mut sum = 0.0;
        for j in 0..6 {
            let a = if i < 3 && j < 3 {
                r.rotation[i][j]
            } else if i >= 3 && j >= 3 {
                r.rotation[i - 3][j - 3]
            } else if i >= 3 {
                r.rotation_dot[i - 3][j]
            } else {
                0.0
            };
            sum += a * v[j];
        }
        sum
    })
}
fn vector_separation(a: [f64; 3], b: [f64; 3]) -> f64 {
    let na = crate::surface::norm(a);
    let nb = crate::surface::norm(b);
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    let a = a.map(|x| x / na);
    let b = b.map(|x| x / nb);
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    if dot > 0.0 {
        2.0 * (crate::surface::norm(std::array::from_fn(|i| a[i] - b[i])) * 0.5).asin()
    } else if dot < 0.0 {
        PI - 2.0 * (crate::surface::norm(std::array::from_fn(|i| a[i] + b[i])) * 0.5).asin()
    } else {
        PI * 0.5
    }
}

// N0067 ZZMSXF: the derivative block is a six-term, left-associated sum.
fn state_product(a: FrameRotation, b: FrameRotation) -> FrameRotation {
    let rotation = mat_mul(&a.rotation, &b.rotation);
    let rotation_dot = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            a.rotation_dot[i][0] * b.rotation[0][j]
                + a.rotation_dot[i][1] * b.rotation[1][j]
                + a.rotation_dot[i][2] * b.rotation[2][j]
                + a.rotation[i][0] * b.rotation_dot[0][j]
                + a.rotation[i][1] * b.rotation_dot[1][j]
                + a.rotation[i][2] * b.rotation_dot[2][j]
        })
    });
    FrameRotation {
        rotation,
        rotation_dot,
    }
}

/// Parses a body-fixed frame name of the form `IAU_<naif-id>`.
fn iau_body_id(name: &str) -> Option<i32> {
    let rest = name.trim().to_ascii_uppercase();
    rest.strip_prefix("IAU_")?.parse::<i32>().ok()
}

/// Rotation mapping J2000 vectors into an IAU body-fixed frame.
fn body_j2000_orientation(
    body: i32,
    frame: &str,
    et: f64,
    pck: Option<&Pck>,
) -> Result<(Mat3, Mat3), GeoError> {
    let pck = pck.ok_or_else(|| GeoError::FrameNotFound(frame.to_string()))?;
    let (rotation, rotation_dot) = pck.body_state_rotation_checked(body, et)?;
    Ok((transpose(&rotation), transpose(&rotation_dot)))
}

pub(crate) fn extract_data_blocks(text: &str) -> String {
    // NAIF requires `\begindata` and `\begintext` to sit on a line by
    // themselves. A substring search would also match the prose in a
    // kernel's help text, so scan line by line.
    let mut output = String::new();
    let mut inside = false;

    for line in text.lines() {
        match line.trim() {
            "\\begindata" => inside = true,
            "\\begintext" => inside = false,
            _ if inside => {
                output.push_str(line);
                output.push('\n');
            }
            _ => {}
        }
    }

    output
}

pub(crate) fn parse_f64(token: &str) -> Result<f64, GeoError> {
    if token.starts_with('@') {
        return stkcraft_time::kernel_date::parse_kernel_date(token)
            .map(|date| date.seconds_past_j2000)
            .map_err(|error| {
                GeoError::UnsupportedFrameSpec(format!(
                    "STKCraft-time kernel date {token}: {error}"
                ))
            });
    }
    // PRSDP/NPARSD deliberately evaluates integer and fractional parts
    // separately. Correctly-rounded strtod is not bit-equivalent (for
    // example BODY301_NUT_PREC_RA=-3.8787). Match the kernel pool parser.
    let invalid = || GeoError::FrameParse("invalid numeric value in a text kernel");
    let text: String = token
        .chars()
        .filter(|c| *c != ' ' && *c != ',')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let (sign, unsigned) = if let Some(s) = text.strip_prefix('-') {
        (-1.0, s)
    } else {
        (1.0, text.strip_prefix('+').unwrap_or(&text))
    };
    if unsigned == "PI" {
        return Ok(sign * PI);
    }
    let exponent_position = unsigned.find(['E', 'D']);
    let (mantissa, exponent) = match exponent_position {
        Some(position) => (&unsigned[..position], &unsigned[position + 1..]),
        None => (unsigned, ""),
    };
    let mut integer = 0.0;
    let mut fraction = 0.0;
    let mut divisor = 1.0;
    let mut factor = 1.0;
    let mut exponent_shift = 0;
    let mut decimal = false;
    let mut zero_integer = true;
    let mut has_digits = false;
    let mut round_integer = true;
    let mut round_decimal = true;
    const INTEGER_BOUND: f64 = 1e15;
    for character in mantissa.chars() {
        if character == '.' && !decimal {
            decimal = true;
            zero_integer = integer == 0.0;
            continue;
        }
        let digit = character.to_digit(10).ok_or_else(invalid)? as f64;
        has_digits = true;
        if !decimal {
            if integer < INTEGER_BOUND {
                integer = integer * 10.0 + digit;
            } else {
                exponent_shift += 1;
                factor /= 10.0;
                if round_integer {
                    round_integer = false;
                    if digit > 5.0 {
                        integer += 1.0;
                    }
                }
            }
        } else if zero_integer {
            if fraction < INTEGER_BOUND {
                fraction = fraction * 10.0 + digit;
                exponent_shift -= 1;
            } else if round_decimal {
                round_decimal = false;
                if digit >= 5.0 {
                    fraction += 1.0;
                }
            }
        } else if divisor < INTEGER_BOUND {
            fraction = fraction * 10.0 + digit;
            divisor *= 10.0;
        }
    }
    if !has_digits && !(mantissa.is_empty() && exponent_position.is_some()) {
        return Err(invalid());
    }
    let mut value = if has_digits {
        integer + fraction / divisor * factor
    } else {
        1.0
    };
    let parsed_exponent = if exponent.is_empty() || matches!(exponent, "+" | "-") {
        0
    } else {
        exponent.parse::<i32>().map_err(|_| invalid())?
    };
    let mut power = parsed_exponent
        .checked_add(exponent_shift)
        .ok_or_else(invalid)?;
    const TEN: [f64; 11] = [1.0, 10.0, 100.0, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10];
    if power < 0 {
        while power < -10 && value != 0.0 {
            value /= TEN[10];
            power += 10;
        }
        if value != 0.0 {
            value /= TEN[(-power) as usize];
        }
    } else if power > 0 {
        while power > 10 {
            value *= TEN[10];
            power -= 10;
            if !value.is_finite() {
                return Err(invalid());
            }
        }
        value *= TEN[power as usize];
    }
    if !value.is_finite() {
        return Err(invalid());
    }
    Ok(sign * value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &Mat3, b: &Mat3) {
        for row in 0..3 {
            for column in 0..3 {
                assert!(
                    (a[row][column] - b[row][column]).abs() < 1e-12,
                    "row {row} column {column}: {} vs {}",
                    a[row][column],
                    b[row][column]
                );
            }
        }
    }

    #[test]
    fn euler_matches_spice_ordering() {
        // Verified value from CSPICE pxform for axes (3,1,3) at
        // (10, 20, 30) degrees.
        let matrix = mat_mul(
            &mat_mul(&rot3(10.0f64.to_radians()), &rot1(20.0f64.to_radians())),
            &rot3(30.0f64.to_radians()),
        );
        let expected = [
            [0.771280576369, 0.633718360862, 0.059391174614],
            [-0.613092022380, 0.714610177143, 0.336824088833],
            [0.171010071663, -0.296198132726, 0.939692620786],
        ];
        close(&matrix, &expected);
    }

    #[test]
    fn quaternion_scalar_first() {
        // Verified value from CSPICE q2m.
        let matrix = quaternion_matrix([0.1, 0.2, 0.3, 0.927_361_849_549_570_3]).unwrap();
        let expected = [
            [-0.9, -0.065472369910, 0.430944739820],
            [0.305472369910, -0.8, 0.516417109730],
            [0.310944739820, 0.596417109730, 0.74],
        ];
        close(&matrix, &expected);
    }

    #[test]
    fn eclipj2000_matches_cspice() {
        let frames = Frames::new();
        let matrix = frames.pxform("J2000", "ECLIPJ2000", 0.0).unwrap();
        let expected = [
            [1.0, 0.0, 0.0],
            [0.0, 0.917482062069, 0.397777155932],
            [0.0, -0.397777155932, 0.917482062069],
        ];
        close(&matrix, &expected);
    }

    #[test]
    fn round_trip_is_identity() {
        let frames = Frames::new();
        let forward = frames.pxform("J2000", "ECLIPJ2000", 0.0).unwrap();
        let backward = frames.pxform("ECLIPJ2000", "J2000", 0.0).unwrap();
        close(&mat_mul(&backward, &forward), &IDENTITY);
    }

    #[test]
    fn parses_matrix_angles_and_quaternion() {
        let text = r#"
KPL/FK
\begindata

FRAME_DEMO_MATRIX       = 1400101
FRAME_1400101_NAME      = 'DEMO_MATRIX'
FRAME_1400101_CLASS     = 4
FRAME_1400101_CLASS_ID  = 1400101
FRAME_1400101_CENTER    = 399
TKFRAME_1400101_RELATIVE = 'J2000'
TKFRAME_1400101_SPEC    = 'MATRIX'
TKFRAME_1400101_MATRIX  = ( 2.0, 1.0, 0.0,
                            1.0, 2.0, 1.0,
                            0.0, 1.0, 2.0 )

FRAME_DEMO_ANGLES       = 1400102
FRAME_1400102_NAME      = 'DEMO_ANGLES'
FRAME_1400102_CLASS     = 4
FRAME_1400102_CLASS_ID  = 1400102
FRAME_1400102_CENTER    = 399
TKFRAME_1400102_RELATIVE = 'J2000'
TKFRAME_1400102_SPEC    = 'ANGLES'
TKFRAME_1400102_ANGLES  = ( 10.0, 20.0, 30.0 )
TKFRAME_1400102_AXES    = ( 3, 1, 3 )
TKFRAME_1400102_UNITS   = 'DEGREES'

FRAME_DEMO_Q            = 1400103
FRAME_1400103_NAME      = 'DEMO_Q'
FRAME_1400103_CLASS     = 4
FRAME_1400103_CLASS_ID  = 1400103
FRAME_1400103_CENTER    = 399
TKFRAME_1400103_RELATIVE = 'J2000'
TKFRAME_1400103_SPEC    = 'QUATERNION'
TKFRAME_1400103_Q       = ( 0.1, 0.2, 0.3, 0.92736184954957038 )

\begintext
"#;
        let frames = Frames::from_text(text).unwrap();

        // MATRIX: CSPICE Gram-Schmidt result for the same tokens.
        let matrix = frames.pxform("DEMO_MATRIX", "J2000", 0.0).unwrap();
        let expected = [
            [0.894427191, -0.3585685828, 0.267261241912],
            [0.4472135955, 0.717137165601, -0.534522483825],
            [0.0, 0.597614304667, 0.801783725737],
        ];
        close(&matrix, &expected);

        // ANGLES: same as the unit-test value above.
        let angles = frames.pxform("DEMO_ANGLES", "J2000", 0.0).unwrap();
        let expected_angles = [
            [0.771280576369, 0.633718360862, 0.059391174614],
            [-0.613092022380, 0.714610177143, 0.336824088833],
            [0.171010071663, -0.296198132726, 0.939692620786],
        ];
        close(&angles, &expected_angles);

        // QUATERNION: same as the q2m unit test.
        let quaternion = frames.pxform("DEMO_Q", "J2000", 0.0).unwrap();
        let expected_quaternion = [
            [-0.9, -0.065472369910, 0.430944739820],
            [0.305472369910, -0.8, 0.516417109730],
            [0.310944739820, 0.596417109730, 0.74],
        ];
        close(&quaternion, &expected_quaternion);

        // Aliases and ids resolve to the same frames.
        assert_eq!(frames.frame_id("demo_q").unwrap(), 1400103);
        assert_eq!(frames.frame_id("1400103").unwrap(), 1400103);
    }

    #[test]
    fn class2_ignores_unrelated_tk_spec_and_requires_orientation() {
        let text = r#"
\begindata
FRAME_X                 = 1400201
FRAME_1400201_NAME      = 'X'
FRAME_1400201_CLASS     = 2
FRAME_1400201_CLASS_ID  = 1400201
FRAME_1400201_CENTER    = 399
TKFRAME_1400201_RELATIVE = 'J2000'
TKFRAME_1400201_SPEC    = 'MYSTERY'
\begintext
"#;
        assert_eq!(
            text,
            include_str!("../../tests/golden/frame_kernel_errors/legacy_class2_tk_spec.tf")
        );
        let frames = Frames::from_text(text).unwrap();
        assert_eq!(frames.pxform("X", "X", 0.0).unwrap(), IDENTITY);
        assert!(matches!(
            frames.pxform("X", "J2000", 0.0),
            Err(GeoError::BadSearch("SPICE(FRAMEDATANOTFOUND)"))
        ));
    }
}
