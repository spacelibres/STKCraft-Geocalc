// Preserve native numerical indexing, constants, interface shape and operation grouping.
#![allow(clippy::needless_range_loop, clippy::assign_op_pattern)]
//! SPK ephemeris kernels: all16 segment types dispatched by N0067 SPKPVN.
//!
//! Type 2 stores Chebyshev coefficients for position and derives velocity
//! analytically. Type 3 stores separate Chebyshev coefficients for
//! position and velocity. In both types the four-word directory
//! (`INIT`, `INTLEN`, `RSIZE`, `N`) sits at the **end** of the segment.
//!
//! All 21 built-in inertial SPK segment frames are supported. Rotating
//! source frames are supported by the explicit `Frames`/PCK context APIs.
//! Geometric states are returned by [`SpkKernel::state`]; light-time and stellar
//! aberration corrections are applied by [`SpkKernel::state_corrected`]
//! with the same algorithm as CSPICE `spkezr`. These limits are stated
//! explicitly rather than approximated.

use std::collections::HashSet;
use std::ops::{Add, Neg, Sub};
use std::path::Path;

use stkcraft_time::convert::tai_to_et;
use stkcraft_time::{Epoch, LeapSecondTable};

use crate::daf::DafFile;
use crate::error::GeoError;
use crate::frame::Frames;
use crate::frame::Mat3;
use crate::pck::Pck;

/// Position and velocity of a target relative to an observer.
///
/// Position is in kilometres and velocity in kilometres per second, in the
/// J2000 frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    /// Position in km.
    pub position: [f64; 3],
    /// Velocity in km/s.
    pub velocity: [f64; 3],
}

impl State {
    /// The zero state.
    pub const ZERO: State = State {
        position: [0.0; 3],
        velocity: [0.0; 3],
    };
}

impl Add for State {
    type Output = State;

    fn add(self, other: State) -> State {
        State {
            position: [
                self.position[0] + other.position[0],
                self.position[1] + other.position[1],
                self.position[2] + other.position[2],
            ],
            velocity: [
                self.velocity[0] + other.velocity[0],
                self.velocity[1] + other.velocity[1],
                self.velocity[2] + other.velocity[2],
            ],
        }
    }
}

impl Neg for State {
    type Output = State;

    fn neg(self) -> State {
        State {
            position: [-self.position[0], -self.position[1], -self.position[2]],
            velocity: [-self.velocity[0], -self.velocity[1], -self.velocity[2]],
        }
    }
}

impl Sub for State {
    type Output = State;

    fn sub(self, other: State) -> State {
        self + (-other)
    }
}

/// Speed of light in km/s, the value used by CSPICE.
pub(crate) const CLIGHT: f64 = 299_792.458;

/// Aberration corrections understood by [`SpkKernel::state_corrected`].
///
/// The names match the CSPICE `ABCORR` argument, including the
/// `X`-prefixed transmission variants. `S` alone is rejected, because
/// CSPICE `spkezr` rejects stellar aberration without light time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aberration {
    /// No correction.
    None,
    /// One iteration of the one-way light-time solution.
    LightTime,
    /// One iteration of light time plus stellar aberration.
    LightTimeStellar,
    /// Converged Newtonian light time.
    ConvergedLightTime,
    /// Converged Newtonian light time plus stellar aberration.
    ConvergedLightTimeStellar,
    /// Transmission case, one iteration of light time.
    TransmissionLightTime,
    /// Transmission case, light time plus stellar aberration.
    TransmissionLightTimeStellar,
    /// Transmission case, converged light time.
    TransmissionConvergedLightTime,
    /// Transmission case, converged light time plus stellar aberration.
    TransmissionConvergedLightTimeStellar,
}

impl Aberration {
    /// Parses a CSPICE `ABCORR` string.
    pub fn parse(text: &str) -> Result<Self, GeoError> {
        match text.trim().to_uppercase().as_str() {
            "NONE" => Ok(Aberration::None),
            "LT" => Ok(Aberration::LightTime),
            "LT+S" => Ok(Aberration::LightTimeStellar),
            "CN" => Ok(Aberration::ConvergedLightTime),
            "CN+S" => Ok(Aberration::ConvergedLightTimeStellar),
            "XLT" => Ok(Aberration::TransmissionLightTime),
            "XLT+S" => Ok(Aberration::TransmissionLightTimeStellar),
            "XCN" => Ok(Aberration::TransmissionConvergedLightTime),
            "XCN+S" => Ok(Aberration::TransmissionConvergedLightTimeStellar),
            _ => Err(GeoError::BadAberration(text.to_string())),
        }
    }

    /// Returns `(use_light_time, converged, stellar, transmission)`.
    pub(crate) fn flags(self) -> (bool, bool, bool, bool) {
        match self {
            Aberration::None => (false, false, false, false),
            Aberration::LightTime => (true, false, false, false),
            Aberration::LightTimeStellar => (true, false, true, false),
            Aberration::ConvergedLightTime => (true, true, false, false),
            Aberration::ConvergedLightTimeStellar => (true, true, true, false),
            Aberration::TransmissionLightTime => (true, false, false, true),
            Aberration::TransmissionLightTimeStellar => (true, false, true, true),
            Aberration::TransmissionConvergedLightTime => (true, true, false, true),
            Aberration::TransmissionConvergedLightTimeStellar => (true, true, true, true),
        }
    }
}

/// A state together with the light time used to compute it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CorrectedState {
    /// Apparent state of the target relative to the observer.
    pub state: State,
    /// One-way light time in seconds.
    pub light_time: f64,
    /// Rate of change of light time, dimensionless.
    pub light_time_rate: f64,
}

