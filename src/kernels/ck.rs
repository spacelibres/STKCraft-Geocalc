//! All6 native N0067 CK segment types at encoded spacecraft-clock ticks.
//! Tick lookup is independent of ET: no clock rate or time-scale approximation
//! is applied. Frame conversion here accepts the built-in inertial frames.
use crate::{
    daf::{DafFile, DafKind},
    error::GeoError,
    frame::{mat_mul, transpose, Frames, Mat3, IDENTITY},
};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};
#[derive(Clone, Debug)]
pub struct CkPointing {
    /// Rotation from the reference frame into the instrument frame.
    pub rotation: Mat3,
    /// Instrument angular velocity relative to, and expressed in, reference.
    pub angular_velocity: Option<[f64; 3]>,
    /// Encoded clock tick at which pointing was returned.
    pub clock: f64,
    /// NAIF reference-frame ID of the returned data.
    pub reference_frame: i32,
}
#[derive(Clone, Debug)]
pub struct CkSegment {
    pub instrument: i32,
    pub reference_frame: i32,
    pub data_type: i32,
    pub has_angular_velocity: bool,
    pub start_ticks: f64,
    pub end_ticks: f64,
    pub name: String,
    begin: usize,
    end: usize,
    source: usize,
}
#[derive(Clone)]
pub struct CkKernel {
    files: Vec<Arc<DafFile>>,
    segments: Vec<CkSegment>,
    frames: Frames,
    reuse: Arc<Mutex<HashMap<i32, Reuse>>>,
    reuse6: Arc<Mutex<Option<Reuse6>>>,
}
#[derive(Clone, Copy)]
struct Reuse {
    segment: usize,
    lower: f64,
    upper: f64,
}
#[derive(Clone, Copy)]
struct Reuse6 {
    source: usize,
    segment_begin: usize,
    mini: usize,
    count: usize,
    select_last: bool,
    lower: f64,
    upper: f64,
    begin: usize,
    rate: f64,
    subtype: usize,
    window: usize,
    packets: usize,
}
impl std::fmt::Debug for CkKernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CkKernel")
            .field("segments", &self.segments)
            .finish()
    }
}
impl CkKernel {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GeoError> {
        Self::open_all(&[path])
    }
    /// Later files and later segments have higher priority, as in CKBSR.
    pub fn open_all(paths: &[impl AsRef<Path>]) -> Result<Self, GeoError> {
        let mut kernel = Self {
            files: Vec::new(),
            segments: Vec::new(),
            frames: Frames::new(),
            reuse: Arc::new(Mutex::new(HashMap::new())),
            reuse6: Arc::new(Mutex::new(None)),
        };
        for path in paths {
            let file = DafFile::open(path)?;
            if file.kind() != DafKind::Ck || file.nd() != 2 || file.ni() != 6 {
                return Err(GeoError::BadDaf(
                    "CK requires DAF/CK summaries with ND=2 NI=6",
                ));
            }
            for segment in file.segments() {
                let ic = &segment.ic;
                if ic[4] <= 0 || ic[5] < ic[4] {
                    return Err(GeoError::BadDaf("invalid CK segment addresses"));
                }
                kernel.segments.push(CkSegment {
                    instrument: ic[0],
                    reference_frame: ic[1],
                    data_type: ic[2],
                    has_angular_velocity: ic[3] == 1,
                    start_ticks: segment.dc[0],
                    end_ticks: segment.dc[1],
                    name: segment.name.clone(),
                    begin: ic[4] as usize,
                    end: ic[5] as usize,
                    source: kernel.files.len(),
                });
            }
            kernel.files.push(Arc::new(file));
        }
        Ok(kernel)
    }
    pub fn segments(&self) -> &[CkSegment] {
        &self.segments
    }
    pub fn ckgp(
        &self,
        instrument: i32,
        ticks: f64,
        tolerance: f64,
        reference: i32,
    ) -> Result<Option<CkPointing>, GeoError> {
        self.pointing_in(instrument, ticks, tolerance, false, reference)
    }
    pub fn ckgpav(
        &self,
        instrument: i32,
        ticks: f64,
        tolerance: f64,
        reference: i32,
    ) -> Result<Option<CkPointing>, GeoError> {
        self.pointing_in(instrument, ticks, tolerance, true, reference)
    }
    fn pointing_in(
        &self,
        instrument: i32,
        ticks: f64,
        tolerance: f64,
        need_av: bool,
        reference: i32,
    ) -> Result<Option<CkPointing>, GeoError> {
        let Some(mut pointing) = self.pointing(instrument, ticks, tolerance, need_av)? else {
            return Ok(None);
        };
        if pointing.reference_frame != reference {
            if !(1..=21).contains(&reference) || !(1..=21).contains(&pointing.reference_frame) {
                return Err(GeoError::Unsupported(
                    "CK non-inertial reference conversion requires an SCLK/frame context".into(),
                ));
            }
            let rotation = self.frames.pxform(
                &reference.to_string(),
                &pointing.reference_frame.to_string(),
                0.0,
            )?;
            pointing.rotation = mat_mul(&pointing.rotation, &rotation);
            pointing.angular_velocity = pointing
                .angular_velocity
                .map(|av| rotate(&transpose(&rotation), av).map(|x| 0.0 + x));
            pointing.reference_frame = reference;
        }
        Ok(Some(pointing))
    }
    /// Native segment-frame pointing. NEED_AV excludes segments without AV.
    pub fn pointing(
        &self,
        instrument: i32,
        ticks: f64,
        tolerance: f64,
        need_av: bool,
    ) -> Result<Option<CkPointing>, GeoError> {
        if !ticks.is_finite() || !tolerance.is_finite() {
            return Err(GeoError::BadSearch("CK ticks and tolerance must be finite"));
        }
        if tolerance < 0.0 {
            return Ok(None);
        }
        // CKBSR saves the first candidate's topologically open re-use
        // interval. Its AV filter affects candidate selection, not interval
        // trimming; CKGP following CKGPAV must retain this native behavior.
        let mut reuse = self
            .reuse
            .lock()
            .map_err(|_| GeoError::BadSearch("CK search lock is poisoned"))?;
        if let Some(previous) = reuse.get(&instrument).copied() {
            let segment = &self.segments[previous.segment];
            if ticks > previous.lower + tolerance
                && ticks < previous.upper - tolerance
                && (!need_av || segment.has_angular_velocity)
            {
                if let Some(result) = self.evaluate(segment, ticks, tolerance, need_av)? {
                    return Ok(Some(result));
                }
            }
        }
        reuse.remove(&instrument);
        let (mut lower, mut upper) = (-f64::MAX, f64::MAX);
        let mut new_search = true;
        for (index, segment) in self.segments.iter().enumerate().rev() {
            if segment.instrument != instrument {
                continue;
            }
            if new_search {
                if ticks > segment.end_ticks {
                    lower = lower.max(segment.end_ticks);
                } else if ticks < segment.start_ticks {
                    upper = upper.min(segment.start_ticks);
                }
            }
            if (need_av && !segment.has_angular_velocity)
                || ticks + tolerance < segment.start_ticks
                || ticks - tolerance > segment.end_ticks
            {
                continue;
            }
            if new_search && ticks > segment.start_ticks && ticks < segment.end_ticks {
                reuse.insert(
                    instrument,
                    Reuse {
                        segment: index,
                        lower: lower.max(segment.start_ticks),
                        upper: upper.min(segment.end_ticks),
                    },
                );
            }
            new_search = false;
            if let Some(result) = self.evaluate(segment, ticks, tolerance, need_av)? {
                return Ok(Some(result));
            }
        }
        reuse.remove(&instrument);
        Ok(None)
    }
    fn evaluate(
        &self,
        segment: &CkSegment,
        ticks: f64,
        tolerance: f64,
        need_av: bool,
    ) -> Result<Option<CkPointing>, GeoError> {
        match segment.data_type {
            1 => self.type1(segment, ticks, tolerance, need_av),
            2 => self.type2(segment, ticks, tolerance, need_av),
            3 => self.type3(segment, ticks, tolerance, need_av),
            4 => self.type4(segment, ticks, tolerance, need_av),
            5 => self.type5(segment, ticks, tolerance, need_av),
            6 => self.type6(segment, ticks, tolerance, need_av),
            other => Err(GeoError::Unsupported(format!("CK data type {other}"))),
        }
    }
    fn type4(
        &self,
        s: &CkSegment,
        t: f64,
        tol: f64,
        need_av: bool,
    ) -> Result<Option<CkPointing>, GeoError> {
        let generic =
            crate::generic_segment::GenericSegment::new(&self.files[s.source], s.begin, s.end)?;
        let n = generic.packet_count();
        let selected = generic.reference_at(t)?;
        let mut record;
        let clock;
        if let Some((index, _)) = selected {
            record = generic.packet(index.min(n))?;
            if record.len() < 3 {
                return Err(GeoError::BadDaf("short CK4 packet"));
            }
            let end = record[0] + record[1];
            if index >= n {
                if t > record[0] + record[1] + tol {
                    return Ok(None);
                }
                clock = if t >= end { end } else { t };
            } else if t <= end {
                clock = t;
            } else {
                let next = generic.packet(index + 1)?;
                if next.len() < 3 {
                    return Err(GeoError::BadDaf("short CK4 packet"));
                }
                let start = next[0] - next[1];
                if t - end <= start - t {
                    if t > record[0] + record[1] + tol {
                        return Ok(None);
                    }
                    clock = end;
                } else {
                    if t < next[0] - next[1] - tol {
                        return Ok(None);
                    }
                    clock = start;
                    record = next;
                }
            }
        } else {
            record = generic.packet(1)?;
            if record.len() < 3 {
                return Err(GeoError::BadDaf("short CK4 packet"));
            }
            if t < record[0] - record[1] - tol {
                return Ok(None);
            }
            clock = record[0] - record[1];
        }
        let packed = record[2] as u64;
        let counts: [usize; 7] = std::array::from_fn(|i| ((packed >> (7 * i)) & 127) as usize);
        if record[1] <= 0.0
            || record.len() != 3 + counts.iter().sum::<usize>()
            || counts[..4].contains(&0)
            || (need_av && counts[4..].contains(&0))
        {
            return Err(GeoError::BadDaf("invalid CK4 Chebyshev packet"));
        }
        let tau = (clock - record[0]) / record[1];
        let mut offset = 3;
        let mut values = [0.0; 7];
        for component in 0..if need_av { 7 } else { 4 } {
            let count = counts[component];
            values[component] = crate::spk::chebyshev(&record[offset..offset + count], tau);
            offset += count;
        }
        let quaternion = [values[0], values[1], values[2], values[3]];
        let length = norm4(quaternion);
        let q = if length > 0.0 {
            quaternion.map(|v| v / length)
        } else {
            [0.0; 4]
        };
        Ok(Some(CkPointing {
            rotation: q2m(q),
            angular_velocity: if need_av {
                Some([values[4], values[5], values[6]])
            } else {
                None
            },
            clock,
            reference_frame: s.reference_frame,
        }))
    }
    fn type5(
        &self,
        s: &CkSegment,
        t: f64,
        tol: f64,
        need_av: bool,
    ) -> Result<Option<CkPointing>, GeoError> {
        let file = &self.files[s.source];
        let control = file.read_doubles(s.end - 4, 5)?;
        let rate = control[0];
        let subtype = control[1].round() as usize;
        let window = control[2].round() as usize;
        let intervals = control[3].round() as usize;
        let n = control[4].round() as usize;
        let packet_size = match subtype {
            0 => 8,
            1 => 4,
            2 => 14,
            3 => 7,
            _ => return Err(GeoError::BadDaf("invalid CK5 subtype")),
        };
        if n == 0
            || intervals == 0
            || rate <= 0.0
            || window < 2
            || window % 2 != 0
            || window > if subtype % 2 == 0 { 12 } else { 24 }
        {
            return Err(GeoError::BadDaf("invalid CK5 control data"));
        }
        let epochs = file.read_doubles(s.begin + n * packet_size, n)?;
        let starts =
            file.read_doubles(s.begin + (packet_size + 1) * n + (n - 1) / 100, intervals)?;
        let mut clock = t;
        if clock < epochs[0] {
            if t + tol < epochs[0] {
                return Ok(None);
            }
            clock = epochs[0];
        } else if clock > epochs[n - 1] {
            if t - tol > epochs[n - 1] {
                return Ok(None);
            }
            clock = epochs[n - 1];
        }
        let i = epochs.partition_point(|v| *v < clock);
        let mut low = i.max(1);
        let mut high = if i == 0 { n.min(2) } else { low + 1 };
        let mut left_epoch = epochs[low - 1];
        let right_epoch = epochs[high - 1];
        let mut interval = starts.partition_point(|v| *v <= clock).saturating_sub(1);
        let mut start = starts[interval];
        let mut next = starts.get(interval + 1).copied().unwrap_or(f64::MAX);
        if right_epoch == next {
            if (clock - left_epoch).abs() <= (right_epoch - clock).abs() {
                if (clock - left_epoch).abs() > tol {
                    return Ok(None);
                }
                clock = left_epoch;
                high = low;
            } else {
                if (right_epoch - clock).abs() > tol {
                    return Ok(None);
                }
                clock = right_epoch;
                low = high;
                left_epoch = right_epoch;
                interval += 1;
                start = starts[interval];
                next = starts.get(interval + 1).copied().unwrap_or(f64::MAX);
            }
        }
        let _ = left_epoch;
        let left = (window / 2).min(low);
        let right = (window / 2).min(n - high + 1);
        let mut first = low - left;
        let mut last = first + left + right;
        // CKR05 deliberately forms the nominal window before trimming it
        // to the selected interval; retain its complete DAF read/order.
        let candidate = file.read_doubles(s.begin + n * packet_size + first, last - first)?;
        let skip = last_less_than(&candidate, start);
        let limit = last_less_than(&candidate, next);
        last = first + limit;
        first += skip;
        if first >= last {
            return Ok(None);
        }
        let times = &candidate[skip..limit];
        let packets =
            file.read_doubles(s.begin + first * packet_size, (last - first) * packet_size)?;
        let (rotation, av) =
            evaluate_quaternion_packets(subtype, rate, clock, times, packets, need_av)?;
        Ok(Some(point(s, clock, rotation, av)))
    }
    fn type6(
        &self,
        s: &CkSegment,
        t: f64,
        tol: f64,
        need_av: bool,
    ) -> Result<Option<CkPointing>, GeoError> {
        let file = &self.files[s.source];
        let mut clock = t.max(s.start_ticks).min(s.end_ticks);
        let mut saved = self
            .reuse6
            .lock()
            .map_err(|_| GeoError::BadSearch("CK6 search lock is poisoned"))?;
        let previous = saved.take();
        let reusable = previous.filter(|p| {
            if p.source != s.source || p.segment_begin != s.begin {
                return false;
            }
            if p.select_last {
                clock >= p.lower
                    && if p.mini + 1 < p.count {
                        clock < p.upper
                    } else {
                        clock <= p.upper
                    }
            } else {
                clock <= p.upper
                    && if p.mini > 0 {
                        clock > p.lower
                    } else {
                        clock >= p.lower
                    }
            }
        });
        let selected = if let Some(previous) = reusable {
            previous
        } else {
            let control = file.read_doubles(s.end - 1, 2)?;
            let last = control[0].round() == 1.0;
            let n = control[1].round() as usize;
            if n == 0 {
                return Err(GeoError::BadDaf("empty CK6 interval directory"));
            }
            let pointer_base = s.end - 2 - (n + 1);
            let directory_base = pointer_base - n / 100;
            let bounds = file.read_doubles(directory_base - n, n + 1)?;
            let pointers = file.read_doubles(pointer_base + 1, n + 1)?;
            let mini = if last {
                bounds.partition_point(|v| *v <= clock)
            } else {
                bounds.partition_point(|v| *v < clock)
            }
            .max(1)
            .min(n)
                - 1;
            let read_mini = |i: usize, round_rate: bool| -> Result<(Reuse6, f64), GeoError> {
                let begin = s.begin + pointers[i].round() as usize - 1;
                let end = s.begin + pointers[i + 1].round() as usize - 2;
                let controls = file.read_doubles(end - 3, 4)?;
                let rate = if round_rate {
                    controls[0].round()
                } else {
                    controls[0]
                };
                let subtype = controls[1].round() as usize;
                let window = controls[2].round() as usize;
                let packets = controls[3].round() as usize;
                if subtype > 3
                    || packets < 2
                    || window < 2
                    || window % 2 != 0
                    || window > if subtype % 2 == 0 { 12 } else { 24 }
                {
                    return Err(GeoError::BadDaf("invalid CK6 packet control data"));
                }
                let last_epoch = file.read_doubles(end - 4 - (packets - 1) / 100, 1)?[0];
                Ok((
                    Reuse6 {
                        source: s.source,
                        segment_begin: s.begin,
                        mini: i,
                        count: n,
                        select_last: last,
                        lower: bounds[i],
                        upper: bounds[i + 1].min(last_epoch),
                        begin,
                        rate,
                        subtype,
                        window,
                        packets,
                    },
                    last_epoch,
                ))
            };
            let (mut selected, last_epoch) = read_mini(mini, false)?;
            if clock > last_epoch {
                if tol == 0.0 {
                    return Ok(None);
                }
                if clock - last_epoch > tol && bounds[mini + 1] - clock > tol {
                    return Ok(None);
                }
                if mini + 1 == n {
                    if clock - last_epoch > tol {
                        return Ok(None);
                    }
                    clock = last_epoch;
                } else if clock - last_epoch <= (bounds[mini + 1] - last_epoch) / 2.0 {
                    clock = last_epoch;
                } else {
                    // CKR06's gap→next branch applies NINT to the clock
                    // rate and caches that value. Retain its call history.
                    selected = read_mini(mini + 1, true)?.0;
                    clock = bounds[mini + 1];
                }
            }
            selected
        };
        let packet_size = [8, 4, 14, 7][selected.subtype];
        let n = selected.packets;
        let epochs = file.read_doubles(selected.begin + n * packet_size, n)?;
        let low = last_less_than(&epochs, clock).max(1);
        let high = low + 1;
        let left = (selected.window / 2).min(low);
        let right = (selected.window / 2).min(n - high + 1);
        let first = low - left;
        let size = left + right;
        let times = &epochs[first..first + size];
        let packets =
            file.read_doubles(selected.begin + first * packet_size, size * packet_size)?;
        *saved = Some(selected);
        drop(saved);
        let (rotation, av) = evaluate_quaternion_packets(
            selected.subtype,
            selected.rate,
            clock,
            times,
            packets,
            need_av,
        )?;
        Ok(Some(point(s, clock, rotation, av)))
    }
    fn type1(
        &self,
        s: &CkSegment,
        t: f64,
        tol: f64,
        need_av: bool,
    ) -> Result<Option<CkPointing>, GeoError> {
        let file = &self.files[s.source];
        let n = file.read_doubles(s.end, 1)?[0] as usize;
        if n == 0 {
            return Err(GeoError::BadDaf("empty CK1 segment"));
        }
        let size = if s.has_angular_velocity { 7 } else { 4 };
        let ndir = (n - 1) / 100;
        let directories = file.read_doubles(s.begin + (size + 1) * n, ndir)?;
        let skip = directories.partition_point(|x| *x <= t) * 100;
        let times = file.read_doubles(s.begin + size * n + skip, (n - skip).min(100))?;
        let i = closest(&times, t);
        if (t - times[i]).abs() > tol {
            return Ok(None);
        }
        let record = file.read_doubles(s.begin + (skip + i) * size, size)?;
        Ok(Some(point(
            s,
            times[i],
            q2m(record[..4].try_into().unwrap()),
            if need_av {
                Some(record[4..7].try_into().unwrap())
            } else {
                None
            },
        )))
    }
    fn type2(
        &self,
        s: &CkSegment,
        t: f64,
        tol: f64,
        need_av: bool,
    ) -> Result<Option<CkPointing>, GeoError> {
        let file = &self.files[s.source];
        let n = (((s.end - s.begin + 1) as f64 * 100.0 + 1.0) / 1001.0).round() as usize;
        if n == 0 {
            return Err(GeoError::BadDaf("empty CK2 segment"));
        }
        let ndir = (n - 1) / 100;
        let directories = file.read_doubles(s.begin + 10 * n, ndir)?;
        let skip = directories.partition_point(|x| *x <= t) * 100;
        let starts = file.read_doubles(s.begin + 8 * n + skip, (n - skip).min(100))?;
        let i = starts.partition_point(|x| *x <= t);
        let (index, clock) = if i == 0 {
            if t + tol < starts[0] {
                return Ok(None);
            }
            (0, starts[0])
        } else {
            let stop = file.read_doubles(s.begin + 9 * n + skip + i - 1, 1)?[0];
            if t <= stop {
                (i - 1, t)
            } else if i == starts.len() {
                if t - tol > stop {
                    return Ok(None);
                }
                (i - 1, stop)
            } else {
                let left = t - stop;
                let right = starts[i] - t;
                if left.min(right) > tol {
                    return Ok(None);
                }
                if right <= left {
                    (i, starts[i])
                } else {
                    (i - 1, stop)
                }
            }
        };
        let record = file.read_doubles(s.begin + (skip + index) * 8, 8)?;
        let av: [f64; 3] = record[4..7].try_into().unwrap();
        let time = (clock - starts[index]) * record[7];
        let angle = time * crate::surface::norm(av);
        let rotation = mat_mul(
            &q2m(record[..4].try_into().unwrap()),
            &transpose(&axisar(av, angle)),
        );
        Ok(Some(point(
            s,
            clock,
            rotation,
            if need_av { Some(av) } else { None },
        )))
    }
    fn type3(
        &self,
        s: &CkSegment,
        t: f64,
        tol: f64,
        need_av: bool,
    ) -> Result<Option<CkPointing>, GeoError> {
        let file = &self.files[s.source];
        let controls = file.read_doubles(s.end - 1, 2)?;
        let intervals = controls[0].round() as usize;
        let n = controls[1].round() as usize;
        if n == 0 || intervals == 0 {
            return Err(GeoError::BadDaf("empty CK3 segment"));
        }
        let size = if s.has_angular_velocity { 7 } else { 4 };
        let ndir = (n - 1) / 100;
        let directories = file.read_doubles(s.begin + (size + 1) * n, ndir)?;
        let skip = directories.partition_point(|x| *x < t) * 100;
        let times = file.read_doubles(s.begin + size * n + skip, (n - skip).min(100))?;
        let i = times.partition_point(|x| *x <= t);
        let (left, right) = if i == 0 && skip == 0 {
            if times[0] - t > tol {
                return Ok(None);
            }
            (0, 0)
        } else if i == times.len() {
            if t - times[i - 1] > tol {
                return Ok(None);
            }
            (skip + i - 1, skip + i - 1)
        } else if i == 0 {
            (skip - 1, skip)
        } else {
            (skip + i - 1, skip + i)
        };
        let mut li = left;
        let mut ri = right;
        let mut lt = file.read_doubles(s.begin + size * n + li, 1)?[0];
        let mut rt = file.read_doubles(s.begin + size * n + ri, 1)?[0];
        if li != ri {
            let starts = file.read_doubles(s.begin + (size + 1) * n + ndir, intervals)?;
            let interval = starts.partition_point(|x| *x <= t);
            let next = starts.get(interval).copied().unwrap_or(f64::MAX);
            if rt >= next {
                let ld = t - lt;
                let rd = rt - t;
                if ld > tol && rd > tol {
                    return Ok(None);
                }
                if ld < rd {
                    ri = li;
                    rt = lt;
                } else {
                    li = ri;
                    lt = rt;
                }
            }
        }
        let l = file.read_doubles(s.begin + li * size, size)?;
        let r = file.read_doubles(s.begin + ri * size, size)?;
        let av1 = if need_av {
            Some(<[f64; 3]>::try_from(&l[4..7]).unwrap())
        } else {
            None
        };
        if lt == rt {
            return Ok(Some(point(s, lt, q2m(l[..4].try_into().unwrap()), av1)));
        }
        let fraction = (t - lt) / (rt - lt);
        let m1 = q2m(l[..4].try_into().unwrap());
        let m2 = q2m(r[..4].try_into().unwrap());
        let relative = mat_mul(&transpose(&m2), &m1);
        let (angle, axis) = crate::orientation::matrix_to_axis_angle(&relative);
        let delta = axisar(axis, angle * fraction);
        let rotation = mat_mul(&m1, &transpose(&delta));
        let av =
            av1.map(|a| std::array::from_fn(|j| (1.0 - fraction) * a[j] + fraction * r[j + 4]));
        Ok(Some(point(s, t, rotation, av)))
    }
}
fn point(
    s: &CkSegment,
    clock: f64,
    rotation: Mat3,
    angular_velocity: Option<[f64; 3]>,
) -> CkPointing {
    CkPointing {
        rotation,
        angular_velocity,
        clock,
        reference_frame: s.reference_frame,
    }
}
// Keep the native end-of-table branch separate from the distance/tie branch.
#[allow(clippy::if_same_then_else)]
fn closest(times: &[f64], t: f64) -> usize {
    let right = times.partition_point(|x| *x <= t);
    if right == 0 {
        0
    } else if right == times.len() {
        right - 1
    } else if t - times[right - 1] < times[right] - t {
        right - 1
    } else {
        right
    }
}
fn rotate(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
}
fn evaluate_quaternion_packets(
    subtype: usize,
    rate: f64,
    t: f64,
    epochs: &[f64],
    mut packets: Vec<f64>,
    need_av: bool,
) -> Result<(Mat3, Option<[f64; 3]>), GeoError> {
    let size = [8, 4, 14, 7][subtype];
    let n = epochs.len();
    for i in 1..n {
        let previous: [f64; 4] = packets[(i - 1) * size..(i - 1) * size + 4]
            .try_into()
            .unwrap();
        let current: [f64; 4] = packets[i * size..i * size + 4].try_into().unwrap();
        let negative_distance = norm4(std::array::from_fn(|j| previous[j] - (-current[j])));
        let distance = norm4(std::array::from_fn(|j| previous[j] - current[j]));
        if negative_distance < distance {
            if subtype % 2 == 0 {
                return Err(GeoError::BadSearch("SPICE(BADQUATSIGN)"));
            }
            for j in 0..4 {
                packets[i * size + j] = -packets[i * size + j];
            }
        }
    }
    let mut values = [0.0; 8];
    for component in 0..4 {
        if subtype == 1 {
            let ys = (0..n).map(|i| packets[i * size + component]).collect();
            let (p, d) = lagrange_with_derivative(epochs, ys, t)?;
            values[component] = p;
            values[component + 4] = d;
        } else if subtype == 3 {
            values[component] = crate::spk::lagrange_irregular(
                epochs,
                (0..n).map(|i| packets[i * size + component]).collect(),
                t,
            )?;
        } else {
            let ys: Vec<_> = (0..n)
                .flat_map(|i| {
                    [
                        packets[i * size + component],
                        packets[i * size + component + 4] * rate,
                    ]
                })
                .collect();
            let (p, d) = crate::spk::hermite_irregular(epochs, &ys, t)?;
            values[component] = p;
            values[component + 4] = d;
        }
    }
    let raw: [f64; 4] = values[..4].try_into().unwrap();
    let magnitude = norm4(raw);
    let q = if subtype == 3 {
        if magnitude > 0.0 {
            raw.map(|v| v / magnitude)
        } else {
            [0.0; 4]
        }
    } else {
        if magnitude == 0.0 {
            return Err(GeoError::BadSearch("SPICE(DIVIDEBYZERO)"));
        }
        let inverse = 1.0 / magnitude;
        raw.map(|v| inverse * v)
    };
    let av = if !need_av {
        None
    } else if subtype == 3 {
        let mut av = [0.0; 3];
        for c in 0..3 {
            av[c] = crate::spk::lagrange_irregular(
                epochs,
                (0..n).map(|i| packets[i * size + c + 4]).collect(),
                t,
            )?;
        }
        Some(av)
    } else if subtype == 2 {
        let mut av = [0.0; 3];
        for c in 0..3 {
            let ys: Vec<_> = (0..n)
                .flat_map(|i| [packets[i * size + c + 8], packets[i * size + c + 11] * rate])
                .collect();
            av[c] = crate::spk::hermite_irregular(epochs, &ys, t)?.0;
        }
        Some(av)
    } else {
        let derivative: [f64; 4] = values[4..].try_into().unwrap();
        let mut product = 0.0;
        for i in 0..4 {
            product += q[i] * derivative[i];
        }
        let inverse = 1.0 / magnitude;
        let radial = product / magnitude;
        let dq: [f64; 4] = std::array::from_fn(|i| inverse * derivative[i] - radial * q[i]);
        let length = norm4(q);
        let qhat = q.map(|v| v / length);
        let star = [qhat[0], -qhat[1], -qhat[2], -qhat[3]];
        let cross = [
            star[2] * dq[3] - star[3] * dq[2],
            star[3] * dq[1] - star[1] * dq[3],
            star[1] * dq[2] - star[2] * dq[1],
        ];
        let av = std::array::from_fn(|i| {
            let v = star[0] * dq[i + 1] + dq[0] * star[i + 1] + 1.0 * cross[i];
            (1.0 / rate) * (-2.0 * v)
        });
        Some(av)
    };
    Ok((q2m(q), av))
}
fn lagrange_with_derivative(
    epochs: &[f64],
    mut work: Vec<f64>,
    x: f64,
) -> Result<(f64, f64), GeoError> {
    let n = epochs.len();
    let mut derivatives = vec![0.0; n];
    for j in 1..n {
        for i in 0..n - j {
            let denominator = epochs[i] - epochs[i + j];
            if denominator == 0.0 {
                return Err(GeoError::BadSearch("SPICE(DIVIDEBYZERO)"));
            }
            let c1 = x - epochs[i + j];
            let c2 = epochs[i] - x;
            derivatives[i] =
                (c1 * derivatives[i] + c2 * derivatives[i + 1] + (work[i] - work[i + 1]))
                    / denominator;
            work[i] = (c1 * work[i] + c2 * work[i + 1]) / denominator;
        }
    }
    Ok((work[0], derivatives[0]))
}
fn last_less_than(values: &[f64], x: f64) -> usize {
    let n = values.len();
    if n == 0 || x <= values[0] {
        return 0;
    }
    if values[n - 1] < x {
        return n;
    }
    let mut begin = 1;
    let mut end = n;
    let mut items = n;
    while items > 2 {
        let middle = begin + items / 2;
        if values[middle - 1] < x {
            begin = middle;
        } else {
            end = middle;
        }
        items = end - begin + 1;
    }
    begin
}
fn norm4(q: [f64; 4]) -> f64 {
    let maximum = q.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
    if maximum == 0.0 {
        return 0.0;
    }
    let mut sum = 0.0;
    for value in q {
        let scaled = value / maximum;
        sum += scaled * scaled;
    }
    maximum * sum.sqrt()
}
fn q2m(q: [f64; 4]) -> Mat3 {
    let (mut a, mut b, mut c) = (q[0] * q[1], q[0] * q[2], q[0] * q[3]);
    let (mut d, mut e, mut f) = (q[1] * q[2], q[1] * q[3], q[2] * q[3]);
    let (mut x, mut y, mut z) = (q[1] * q[1], q[2] * q[2], q[3] * q[3]);
    let sum = q[0] * q[0] + x + y + z;
    if sum != 1.0 && sum != 0.0 {
        let scale = 1.0 / sum;
        a *= scale;
        b *= scale;
        c *= scale;
        d *= scale;
        e *= scale;
        f *= scale;
        x *= scale;
        y *= scale;
        z *= scale;
    }
    [
        [1.0 - (y + z) * 2.0, (d - c) * 2.0, (e + b) * 2.0],
        [(d + c) * 2.0, 1.0 - (x + z) * 2.0, (f - a) * 2.0],
        [(e - b) * 2.0, (f + a) * 2.0, 1.0 - (x + y) * 2.0],
    ]
}
/// Source AXISAR rotates the identity columns using VROTV/VPROJ.
fn axisar(axis: [f64; 3], angle: f64) -> Mat3 {
    if crate::surface::norm(axis) == 0.0 {
        return IDENTITY;
    }
    let unit = crate::surface::unit(axis);
    let c = angle.cos();
    let s = angle.sin();
    let mut matrix = IDENTITY;
    for column in 0..3 {
        let v = std::array::from_fn(|i| IDENTITY[i][column]);
        let p = crate::surface::project(v, unit);
        let v1: [f64; 3] = std::array::from_fn(|i| v[i] - p[i]);
        let v2 = [
            unit[1] * v1[2] - unit[2] * v1[1],
            unit[2] * v1[0] - unit[0] * v1[2],
            unit[0] * v1[1] - unit[1] * v1[0],
        ];
        for row in 0..3 {
            matrix[row][column] = (c * v1[row] + s * v2[row]) + p[row];
        }
    }
    matrix
}