/// The output reference frame of an apparent state, as CSPICE `spkezr`
/// resolves the `ref` argument.
#[derive(Clone, Copy, Debug)]
pub enum FrameSource<'a> {
    /// The J2000 inertial frame.
    J2000,
    /// An inertial output rotation, such as `ECLIPJ2000`. Corrections are
    /// evaluated in this frame. A class-4 TKFRAME uses FixedNonInertial.
    Fixed(&'a Mat3),
    /// A class-4 frame with constant J2000 orientation and a declared center.
    /// Corrections are evaluated in J2000 before the final rotation.
    FixedNonInertial { matrix: &'a Mat3, center: i32 },
    /// A named frame resolved through an FK and text/binary PCK context.
    Named {
        frames: &'a crate::frame::Frames,
        pck: &'a Pck,
        frame_name: &'a str,
    },
    /// An IAU body-fixed frame taken from a text PCK.
    BodyFixed { pck: &'a Pck, body: i32 },
}

impl FrameSource<'_> {
    /// Position-only J2000 rotation, following REFCHG rather than FRMCHG.
    pub fn rotation_at(&self, et: f64) -> Result<Mat3, GeoError> {
        match self {
            FrameSource::J2000 => Ok(identity()),
            FrameSource::Fixed(matrix) | FrameSource::FixedNonInertial { matrix, .. } => {
                Ok(**matrix)
            }
            FrameSource::Named {
                frames,
                pck,
                frame_name,
            } => frames.pxform_in("J2000", frame_name, et, Some(pck)),
            FrameSource::BodyFixed { pck, body } => pck.body_rotation_checked(*body, et),
        }
    }

    /// Rotation and its time derivative from J2000 at `et`.
    pub fn at(&self, et: f64) -> Result<(Mat3, Mat3), GeoError> {
        match self {
            FrameSource::J2000 => Ok((identity(), [[0.0; 3]; 3])),
            FrameSource::Fixed(matrix) | FrameSource::FixedNonInertial { matrix, .. } => {
                Ok((**matrix, [[0.0; 3]; 3]))
            }
            FrameSource::Named {
                frames,
                pck,
                frame_name,
            } => {
                let state = frames.pxform_with_derivative("J2000", frame_name, et, Some(pck))?;
                Ok((state.rotation, state.rotation_dot))
            }
            FrameSource::BodyFixed { pck, body } => pck.body_state_rotation_checked(*body, et),
        }
    }

    /// The NAIF id the frame is centered on, when it has one.
    fn center(&self) -> Option<i32> {
        match self {
            FrameSource::BodyFixed { body, .. } => Some(*body),
            FrameSource::Named {
                frames, frame_name, ..
            } => frames.center(frame_name).ok(),
            FrameSource::FixedNonInertial { center, .. } => Some(*center),
            _ => None,
        }
    }
}

/// One SPK segment.
#[derive(Clone, Debug)]
pub struct SpkSegment {
    /// Segment name.
    pub name: String,
    /// NAIF id of the target body.
    pub target: i32,
    /// NAIF id of the center body.
    pub center: i32,
    /// Reference frame id (1 is J2000).
    pub frame: i32,
    /// SPK data type (1/2/3/5/8/9/10/12/13/14/15/17/18/19/20/21).
    pub data_type: i32,
    /// First epoch covered, ET seconds past J2000.
    pub start_et: f64,
    /// Last epoch covered, ET seconds past J2000.
    pub end_et: f64,
    /// First double-word address of the segment data.
    pub start_word: usize,
    /// Last double-word address of the segment data.
    pub end_word: usize,
}

/// An opened SPK kernel.
pub struct SpkKernel {
    dafs: Vec<DafFile>,
    segments: Vec<SpkSegment>,
    segment_sources: Vec<usize>,
    lsk: Option<LeapSecondTable>,
}

impl SpkKernel {
    /// Opens a SPK kernel.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GeoError> {
        Self::open_all(&[path])
    }

    /// Opens kernels in SPICE load order. A later loaded file has priority
    /// for a body/epoch, and its last applicable segment wins within it.
    pub fn open_all(paths: &[impl AsRef<Path>]) -> Result<Self, GeoError> {
        let mut dafs = Vec::new();
        let mut segments = Vec::new();
        let mut segment_sources = Vec::new();
        for path in paths {
            let daf = DafFile::open(path)?;
            if daf.kind() != crate::daf::DafKind::Spk {
                return Err(GeoError::BadDaf("SPK loader requires a DAF/SPK container"));
            }
            if daf.nd() != 2 || daf.ni() != 6 {
                return Err(GeoError::BadDaf("SPK summaries require ND=2 and NI=6"));
            }
            for entry in daf.segments() {
                let start_word = entry.ic[4];
                let end_word = entry.ic[5];
                if start_word <= 0 || end_word < start_word {
                    return Err(GeoError::BadDaf("invalid segment word range"));
                }
                segments.push(SpkSegment {
                    name: entry.name.clone(),
                    target: entry.ic[0],
                    center: entry.ic[1],
                    frame: entry.ic[2],
                    data_type: entry.ic[3],
                    start_et: entry.dc[0],
                    end_et: entry.dc[1],
                    start_word: start_word as usize,
                    end_word: end_word as usize,
                });
                segment_sources.push(dafs.len());
            }
            dafs.push(daf);
        }
        if segments.is_empty() {
            return Err(GeoError::EmptyKernel);
        }
        Ok(Self {
            dafs,
            segments,
            segment_sources,
            lsk: None,
        })
    }

    /// Segments in kernel order.
    pub fn segments(&self) -> &[SpkSegment] {
        &self.segments
    }

    /// Supplies the explicit time context required by type10 TLE epochs.
    /// SPK10 selects Time's native scalar JDUTC convention, including
    /// historical packets, and requires IERS-only/TableDriven LSK settings.
    pub fn attach_lsk(&mut self, lsk: LeapSecondTable) {
        self.lsk = Some(lsk);
    }

    /// Geometric state of `target` relative to `observer` at `et`, in the
    /// J2000 frame.
    ///
    /// `et` is TDB seconds past J2000, matching SPICE. No light-time or
    /// aberration correction is applied.
    pub fn state(&self, target: i32, observer: i32, et: f64) -> Result<State, GeoError> {
        self.state_inertial(target, observer, et, 1)
    }

    /// Geometric state in a built-in NAIF inertial frame (IDs 1 through 21).
    /// Preserves SPKGEO's segment-frame accumulation and 20-node target buffer.
    pub fn state_inertial(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        frame: i32,
    ) -> Result<State, GeoError> {
        if target != observer {
            inertial_rotation(frame, frame)?;
        }
        self.state_with_transform(target, observer, et, frame, rotate_state_between)
    }

    /// SPKGEO in a named frame, retaining source segment-frame accumulation.
    /// The frame context also resolves non-inertial SPK segment references.
    pub fn state_in_frame(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        frames: &Frames,
        pck: Option<&Pck>,
        frame: &str,
    ) -> Result<State, GeoError> {
        let output = frames.frame_id(frame)?;
        self.state_with_transform(target, observer, et, output, |state, from, to| {
            if from == to {
                return Ok(state);
            }
            if (1..=21).contains(&from) && (1..=21).contains(&to) {
                return rotate_state_between(state, from, to);
            }
            let transform =
                frames.pxform_with_derivative(&from.to_string(), &to.to_string(), et, pck)?;
            let mut velocity = [0.0; 3];
            for row in 0..3 {
                for column in 0..3 {
                    velocity[row] += transform.rotation_dot[row][column] * state.position[column];
                }
                for column in 0..3 {
                    velocity[row] += transform.rotation[row][column] * state.velocity[column];
                }
            }
            Ok(State {
                position: rotate(&transform.rotation, state.position),
                velocity,
            })
        })
    }

    /// Position-only SPKEZP path, with named source/output frame context.
    /// Returns position (km) and one-way light time (seconds).
    #[allow(clippy::too_many_arguments)]
    pub fn position_corrected_in_frame(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        correction: Aberration,
        frames: &Frames,
        pck: Option<&Pck>,
        frame: &str,
    ) -> Result<([f64; 3], f64), GeoError> {
        crate::spkapo::corrected_position_in_frame(
            self, target, observer, et, correction, frames, frame, pck,
        )
    }

    /// SPKGPS position-only requested-frame composition (REFCHG).
    pub fn position_in_frame(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        frames: &Frames,
        pck: Option<&Pck>,
        frame: &str,
    ) -> Result<[f64; 3], GeoError> {
        let output = frames.frame_id(frame)?;
        let state =
            self.state_with_transform(target, observer, et, output, |state, from, to| {
                if from == to {
                    return Ok(state);
                }
                let rotation = if (1..=21).contains(&from) && (1..=21).contains(&to) {
                    inertial_rotation(from, to)?
                } else {
                    // SPKGPS uses REFCHG. A state transform can produce a
                    // different rotation for dynamic frames (e.g. EUL2XF's
                    // adjacent-axis reduction), even when velocity is discarded.
                    frames.pxform_in(&from.to_string(), &to.to_string(), et, pck)?
                };
                Ok(State {
                    position: rotate(&rotation, state.position),
                    velocity: [0.0; 3],
                })
            })?;
        Ok(state.position)
    }

    fn state_with_transform<F>(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        frame: i32,
        transform: F,
    ) -> Result<State, GeoError>
    where
        F: Fn(State, i32, i32) -> Result<State, GeoError>,
    {
        if target == observer {
            return Ok(State::ZERO);
        }
        if !et.is_finite() {
            return Err(GeoError::BadSearch("epoch must be finite"));
        }
        let mut nodes = vec![target];
        let mut states = vec![State::ZERO];
        let mut ids = vec![1];
        let mut visited = HashSet::new();
        let mut current = target;
        while current != 0 && current != observer {
            if !visited.insert(current) {
                return Err(GeoError::BadDaf("cyclic SPK target center chain"));
            }
            let Some(index) = self.selected_segment(current, et) else {
                break;
            };
            let segment = &self.segments[index];
            let edge = self.evaluate_segment(index, et)?;
            current = segment.center;
            if nodes.len() < 20 {
                nodes.push(current);
                states.push(edge);
                ids.push(segment.frame);
            } else {
                states[19] = transform(states[19], ids[19], segment.frame)? + edge;
                nodes[19] = current;
                ids[19] = segment.frame;
            }
        }
        if ids.len() > 1 {
            ids[0] = ids[1];
        }
        let mut obs = observer;
        let mut obs_state = State::ZERO;
        let mut obs_frame = None;
        visited.clear();
        let mut common = if nodes.last() == Some(&obs) {
            Some(nodes.len() - 1)
        } else {
            None
        };
        if let Some(index) = common {
            obs_frame = Some(ids[index]);
        }
        while common.is_none() && obs != 0 {
            if !visited.insert(obs) {
                return Err(GeoError::BadDaf("cyclic SPK observer center chain"));
            }
            let Some(index) = self.selected_segment(obs, et) else {
                break;
            };
            let segment = &self.segments[index];
            let edge = self.evaluate_segment(index, et)?;
            obs_state = match obs_frame {
                None => edge,
                Some(previous) => transform(obs_state, previous, segment.frame)? + edge,
            };
            obs_frame = Some(segment.frame);
            obs = segment.center;
            common = nodes.iter().position(|id| *id == obs);
        }
        let common = common.ok_or(GeoError::NoPath { target, observer })?;
        let mut obs_frame = obs_frame.unwrap_or(ids[common]);
        if common == 0 {
            ids[0] = obs_frame;
        }
        for index in 1..common {
            states[index + 1] =
                transform(states[index], ids[index], ids[index + 1])? + states[index + 1];
        }
        let result = if ids[common] == obs_frame {
            states[common] - obs_state
        } else if ids[common] == frame {
            obs_state = transform(obs_state, obs_frame, frame)?;
            obs_frame = frame;
            states[common] - obs_state
        } else {
            transform(states[common], ids[common], obs_frame)? - obs_state
        };
        transform(result, obs_frame, frame)
    }

    /// Geometric state at an `Epoch`, converting TAI to ET with the
    /// supplied leap-second kernel.
    pub fn state_at_epoch(
        &self,
        target: i32,
        observer: i32,
        epoch: Epoch,
        lsk: &LeapSecondTable,
    ) -> Result<State, GeoError> {
        self.state(target, observer, tai_to_et(epoch, lsk))
    }

    /// Apparent state of `target` relative to `observer`, corrected for
    /// light time and/or stellar aberration exactly as CSPICE
    /// `spkezr` does.
    ///
    /// The observer and target are both evaluated relative to the solar
    /// system barycenter (NAIF id 0), so the kernel must connect both
    /// bodies to the barycenter. This is the same requirement CSPICE has
    /// for stellar aberration corrections.
    pub fn state_corrected(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        abcorr: Aberration,
    ) -> Result<CorrectedState, GeoError> {
        self.state_corrected_with_geometry(target, observer, et, abcorr, |t, o, epoch| {
            self.state(t, o, epoch)
        })
    }

    fn state_corrected_with_geometry<F>(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        abcorr: Aberration,
        geometry: F,
    ) -> Result<CorrectedState, GeoError>
    where
        F: Fn(i32, i32, f64) -> Result<State, GeoError>,
    {
        let (use_light_time, converged, stellar, transmission) = abcorr.flags();

        if !use_light_time {
            // SPKEZ's geometric branch calls SPKGEO directly. Subtracting
            // two SSB states instead loses the common-center cancellation.
            let state = geometry(target, observer, et)?;
            let distance = norm(state.position);
            return Ok(CorrectedState {
                state,
                light_time: distance / CLIGHT,
                light_time_rate: if distance == 0.0 {
                    0.0
                } else {
                    dot(state.position, state.velocity) / (distance * CLIGHT)
                },
            });
        }

        let observer_state = geometry(observer, 0, et)?;
        let (mut relative, light_time, light_time_rate) = self.light_time_state_with(
            et,
            &observer_state,
            |epoch| geometry(target, 0, epoch),
            use_light_time,
            converged,
            transmission,
        )?;

        if stellar {
            let before = geometry(observer, 0, et - 1.0)?.velocity;
            let after = geometry(observer, 0, et + 1.0)?.velocity;
            let acceleration = scale(subtract(after, before), 0.5);
            relative = Self::aberrate(
                transmission,
                acceleration,
                observer_state.velocity,
                &relative,
            );
        }

        Ok(CorrectedState {
            state: relative,
            light_time,
            light_time_rate,
        })
    }

    /// Apparent state at an `Epoch`, converting TAI to ET with the
    /// supplied leap-second kernel.
    pub fn state_at_epoch_corrected(
        &self,
        target: i32,
        observer: i32,
        epoch: Epoch,
        abcorr: Aberration,
        lsk: &LeapSecondTable,
    ) -> Result<CorrectedState, GeoError> {
        self.state_corrected(target, observer, tai_to_et(epoch, lsk), abcorr)
    }

    /// Apparent state in a reference frame that is not necessarily
    /// inertial, matching CSPICE `spkezr(target, et, frame, abcorr, obs)`.
    ///
    /// CSPICE evaluates a body-fixed output frame at the epoch at which the
    /// **observer** sees the **frame's centre**, `et + sigma*lt(observer,
    /// centre)`, with `sigma` equal to `-1` for reception corrections and
    /// `+1` for the `X`-prefixed transmission ones. When the observer is the
    /// centre itself the shift is zero. Because that epoch moves with the
    /// observer, the body-fixed velocity picks up the frame's rotation at
    /// the rate `1 + sigma*dlt/dt`:
    ///
    /// ```text
    /// position = M(e) r
    /// velocity = M(e) v + (1 + sigma dlt/dt) M'(e) r
    /// ```
    ///
    /// with `e = et + sigma*lt(observer, centre)`.
    ///
    /// Note that this is **not** the observer-target light time. The two
    /// coincide only when the target is the frame's centre, which is the
    /// common case but not the general one; using the wrong one rotates the
    /// result about the frame's spin axis by the difference. The rule was
    /// established against CSPICE `spkezp` with the target and the frame
    /// centre deliberately chosen to differ.
    ///
    /// [`FrameSource::Fixed`] evaluates inertial corrections in the requested
    /// frame. Constant class-4 TKFRAMEs use [`FrameSource::FixedNonInertial`],
    /// which evaluates corrections in J2000 before rotating the result.
    pub fn apparent_state_in(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        abcorr: Aberration,
        frame: &FrameSource<'_>,
    ) -> Result<CorrectedState, GeoError> {
        if let FrameSource::BodyFixed { pck, body } = frame {
            let frames = Frames::new();
            let frame_name = format!("IAU_{body}");
            if frames.frame_id(&frame_name).is_ok() {
                return self.apparent_state_in(
                    target,
                    observer,
                    et,
                    abcorr,
                    &FrameSource::Named {
                        frames: &frames,
                        pck,
                        frame_name: &frame_name,
                    },
                );
            }
        }
        if let FrameSource::Fixed(matrix) = frame {
            return self.state_corrected_inertial(target, observer, et, abcorr, matrix);
        }
        if let FrameSource::Named {
            frames,
            pck,
            frame_name,
        } = frame
        {
            if abcorr == Aberration::None {
                let state =
                    self.state_in_frame(target, observer, et, frames, Some(pck), frame_name)?;
                let distance = norm(state.position);
                return Ok(CorrectedState {
                    state,
                    light_time: distance / CLIGHT,
                    light_time_rate: if distance == 0.0 {
                        0.0
                    } else {
                        dot(state.position, state.velocity) / (distance * CLIGHT)
                    },
                });
            }
            if frames.is_inertial(frame_name)? {
                return self.state_corrected_with_geometry(
                    target,
                    observer,
                    et,
                    abcorr,
                    |t, o, epoch| self.state_in_frame(t, o, epoch, frames, Some(pck), frame_name),
                );
            }
        }
        let corrected_at = |t, o| match frame {
            FrameSource::Named { frames, pck, .. } => {
                self.state_corrected_with_geometry(t, o, et, abcorr, |target, observer, epoch| {
                    self.state_in_frame(target, observer, epoch, frames, Some(pck), "J2000")
                })
            }
            _ => self.state_corrected(t, o, et, abcorr),
        };
        let corrected = corrected_at(target, observer)?;
        let (use_light_time, _, _, transmission) = abcorr.flags();
        let sign = if transmission { 1.0 } else { -1.0 };

        let (epoch, factor) = if use_light_time {
            match frame.center() {
                Some(center) if center != observer => {
                    let to_center = corrected_at(center, observer)?;
                    (
                        et + sign * to_center.light_time,
                        1.0 + sign * to_center.light_time_rate,
                    )
                }
                _ => (et, 1.0),
            }
        } else {
            (et, 1.0)
        };

        let (matrix, matrix_dot) = frame.at(epoch)?;
        let position = rotate(&matrix, corrected.state.position);
        let mut velocity = [0.0; 3];
        for row in 0..3 {
            // SPKEZ scales DR entry by entry, then MXVG accumulates the
            // derivative and velocity blocks in their six-column order.
            for column in 0..3 {
                velocity[row] +=
                    (factor * matrix_dot[row][column]) * corrected.state.position[column];
            }
            for column in 0..3 {
                velocity[row] += matrix[row][column] * corrected.state.velocity[column];
            }
        }

        Ok(CorrectedState {
            state: State { position, velocity },
            light_time: if use_light_time {
                corrected.light_time
            } else {
                norm(position) / CLIGHT
            },
            light_time_rate: corrected.light_time_rate,
        })
    }

    /// SPKACS evaluates all input states and acceleration samples in the
    /// requested inertial frame before light time and stellar corrections.
    pub(crate) fn state_corrected_inertial(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        abcorr: Aberration,
        matrix: &Mat3,
    ) -> Result<CorrectedState, GeoError> {
        let (light_time, converged, stellar, transmission) = abcorr.flags();
        if !light_time {
            let state = self.geometric_in_matrix(target, observer, et, matrix)?;
            let distance = norm(state.position);
            return Ok(CorrectedState {
                state,
                light_time: distance / CLIGHT,
                light_time_rate: if distance == 0.0 {
                    0.0
                } else {
                    dot(state.position, state.velocity) / (distance * CLIGHT)
                },
            });
        }
        let observer_state = self.geometric_in_matrix(observer, 0, et, matrix)?;
        let (mut state, lt, rate) = self.light_time_state_with(
            et,
            &observer_state,
            |epoch| self.geometric_in_matrix(target, 0, epoch, matrix),
            true,
            converged,
            transmission,
        )?;
        if stellar {
            let before = self
                .geometric_in_matrix(observer, 0, et - 1.0, matrix)?
                .velocity;
            let after = self
                .geometric_in_matrix(observer, 0, et + 1.0, matrix)?
                .velocity;
            let acceleration = scale(subtract(after, before), 0.5);
            state = Self::aberrate(transmission, acceleration, observer_state.velocity, &state);
        }
        Ok(CorrectedState {
            state,
            light_time: lt,
            light_time_rate: rate,
        })
    }

    pub(crate) fn geometric_in_matrix(
        &self,
        target: i32,
        observer: i32,
        et: f64,
        matrix: &Mat3,
    ) -> Result<State, GeoError> {
        if let Some(frame) = inertial_id(matrix) {
            return self.state_inertial(target, observer, et, frame);
        }
        let state = self.state(target, observer, et)?;
        Ok(State {
            position: rotate(matrix, state.position),
            velocity: rotate(matrix, state.velocity),
        })
    }

    pub(crate) fn absolute_state(&self, body: i32, et: f64) -> Result<State, GeoError> {
        self.state(body, 0, et)
    }

    /// Light-time solution for an arbitrary target trajectory.
    ///
    /// `target_state_at` evaluates the target's position and velocity
    /// relative to the solar system barycenter at a given epoch. It is a
    /// closure so that a point location with a constant velocity can be
    /// corrected by the same iteration CSPICE applies to an ephemeris
    /// object.
    pub(crate) fn light_time_state_with<F>(
        &self,
        et: f64,
        observer_state: &State,
        target_state_at: F,
        use_light_time: bool,
        converged: bool,
        transmission: bool,
    ) -> Result<(State, f64, f64), GeoError>
    where
        F: Fn(f64) -> Result<State, GeoError>,
    {
        let target_state = target_state_at(et)?;
        let mut relative = target_state - *observer_state;
        let distance = norm(relative.position);
        let mut light_time = distance / CLIGHT;

        if light_time == 0.0 {
            return Ok((relative, 0.0, 0.0));
        }

        if !use_light_time {
            let rate = dot(relative.position, relative.velocity) / (distance * CLIGHT);
            return Ok((relative, light_time, rate));
        }

        let sign = if transmission { 1.0 } else { -1.0 };
        let iterations = if converged { 5 } else { 1 };
        let mut retarded = target_state;
        let mut error = 1.0;
        let mut step = 0;

        while step < iterations && error > 1e-17 {
            let epoch = et + sign * light_time;
            retarded = target_state_at(epoch)?;
            relative = retarded - *observer_state;

            let previous = light_time;
            light_time = norm(relative.position) / CLIGHT;
            let time_scale = f64::max(1.0, epoch.abs());
            error = (light_time - previous).abs() / time_scale;
            step += 1;
        }

        let a = 1.0 / (CLIGHT * norm(relative.position));
        let b = dot(relative.position, relative.velocity);
        let c = dot(relative.position, retarded.velocity);
        let rate = a * b / (1.0 - sign * c * a);

        relative.velocity = subtract(
            scale(retarded.velocity, 1.0 + sign * rate),
            observer_state.velocity,
        );

        Ok((relative, light_time, rate))
    }

    /// Applies stellar aberration to a relative state.
    pub(crate) fn stellar_offsets(
        transmission: bool,
        acceleration: [f64; 3],
        observer_velocity: [f64; 3],
        relative: &State,
    ) -> ([f64; 3], [f64; 3]) {
        stellar_aberration(transmission, acceleration, observer_velocity, relative)
    }

    /// Applies stellar aberration to a relative state.
    pub(crate) fn aberrate(
        transmission: bool,
        acceleration: [f64; 3],
        observer_velocity: [f64; 3],
        relative: &State,
    ) -> State {
        let (position, velocity) =
            stellar_aberration(transmission, acceleration, observer_velocity, relative);
        State {
            position: add(relative.position, position),
            velocity: add(relative.velocity, velocity),
        }
    }

    fn selected_segment(&self, body: i32, et: f64) -> Option<usize> {
        // SPKSFS: the last applicable segment for this target wins,
        // independent of its center. Unsupported newer segments must not
        // silently fall back to stale data from an older segment.
        self.segments.iter().rposition(|segment| {
            segment.target == body && et >= segment.start_et && et <= segment.end_et
        })
    }

    /// SPKR08/SPKR09 record selection, shared with types12/13.
    fn evaluate_interpolated_segment(&self, index: usize, et: f64) -> Result<State, GeoError> {
        let segment = &self.segments[index];
        let daf = &self.dafs[self.segment_sources[index]];
        let uniform = matches!(segment.data_type, 8 | 12);
        let hermite = matches!(segment.data_type, 12 | 13);
        let control = daf.read_doubles(
            segment.end_word - if uniform { 3 } else { 1 },
            if uniform { 4 } else { 2 },
        )?;
        let degree = control[if uniform { 2 } else { 0 }].round() as usize;
        let n = control[if uniform { 3 } else { 1 }].round() as usize;
        let size = degree + 1;
        if n < size || size > 33 || n == 0 {
            return Err(GeoError::BadDaf("invalid SPK interpolation control data"));
        }
        let (first, start, step) = if uniform {
            let start = control[0];
            let step = control[1];
            if !step.is_finite() || step <= 0.0 {
                return Err(GeoError::BadDaf("invalid SPK interpolation step"));
            }
            let location = (et - start) / step;
            let center = if size % 2 == 1 {
                location.round() as i64 + 1
            } else {
                location.trunc() as i64 + 1
            };
            let first = (center - degree as i64 / 2).max(1).min((n - degree) as i64) as usize;
            (first, start + (first - 1) as f64 * step, step)
        } else {
            let ndir = (n - 1) / 100;
            let dirbase = segment.end_word - ndir - 2;
            let group = if ndir == 0 {
                1
            } else {
                let (mut base, mut read, mut remaining) =
                    (dirbase, ndir.min(101), ndir - ndir.min(101));
                let mut buffer = daf.read_doubles(base + 1, read)?;
                while buffer[read - 1] < et && remaining > 0 {
                    base += read;
                    read = remaining.min(101);
                    remaining -= read;
                    buffer = daf.read_doubles(base + 1, read)?;
                }
                base - dirbase + buffer.partition_point(|x| *x < et) + 1
            };
            let begin = if group == 1 { 1 } else { (group - 1) * 100 };
            let end = if group == 1 {
                n.min(100)
            } else {
                (begin + 100).min(n)
            };
            let timebase = dirbase - n;
            let buffer = daf.read_doubles(timebase + begin, end - begin + 1)?;
            let i = buffer.partition_point(|x| *x < et);
            let low = if i == 0 { 1 } else { begin + i - 1 };
            let center = if size % 2 == 1 {
                if i == 0 || (et - buffer[i - 1]).abs() < (et - buffer[i]).abs() {
                    low
                } else {
                    low + 1
                }
            } else {
                low
            };
            (
                (center as i64 - degree as i64 / 2)
                    .max(1)
                    .min((n - degree) as i64) as usize,
                0.0,
                0.0,
            )
        };
        let records = daf.read_doubles(segment.start_word + (first - 1) * 6, size * 6)?;
        let epochs = if uniform {
            Vec::new()
        } else {
            daf.read_doubles(segment.start_word + n * 6 + first - 1, size)?
        };
        let mut output = [0.0; 6];
        if hermite {
            for component in 0..3 {
                let values: Vec<_> = (0..size)
                    .flat_map(|i| [records[i * 6 + component], records[i * 6 + component + 3]])
                    .collect();
                let (position, velocity) = if uniform {
                    hermite_uniform(&values, start, step, et)
                } else {
                    hermite_irregular(&epochs, &values, et)?
                };
                output[component] = position;
                output[component + 3] = velocity;
            }
        } else {
            for component in 0..6 {
                let values: Vec<_> = (0..size).map(|i| records[i * 6 + component]).collect();
                output[component] = if uniform {
                    lagrange_uniform(values, start, step, et)
                } else {
                    lagrange_irregular(&epochs, values, et)?
                };
            }
        }
        Ok(State {
            position: [output[0], output[1], output[2]],
            velocity: [output[3], output[4], output[5]],
        })
    }

    fn evaluate_segment(&self, index: usize, et: f64) -> Result<State, GeoError> {
        let segment = &self.segments[index];
        let daf = &self.dafs[self.segment_sources[index]];
        if et < segment.start_et || et > segment.end_et {
            return Err(GeoError::TimeOutOfRange {
                target: segment.target,
                center: segment.center,
                et,
            });
        }

        match segment.data_type {
            10 => return self.evaluate_type10(index, et),
            14 => return self.evaluate_type14(index, et),
            15 => {
                return crate::two_body::evaluate_type15(
                    &daf.read_doubles(
                        segment.start_word,
                        segment.end_word - segment.start_word + 1,
                    )?,
                    et,
                )
            }
            5 => return self.evaluate_type5(index, et),
            1 | 21 => return self.evaluate_mda(index, et),
            19 => return self.evaluate_type19(index, et),
            18 => return self.evaluate_type18(index, et),
            17 => {
                return crate::spk_equinoctial::evaluate(
                    &daf.read_doubles(
                        segment.start_word,
                        segment.end_word - segment.start_word + 1,
                    )?,
                    et,
                )
            }
            20 => return self.evaluate_type20(index, et),
            8 | 9 | 12 | 13 => return self.evaluate_interpolated_segment(index, et),
            2 | 3 => {}
            other => return Err(GeoError::UnsupportedType(other)),
        }

        // Directory at the end of the segment: INIT, INTLEN, RSIZE, N.
        let directory = daf.read_doubles(segment.end_word - 3, 4)?;
        let init = directory[0];
        let interval = directory[1];
        let record_size = directory[2] as usize;
        let count = directory[3] as usize;

        if interval <= 0.0 || record_size < 3 || count == 0 {
            return Err(GeoError::BadDaf("invalid Chebyshev directory"));
        }

        let components = match segment.data_type {
            2 => 3,
            3 => 6,
            other => return Err(GeoError::UnsupportedType(other)),
        };
        let per_component = (record_size - 2) / components;
        if per_component == 0 || (record_size - 2) % components != 0 {
            return Err(GeoError::BadDaf("record size is inconsistent with type"));
        }

        let index =
            (((et - init) / interval).floor() as isize).clamp(0, count as isize - 1) as usize;
        let record_word = segment.start_word + index * record_size;
        let record = daf.read_doubles(record_word, record_size)?;

        let midpoint = record[0];
        let radius = record[1];
        if !radius.is_finite() || radius <= 0.0 {
            return Err(GeoError::BadDaf(
                "Chebyshev radius must be positive and finite",
            ));
        }
        let tau = (et - midpoint) / radius;

        let mut state = State::ZERO;
        for axis in 0..3 {
            let offset = 2 + axis * per_component;
            let coefficients = &record[offset..offset + per_component];
            if segment.data_type == 2 {
                let (position, derivative) = chebyshev_with_derivative(coefficients, tau);
                state.position[axis] = position;
                state.velocity[axis] = derivative / radius;
            } else {
                state.position[axis] = chebyshev(coefficients, tau);
                let velocity_offset = 2 + (3 + axis) * per_component;
                state.velocity[axis] = chebyshev(
                    &record[velocity_offset..velocity_offset + per_component],
                    tau,
                );
            }
        }

        Ok(state)
    }

    /// SPKR20/SPKE20: scaled velocity coefficients with midpoint positions.
    fn evaluate_type20(&self, index: usize, et: f64) -> Result<State, GeoError> {
        let segment = &self.segments[index];
        let daf = &self.dafs[self.segment_sources[index]];
        let control = daf.read_doubles(segment.end_word - 6, 7)?;
        let (distance_scale, time_scale, initial_jd, initial_fraction, days) =
            (control[0], control[1], control[2], control[3], control[4]);
        let size = control[5] as usize;
        let count = control[6] as usize;
        if size < 6 || size % 3 != 0 || count == 0 || days <= 0.0 || time_scale <= 0.0 {
            return Err(GeoError::BadDaf("invalid type20 directory"));
        }
        let initial = (initial_jd - 2451545.0 + initial_fraction) * 86400.0;
        let interval = days * 86400.0;
        let record_number = (((et - initial) / interval) as i64 + 1)
            .max(1)
            .min(count as i64) as usize;
        let begin = (initial_jd - 2451545.0 + (record_number - 1) as f64 * days) * 86400.0;
        let radius = interval / 2.0;
        let midpoint = begin + initial_fraction * 86400.0 + radius;
        let record = daf.read_doubles(segment.start_word + (record_number - 1) * size, size)?;
        let terms = size / 3;
        let scale = distance_scale / time_scale;
        let mut state = State::ZERO;
        for axis in 0..3 {
            let coefficients: Vec<_> = record[axis * terms..(axis + 1) * terms - 1]
                .iter()
                .map(|value| value * scale)
                .collect();
            let (velocity, integral) = chebyshev_integral(&coefficients, midpoint, radius, et);
            state.position[axis] = record[(axis + 1) * terms - 1] * distance_scale + integral;
            state.velocity[axis] = velocity;
        }
        Ok(state)
    }

    /// SPKR18/SPKE18 use clipped, potentially odd windows near segment ends.
    fn evaluate_type18(&self, index: usize, et: f64) -> Result<State, GeoError> {
        let segment = &self.segments[index];
        let daf = &self.dafs[self.segment_sources[index]];
        self.evaluate_packet_segment(daf, segment.start_word, segment.end_word, et, false)
    }

    fn evaluate_type19(&self, index: usize, et: f64) -> Result<State, GeoError> {
        let segment = &self.segments[index];
        let daf = &self.dafs[self.segment_sources[index]];
        let control = daf.read_doubles(segment.end_word - 1, 2)?;
        let select_last = control[0].round() == 1.0;
        let n = control[1].round() as usize;
        if n == 0 {
            return Err(GeoError::BadDaf("empty type19 interval directory"));
        }
        let pointer_base = segment.end_word - 2 - (n + 1);
        let directory_base = pointer_base - n / 100;
        let intervals = daf.read_doubles(directory_base - n, n + 1)?;
        let index = if select_last {
            intervals.partition_point(|v| *v <= et)
        } else {
            intervals.partition_point(|v| *v < et)
        }
        .max(1)
        .min(n)
            - 1;
        let pointers = daf.read_doubles(pointer_base + 1 + index, 2)?;
        let begin = segment.start_word + pointers[0].round() as usize - 1;
        let end = segment.start_word + pointers[1].round() as usize - 2;
        self.evaluate_packet_segment(daf, begin, end, et, true)
    }

    fn evaluate_mda(&self, index: usize, et: f64) -> Result<State, GeoError> {
        let segment = &self.segments[index];
        let daf = &self.dafs[self.segment_sources[index]];
        let control_count = if segment.data_type == 1 { 1 } else { 2 };
        let control = daf.read_doubles(segment.end_word + 1 - control_count, control_count)?;
        let n = control[control_count - 1].round() as usize;
        let dimension = if segment.data_type == 1 {
            15
        } else {
            control[0].round() as usize
        };
        if n == 0 || !(1..=25).contains(&dimension) {
            return Err(GeoError::BadDaf("invalid difference-line directory"));
        }
        let epoch_start = segment.end_word + 1 - control_count - n / 100 - n;
        let epochs = daf.read_doubles(epoch_start, n)?;
        let index = epochs.partition_point(|value| *value < et);
        if index >= n {
            return Err(GeoError::BadDaf("difference-line coverage gap"));
        }
        let size = 4 * dimension + 11;
        let record = daf.read_doubles(segment.start_word + index * size, size)?;
        crate::spk_mda::evaluate(&record, dimension, et)
    }

    fn evaluate_type5(&self, index: usize, et: f64) -> Result<State, GeoError> {
        let segment = &self.segments[index];
        let daf = &self.dafs[self.segment_sources[index]];
        let control = daf.read_doubles(segment.end_word - 1, 2)?;
        let gm = control[0];
        let n = control[1].round() as usize;
        if n == 0 {
            return Err(GeoError::BadDaf("empty type5 segment"));
        }
        let epochs = daf.read_doubles(segment.start_word + n * 6, n)?;
        let i = epochs.partition_point(|value| *value < et);
        let (first, last) = if i == 0 {
            (0, 0)
        } else if i == n {
            (n - 1, n - 1)
        } else {
            (i - 1, i)
        };
        let read = |index| -> Result<State, GeoError> {
            let v = daf.read_doubles(segment.start_word + index * 6, 6)?;
            Ok(State {
                position: [v[0], v[1], v[2]],
                velocity: [v[3], v[4], v[5]],
            })
        };
        let s1 = crate::two_body::propagate(gm, read(first)?, et - epochs[first])?;
        if first == last {
            return Ok(s1);
        }
        let s2 = crate::two_body::propagate(gm, read(last)?, et - epochs[last])?;
        let argument = (et - epochs[first]) * std::f64::consts::PI / (epochs[last] - epochs[first]);
        let dargdt = std::f64::consts::PI / (epochs[last] - epochs[first]);
        let w = argument.cos() * 0.5 + 0.5;
        let dw = argument.sin() * -0.5 * dargdt;
        let position = std::array::from_fn(|i| w * s1.position[i] + (1.0 - w) * s2.position[i]);
        let velocity = std::array::from_fn(|i| {
            (w * s1.velocity[i] + (1.0 - w) * s2.velocity[i])
                + (dw * s1.position[i] + (-dw) * s2.position[i])
        });
        Ok(State { position, velocity })
    }

    fn evaluate_type14(&self, index: usize, et: f64) -> Result<State, GeoError> {
        let segment = &self.segments[index];
        let daf = &self.dafs[self.segment_sources[index]];
        let generic =
            crate::generic_segment::GenericSegment::new(daf, segment.start_word, segment.end_word)?;
        let coefficients = generic.constant(1)? as usize;
        let packet = generic
            .packet_at(et)?
            .ok_or(GeoError::BadDaf("type14 reference epoch missing"))?;
        if coefficients == 0
            || coefficients * 6 + 3 > 198
            || packet.len() != 2 + 6 * coefficients
            || packet[1] <= 0.0
        {
            return Err(GeoError::BadDaf("invalid type14 Chebyshev packet"));
        }
        let tau = (et - packet[0]) / packet[1];
        let mut output = [0.0; 6];
        for component in 0..6 {
            let start = 2 + component * coefficients;
            output[component] = chebyshev(&packet[start..start + coefficients], tau);
        }
        Ok(State {
            position: [output[0], output[1], output[2]],
            velocity: [output[3], output[4], output[5]],
        })
    }

    fn evaluate_type10(&self, index: usize, et: f64) -> Result<State, GeoError> {
        use stkcraft_time::contract::{NaifEt, TimeContext};
        let lsk = self.lsk.as_ref().ok_or_else(|| {
            GeoError::Unsupported("SPK10 requires an explicit leap-second/time context".into())
        })?;
        let segment = &self.segments[index];
        let daf = &self.dafs[self.segment_sources[index]];
        let generic =
            crate::generic_segment::GenericSegment::new(daf, segment.start_word, segment.end_word)?;
        let (index, reference) = generic
            .reference_at(et)?
            .ok_or(GeoError::BadDaf("type10 reference epoch missing"))?;
        let (from, to) = if et <= reference {
            (index.saturating_sub(1).max(1), index)
        } else {
            (index, (index + 1).min(generic.reference_count()))
        };
        let mut geophs = [0.0; 8];
        for (i, value) in geophs.iter_mut().enumerate() {
            *value = generic.constant(i + 1)?;
        }
        let first = generic.packet(from)?;
        let last = generic.packet(to)?;
        let evaluate = |packet: &[f64]| -> Result<[f64; 6], GeoError> {
            if packet.len() != 10 && packet.len() != 14 {
                return Err(GeoError::BadDaf("invalid type10 element packet"));
            }
            let elements: [f64; 10] = packet[..10].try_into().unwrap();
            // SPKE10 selects TTRANS's formal JDUTC convention, including
            // historical packets. Keep time in the independent Time crate
            // and avoid integer Epoch/civil-calendar quantization.
            let jd = TimeContext::new(lsk)
                .et_to_spice_julian_utc(NaifEt::from_seconds(elements[9])?)?
                .days();
            crate::sgp4::propagate(
                &geophs,
                &elements,
                1,
                (et - elements[9]) / 60.0,
                jd - 2433281.5,
            )
        };
        let first_state = evaluate(&first)?;
        let mut state = first_state;
        if first[9] != last[9] {
            let last_state = evaluate(&last)?;
            let denominator = last[9] - first[9];
            let arg = (et - first[9]) * std::f64::consts::PI / denominator;
            let derivative = std::f64::consts::PI / denominator;
            let weight = arg.cos() * 0.5 + 0.5;
            let rate = arg.sin() * -0.5 * derivative;
            for i in 0..6 {
                state[i] = weight * first_state[i] + (1.0 - weight) * last_state[i];
            }
            for i in 0..3 {
                state[i + 3] += rate * first_state[i] + (-rate) * last_state[i];
            }
        }
        let result = crate::earth_orientation::apply(crate::earth_orientation::teme(et)?, state);
        Ok(State {
            position: [result[0], result[1], result[2]],
            velocity: [result[3], result[4], result[5]],
        })
    }

    /// Common SPKR18/SPKR19 packet window and SPKE18/SPKE19 evaluation.
    fn evaluate_packet_segment(
        &self,
        daf: &DafFile,
        begin: usize,
        end: usize,
        et: f64,
        type19: bool,
    ) -> Result<State, GeoError> {
        let control = daf.read_doubles(end - 2, 3)?;
        let subtype = control[0].round() as i32;
        let window = control[1].round() as usize;
        let n = control[2].round() as usize;
        let packet_size = match subtype {
            0 => 12,
            1 => 6,
            2 if type19 => 6,
            _ => return Err(GeoError::BadDaf("invalid packet segment subtype")),
        };
        let maximum = if type19 {
            if subtype == 1 {
                28
            } else {
                14
            }
        } else if subtype == 0 {
            8
        } else {
            16
        };
        if n < 2 || window < 2 || window % 2 != 0 || window > maximum {
            return Err(GeoError::BadDaf(
                "invalid packet segment packet/window count",
            ));
        }
        let epochs = daf.read_doubles(begin + n * packet_size, n)?;
        let low = epochs.partition_point(|value| *value < et).max(1);
        let high = low + 1;
        let left = (window / 2).min(low);
        let right = (window / 2).min(n - high + 1);
        let first = low - left;
        let size = left + right;
        let times = &epochs[first..first + size];
        let packets = daf.read_doubles(begin + first * packet_size, size * packet_size)?;
        let mut output = [0.0; 6];
        if subtype == 2 {
            for component in 0..3 {
                let values: Vec<_> = (0..size)
                    .flat_map(|i| [packets[i * 6 + component], packets[i * 6 + component + 3]])
                    .collect();
                let (position, velocity) = hermite_irregular(times, &values, et)?;
                output[component] = position;
                output[component + 3] = velocity;
            }
        } else {
            for component in 0..6 {
                output[component] = if subtype == 1 {
                    let values = (0..size).map(|i| packets[i * 6 + component]).collect();
                    lagrange_irregular(times, values, et)?
                } else {
                    // Each subtype0 packet is [position, d(position)/dt,
                    // separately supplied velocity, d(velocity)/dt].
                    let offset = if component < 3 {
                        component
                    } else {
                        component + 3
                    };
                    let values: Vec<_> = (0..size)
                        .flat_map(|i| [packets[i * 12 + offset], packets[i * 12 + offset + 3]])
                        .collect();
                    hermite_irregular(times, &values, et)?.0
                };
            }
        }
        Ok(State {
            position: [output[0], output[1], output[2]],
            velocity: [output[3], output[4], output[5]],
        })
    }
}

/// CHBIGR's coupled Clenshaw evaluations; integral is zero at the midpoint.
pub(crate) fn chebyshev_integral(cp: &[f64], midpoint: f64, radius: f64, et: f64) -> (f64, f64) {
    let degree = cp.len() - 1;
    let s = (et - midpoint) / radius;
    let twice = s * 2.0;
    let a2 = if cp.len() >= 3 {
        cp[0] - cp[2] * 0.5
    } else {
        cp[0]
    };
    let a_degree1 = if degree >= 2 {
        cp[degree - 1] * 0.5 / degree as f64
    } else {
        0.0
    };
    let a_degree2 = if degree >= 1 {
        cp[degree] * 0.5 / (degree + 1) as f64
    } else {
        0.0
    };
    let mut f = [if degree == 0 { a2 } else { a_degree2 }, 0.0, 0.0];
    let mut z = f;
    let mut w = [0.0; 3];
    for i in (2..=cp.len()).rev() {
        let a = if i == 2 {
            a2
        } else if i < cp.len() {
            (cp[i - 2] - cp[i]) * 0.5 / (i - 1) as f64
        } else {
            a_degree1
        };
        f[2] = f[1];
        f[1] = f[0];
        f[0] = a + (twice * f[1] - f[2]);
        z[2] = z[1];
        z[1] = z[0];
        z[0] = a - z[2];
        w[2] = w[1];
        w[1] = w[0];
        w[0] = cp[i - 1] + (twice * w[1] - w[2]);
    }
    let integral = radius * (z[1] + s * f[0] - f[1]);
    (cp[0] + (s * w[0] - w[1]), integral)
}

/// CSPICE CHBVAL operation order for type 3 SPK records.
pub(crate) fn chebyshev(coefficients: &[f64], tau: f64) -> f64 {
    if coefficients.is_empty() {
        return 0.0;
    }
    let twice = tau * 2.0;
    let (mut current, mut previous) = (0.0, 0.0);
    for &coefficient in coefficients[1..].iter().rev() {
        let old_previous = previous;
        previous = current;
        current = coefficient + (twice * previous - old_previous);
    }
    tau * current - previous + coefficients[0]
}

/// CSPICE CHBINT evaluates value and derivative with coupled Clenshaw
/// recurrences. Forward summation is mathematically equivalent but is not
/// floating-point equivalent, and breaks reference-state equality.
fn chebyshev_with_derivative(coefficients: &[f64], tau: f64) -> (f64, f64) {
    if coefficients.is_empty() {
        return (0.0, 0.0);
    }
    let twice = tau * 2.0;
    let (mut current, mut previous) = (0.0, 0.0);
    let (mut derivative, mut previous_derivative) = (0.0, 0.0);
    for &coefficient in coefficients[1..].iter().rev() {
        let old_previous = previous;
        previous = current;
        current = coefficient + (twice * previous - old_previous);
        let old_derivative = previous_derivative;
        previous_derivative = derivative;
        derivative = previous * 2.0 + previous_derivative * twice - old_derivative;
    }
    (
        coefficients[0] + (tau * current - previous),
        current + tau * derivative - previous_derivative,
    )
}

#[cfg(test)]
fn chebyshev_derivative(coefficients: &[f64], tau: f64) -> f64 {
    chebyshev_with_derivative(coefficients, tau).1
}

/// CSPICE `ZZSTELAB`: corrections to add to a light-time corrected state
/// for stellar aberration.
fn stellar_aberration(
    transmission: bool,
    acceleration: [f64; 3],
    velocity: [f64; 3],
    state: &State,
) -> ([f64; 3], [f64; 3]) {
    let (observer_velocity, observer_acceleration) = if transmission {
        (negate(velocity), negate(acceleration))
    } else {
        (velocity, acceleration)
    };

    let position = state.position;
    let target_velocity = state.velocity;
    let (rhat, drhat) = unit_and_derivative(position, target_velocity);
    let perpendicular_component = perpendicular(observer_velocity, rhat);
    let range_rate = dot(target_velocity, rhat);

    let sine = norm(perpendicular_component) / CLIGHT;
    let cosine = (1.0 - sine * sine).max(0.0).sqrt();
    let perpendicular_hat = unit(perpendicular_component);
    let distance = norm(position);

    let position_correction = add(
        scale(perpendicular_hat, distance * sine),
        scale(rhat, distance * (cosine - 1.0)),
    );

    let velocity_correction = if sine >= 1e-6 {
        let perpendicular_rate = subtract(
            subtract(
                observer_acceleration,
                scale(
                    rhat,
                    dot(observer_velocity, drhat) + dot(observer_acceleration, rhat),
                ),
            ),
            scale(drhat, dot(observer_velocity, rhat)),
        );
        let (_, perpendicular_hat_rate) =
            unit_and_derivative(perpendicular_component, perpendicular_rate);
        let angle_rate = (1.0 / (cosine * CLIGHT)) * dot(perpendicular_rate, perpendicular_hat);

        let term1 = add(
            scale(perpendicular_hat_rate, sine),
            scale(perpendicular_hat, cosine * angle_rate),
        );
        let term2 = add(scale(drhat, cosine - 1.0), scale(rhat, -sine * angle_rate));
        let term3 = add(term1, term2);

        add(
            add(
                scale(term3, distance),
                scale(perpendicular_hat, range_rate * sine),
            ),
            scale(rhat, range_rate * (cosine - 1.0)),
        )
    } else {
        let mut offsets = [[0.0f64; 3]; 2];
        for (index, sign) in [-1.0f64, 1.0].into_iter().enumerate() {
            let estimated_velocity = add(observer_velocity, scale(observer_acceleration, sign));
            let estimated_position = add(position, scale(target_velocity, sign));
            let estimated_rhat = unit(estimated_position);
            let estimated_perpendicular = perpendicular(estimated_velocity, estimated_rhat);
            let estimated_sine = norm(estimated_perpendicular) / CLIGHT;
            let estimated_cosine = (1.0 - estimated_sine * estimated_sine).max(0.0).sqrt();
            let estimated_hat = unit(estimated_perpendicular);
            let estimated_distance = norm(estimated_position);
            offsets[index] = add(
                scale(estimated_hat, estimated_distance * estimated_sine),
                scale(
                    estimated_rhat,
                    estimated_distance * (estimated_cosine - 1.0),
                ),
            );
        }
        add(scale(offsets[1], 0.5), scale(offsets[0], -0.5))
    };

    (position_correction, velocity_correction)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn identity() -> Mat3 {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

fn rotate(matrix: &Mat3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn norm(a: [f64; 3]) -> f64 {
    let largest = a[0].abs().max(a[1].abs()).max(a[2].abs());
    if largest == 0.0 {
        0.0
    } else {
        let scaled = [a[0] / largest, a[1] / largest, a[2] / largest];
        largest * (scaled[0] * scaled[0] + scaled[1] * scaled[1] + scaled[2] * scaled[2]).sqrt()
    }
}

fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn negate(a: [f64; 3]) -> [f64; 3] {
    [-a[0], -a[1], -a[2]]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let length = norm(a);
    if length == 0.0 {
        [0.0; 3]
    } else {
        [a[0] / length, a[1] / length, a[2] / length]
    }
}

fn perpendicular(a: [f64; 3], axis: [f64; 3]) -> [f64; 3] {
    // VPERP scales both operands, then calls VPROJ on the scaled
    // vectors. Assuming AXIS is perfectly unit length changes the final
    // bits of the stellar-aberration derivative for slow observers.
    let ma = a[0].abs().max(a[1].abs()).max(a[2].abs());
    let mb = axis[0].abs().max(axis[1].abs()).max(axis[2].abs());
    if ma == 0.0 {
        return [0.0; 3];
    }
    if mb == 0.0 {
        return a;
    }
    let t = [a[0] / ma, a[1] / ma, a[2] / ma];
    let r = [axis[0] / mb, axis[1] / mb, axis[2] / mb];
    let amount = dot(t, r) / dot(r, r);
    scale(subtract(t, scale(r, amount)), ma)
}

/// CSPICE `DVHAT`: the unit vector of a state and its time derivative.
pub(crate) fn unit_and_derivative(position: [f64; 3], velocity: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let length = norm(position);
    if length == 0.0 {
        return ([0.0; 3], velocity);
    }
    let direction = position.map(|x| x / length);
    (
        direction,
        scale(perpendicular(velocity, direction), 1.0 / length),
    )
}

#[cfg(test)]
mod tests {
    use super::{chebyshev, chebyshev_derivative};

    #[test]
    fn chebyshev_linear() {
        let coefficients = [3.0, 0.5];
        assert!((chebyshev(&coefficients, 0.0) - 3.0).abs() < 1e-15);
        assert!((chebyshev(&coefficients, 0.5) - 3.25).abs() < 1e-15);
        assert!((chebyshev_derivative(&coefficients, 0.5) - 0.5).abs() < 1e-15);
    }

    #[test]
    fn chebyshev_quadratic() {
        // 1 + T2(tau) = 1 + (2 tau^2 - 1) = 2 tau^2.
        let coefficients = [1.0, 0.0, 1.0];
        assert!((chebyshev(&coefficients, 0.5) - 0.5).abs() < 1e-15);
        assert!((chebyshev(&coefficients, -0.25) - 0.125).abs() < 1e-15);
        // derivative = 4 tau.
        assert!((chebyshev_derivative(&coefficients, 0.5) - 2.0).abs() < 1e-15);
        assert!((chebyshev_derivative(&coefficients, -0.25) + 1.0).abs() < 1e-15);
    }
}

#[cfg(test)]
mod mixed_segment_frames {
    use super::*;
    #[test]
    fn exact_cspice_mixed_frame_geometric_chains() {
        let kernel = SpkKernel::open("tests/golden/spk_mixed_frames.bsp").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/spk_mixed_geometric.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let et = f[2].parse().unwrap();
            let result = kernel.state_inertial(
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                et,
                f[3].parse().unwrap(),
            );
            let expected: Vec<f64> = f[4..].iter().map(|x| x.parse().unwrap()).collect();
            match result {
                Ok(x) if [x.position, x.velocity].concat() == expected => {}
                other => errors.push(format!("{line}:actual{other:?}")),
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    #[test]
    fn exact_cspice_static_noninertial_output_frames() {
        let kernel = SpkKernel::open("target/kernels/de440s.bsp").unwrap();
        let frames = crate::frame::Frames::new();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/spk_static_output_frames.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let et = f[2].parse().unwrap();
            let matrix = frames.pxform("J2000", f[3], et).unwrap();
            let state = kernel
                .apparent_state_in(
                    f[0].parse().unwrap(),
                    f[1].parse().unwrap(),
                    et,
                    Aberration::parse(f[4]).unwrap(),
                    &FrameSource::FixedNonInertial {
                        matrix: &matrix,
                        center: 399,
                    },
                )
                .unwrap();
            let actual = [
                state.state.position.to_vec(),
                state.state.velocity.to_vec(),
                vec![state.light_time],
            ]
            .concat();
            let expected: Vec<f64> = f[5..].iter().map(|x| x.parse().unwrap()).collect();
            if actual != expected {
                errors.push(format!("{line}:actual{actual:?}"));
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    #[test]
    fn exact_cspice_mixed_frame_corrected_chains() {
        let kernel = SpkKernel::open("tests/golden/spk_mixed_frames.bsp").unwrap();
        let frames = crate::frame::Frames::new();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/spk_mixed_corrected.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let t = f[0].parse().unwrap();
            let o = f[1].parse().unwrap();
            let et = f[2].parse().unwrap();
            let correction = Aberration::parse(f[4]).unwrap();
            let matrix = frames.pxform("J2000", f[3], et).unwrap();
            let result = kernel
                .state_corrected_inertial(t, o, et, correction, &matrix)
                .and_then(|state| {
                    let (p, lt) = crate::spkapo::corrected_position_inertial(
                        &kernel, t, o, et, correction, &matrix,
                    )?;
                    Ok([
                        state.state.position.to_vec(),
                        state.state.velocity.to_vec(),
                        vec![state.light_time],
                        p.to_vec(),
                        vec![lt],
                    ]
                    .concat())
                });
            let expected: Vec<f64> = f[5..].iter().map(|x| x.parse().unwrap()).collect();
            match result {
                Ok(x) if x == expected => {}
                other => errors.push(format!("{line}:actual{other:?}")),
            }
            count += 1;
        }
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// IRFROT multiplies the saved J2000-to-frame matrices with MXMT.
fn inertial_rotation(from: i32, to: i32) -> Result<Mat3, GeoError> {
    for id in [from, to] {
        if !(1..=21).contains(&id) {
            return Err(GeoError::UnsupportedFrame(id));
        }
    }
    if from == to {
        return Ok(identity());
    }
    let a = crate::builtin_frames::FRAMES[(to - 1) as usize]
        .rotation
        .unwrap();
    let b = crate::builtin_frames::FRAMES[(from - 1) as usize]
        .rotation
        .unwrap();
    Ok(std::array::from_fn(|i| {
        std::array::from_fn(|j| a[0][i] * b[0][j] + a[1][i] * b[1][j] + a[2][i] * b[2][j])
    }))
}
fn rotate_state_between(state: State, from: i32, to: i32) -> Result<State, GeoError> {
    if from == to {
        inertial_rotation(from, to)?;
        return Ok(state);
    }
    let matrix = inertial_rotation(from, to)?;
    Ok(State {
        position: rotate(&matrix, state.position),
        velocity: rotate(&matrix, state.velocity),
    })
}
fn inertial_id(matrix: &Mat3) -> Option<i32> {
    (1..=21).find(|id| inertial_rotation(1, *id).ok().as_ref() == Some(matrix))
}

#[cfg(test)]
mod interpolated_segments {
    use super::*;
    #[test]
    fn native_interpolated_spk_states_exact() {
        let kernel = SpkKernel::open("tests/golden/spk_interpolated.bsp").unwrap();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/spk_interpolated_geometric.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let expected: Vec<f64> = f[3..].iter().map(|v| v.parse().unwrap()).collect();
            let actual = kernel
                .state_inertial(
                    f[0].parse().unwrap(),
                    0,
                    f[1].parse().unwrap(),
                    f[2].parse().unwrap(),
                )
                .map(|x| [x.position.to_vec(), x.velocity.to_vec()].concat());
            match actual {
                Ok(x) if x == expected => {}
                other => errors.push(format!("{} {} {}:actual{other:?}", f[0], f[1], f[2])),
            };
            count += 1;
        }
        assert_eq!(count, 12621);
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    #[test]
    fn native_interpolated_spk_corrections_exact() {
        let kernel = SpkKernel::open("tests/golden/spk_interpolated.bsp").unwrap();
        let frames = crate::frame::Frames::new();
        let mut errors = Vec::new();
        let mut count = 0;
        for line in include_str!("../../tests/golden/spk_interpolated_corrected.csv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split(',').collect();
            let t = f[0].parse().unwrap();
            let o = f[1].parse().unwrap();
            let et = f[2].parse().unwrap();
            let correction = Aberration::parse(f[4]).unwrap();
            let matrix = frames.pxform("J2000", f[3], et).unwrap();
            let expected: Vec<f64> = f[5..].iter().map(|v| v.parse().unwrap()).collect();
            let actual = kernel
                .state_corrected_inertial(t, o, et, correction, &matrix)
                .and_then(|x| {
                    let (p, lt) = crate::spkapo::corrected_position_inertial(
                        &kernel, t, o, et, correction, &matrix,
                    )?;
                    Ok([
                        x.state.position.to_vec(),
                        x.state.velocity.to_vec(),
                        vec![x.light_time],
                        p.to_vec(),
                        vec![lt],
                    ]
                    .concat())
                });
            match actual {
                Ok(x) if x == expected => {}
                other => errors.push(format!(
                    "{} {} {} {}:actual{other:?}",
                    f[0], f[1], f[2], f[3]
                )),
            };
            count += 1;
        }
        assert_eq!(count, 6048);
        assert!(
            errors.is_empty(),
            "{} mismatches/{count}:\n{}",
            errors.len(),
            errors
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    #[test]
    fn unsupported_new_segment_type_masks_older_supported_segment() {
        let kernel = SpkKernel::open("tests/golden/spk_interpolated.bsp").unwrap();
        assert!(matches!(
            kernel.state(990001, 0, 0.0),
            Err(GeoError::UnsupportedType(999))
        ));
    }
}

/// Source LGRESP: Neville columns in the shifted, unit-step abscissa.
fn lagrange_uniform(mut work: Vec<f64>, first: f64, step: f64, x: f64) -> f64 {
    let newx = (x - first) / step + 1.0;
    let n = work.len();
    for j in 1..n {
        for i in 0..n - j {
            let c1 = (i + 1 + j) as f64 - newx;
            let c2 = newx - (i + 1) as f64;
            work[i] = (c1 * work[i] + c2 * work[i + 1]) / j as f64;
        }
    }
    work[0]
}
/// Source LGRINT preserves its signed denominator and product ordering.
pub(crate) fn lagrange_irregular(
    epochs: &[f64],
    mut work: Vec<f64>,
    x: f64,
) -> Result<f64, GeoError> {
    let n = work.len();
    for j in 1..n {
        for i in 0..n - j {
            let denominator = epochs[i] - epochs[i + j];
            if denominator == 0.0 {
                return Err(GeoError::BadDaf("duplicate SPK interpolation epochs"));
            }
            let c1 = x - epochs[i + j];
            let c2 = epochs[i] - x;
            work[i] = (c1 * work[i] + c2 * work[i + 1]) / denominator;
        }
    }
    Ok(work[0])
}
/// Source HRMESP scales its abscissa and derivative values before the table.
fn hermite_uniform(values: &[f64], first: f64, step: f64, x: f64) -> (f64, f64) {
    let n = values.len() / 2;
    let x = (x - first) / step + 1.0;
    let mut work = values.to_vec();
    let mut rates = vec![0.0; 2 * n];
    for i in (1..2 * n).step_by(2) {
        work[i] *= step;
    }
    for i in 1..n {
        let prev = 2 * i - 2;
        let this = prev + 1;
        let next = this + 1;
        let c1 = (i + 1) as f64 - x;
        let c2 = x - i as f64;
        rates[prev] = work[this];
        rates[this] = work[next] - work[prev];
        let temp = work[this] * (x - i as f64) + work[prev];
        work[this] = c1 * work[prev] + c2 * work[next];
        work[prev] = temp;
    }
    rates[2 * n - 2] = work[2 * n - 1];
    work[2 * n - 2] = work[2 * n - 1] * (x - n as f64) + work[2 * n - 2];
    for j in 2..2 * n {
        for i in 0..2 * n - j {
            let xi = (i / 2 + 1) as f64;
            let xij = ((i + j) / 2 + 1) as f64;
            let c1 = xij - x;
            let c2 = x - xi;
            let denominator = xij - xi;
            rates[i] = (c1 * rates[i] + c2 * rates[i + 1] + (work[i + 1] - work[i])) / denominator;
            work[i] = (c1 * work[i] + c2 * work[i + 1]) / denominator;
        }
    }
    (work[0], rates[0] / step)
}
/// Source HRMINT: doubled abscissae, derivative column evaluated first.
pub(crate) fn hermite_irregular(
    epochs: &[f64],
    values: &[f64],
    x: f64,
) -> Result<(f64, f64), GeoError> {
    let n = epochs.len();
    let mut work = values.to_vec();
    let mut rates = vec![0.0; 2 * n];
    for i in 1..n {
        let c1 = epochs[i] - x;
        let c2 = x - epochs[i - 1];
        let denominator = epochs[i] - epochs[i - 1];
        if denominator == 0.0 {
            return Err(GeoError::BadDaf("duplicate SPK interpolation epochs"));
        }
        let prev = 2 * i - 2;
        let this = prev + 1;
        let next = this + 1;
        rates[prev] = work[this];
        rates[this] = (work[next] - work[prev]) / denominator;
        let temp = work[this] * (x - epochs[i - 1]) + work[prev];
        work[this] = (c1 * work[prev] + c2 * work[next]) / denominator;
        work[prev] = temp;
    }
    rates[2 * n - 2] = work[2 * n - 1];
    work[2 * n - 2] = work[2 * n - 1] * (x - epochs[n - 1]) + work[2 * n - 2];
    for j in 2..2 * n {
        for i in 0..2 * n - j {
            let xi = i / 2;
            let xij = (i + j) / 2;
            let c1 = epochs[xij] - x;
            let c2 = x - epochs[xi];
            let denominator = epochs[xij] - epochs[xi];
            if denominator == 0.0 {
                return Err(GeoError::BadDaf("duplicate SPK interpolation epochs"));
            }
            rates[i] = (c1 * rates[i] + c2 * rates[i + 1] + (work[i + 1] - work[i])) / denominator;
            work[i] = (c1 * work[i] + c2 * work[i + 1]) / denominator;
        }
    }
    Ok((work[0], rates[0]))
}
