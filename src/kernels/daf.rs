//! Reader for the SPICE Double Precision Array File (DAF) container.
//!
//! SPK and binary PCK containers share the file record,
//! the summary-record chain, and the name records. Element records are
//! read on demand by word address.

use std::fs;
use std::path::Path;

use crate::error::GeoError;

const RECORD_BYTES: usize = 1024;
const DOUBLE_BYTES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Endian {
    Little,
    Big,
}

/// Kernel family identified by the DAF file record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DafKind {
    Spk,
    Pck,
    Ck,
}

/// One DAF array summary and its name.
#[derive(Clone, Debug)]
pub struct DafSegment {
    /// Segment name from the name record.
    pub name: String,
    /// Double-precision components of the summary (`ND` values).
    pub dc: Vec<f64>,
    /// Integer components of the summary (`NI` values).
    pub ic: Vec<i32>,
}

/// An opened DAF file.
pub struct DafFile {
    kind: DafKind,
    bytes: Vec<u8>,
    endian: Endian,
    nd: usize,
    ni: usize,
    segments: Vec<DafSegment>,
}

impl DafFile {
    /// Opens and parses a DAF file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GeoError> {
        let bytes = fs::read(path)?;
        Self::from_bytes(bytes)
    }

    /// Parses a DAF from an in-memory buffer.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, GeoError> {
        if bytes.len() < RECORD_BYTES {
            return Err(GeoError::NotDaf);
        }
        let kind = match &bytes[..7] {
            b"DAF/SPK" => DafKind::Spk,
            b"DAF/PCK" => DafKind::Pck,
            b"DAF/CK " => DafKind::Ck,
            _ => return Err(GeoError::NotDaf),
        };

        let nd_le = read_u32(&bytes, 8, Endian::Little) as usize;
        let ni_le = read_u32(&bytes, 12, Endian::Little) as usize;
        let nd_be = read_u32(&bytes, 8, Endian::Big) as usize;
        let ni_be = read_u32(&bytes, 12, Endian::Big) as usize;

        let endian = if (1..=4).contains(&nd_le) && (1..=16).contains(&ni_le) {
            Endian::Little
        } else if (1..=4).contains(&nd_be) && (1..=16).contains(&ni_be) {
            Endian::Big
        } else {
            return Err(GeoError::BadDaf("implausible ND/NI in the file record"));
        };

        let nd = if endian == Endian::Little {
            nd_le
        } else {
            nd_be
        };
        let ni = if endian == Endian::Little {
            ni_le
        } else {
            ni_be
        };
        let fward = read_u32(&bytes, 76, endian) as usize;
        let bward = read_u32(&bytes, 80, endian) as usize;
        if fward == 0 || bward == 0 {
            return Err(GeoError::BadDaf("missing summary record pointer"));
        }

        let summary_size = nd + ni.div_ceil(2);

        let mut segments = Vec::new();
        let mut record = fward;
        let mut guard = 0usize;

        while record != 0 {
            guard += 1;
            if guard > 100_000 {
                return Err(GeoError::BadDaf("summary record chain is cyclic"));
            }

            let base = (record - 1) * RECORD_BYTES;
            if base + RECORD_BYTES > bytes.len() {
                return Err(GeoError::BadDaf("summary record is out of range"));
            }

            // The three control words are integer-valued doubles.
            let next = read_f64(&bytes, base, endian);
            let nsum = read_f64(&bytes, base + 16, endian);
            if !(next.is_finite() && nsum.is_finite()) {
                return Err(GeoError::BadDaf("non-finite summary control word"));
            }
            if next < 0.0 || next.fract() != 0.0 || nsum < 0.0 || nsum.fract() != 0.0 {
                return Err(GeoError::BadDaf(
                    "summary control words must be nonnegative integers",
                ));
            }
            let nsum = nsum as usize;
            if nsum > (RECORD_BYTES - 24) / (summary_size * DOUBLE_BYTES) {
                return Err(GeoError::BadDaf("summary count is out of range"));
            }

            // The name record immediately follows the summary record.
            let name_base = record * RECORD_BYTES;
            let name_size = summary_size * DOUBLE_BYTES;
            if name_base + nsum * name_size > bytes.len() {
                return Err(GeoError::BadDaf("name record is out of range"));
            }

            for index in 0..nsum {
                let start = base + 24 + index * summary_size * DOUBLE_BYTES;

                let mut dc = Vec::with_capacity(nd);
                for component in 0..nd {
                    dc.push(read_f64(&bytes, start + component * DOUBLE_BYTES, endian));
                }

                let mut ic = Vec::with_capacity(ni);
                for word in 0..ni.div_ceil(2) {
                    let offset = start + (nd + word) * DOUBLE_BYTES;
                    ic.push(read_i32(&bytes, offset, endian));
                    if ic.len() < ni {
                        ic.push(read_i32(&bytes, offset + 4, endian));
                    }
                }

                let name = read_name(&bytes, name_base + index * name_size, name_size);
                segments.push(DafSegment { name, dc, ic });
            }

            record = next as usize;
        }

        Ok(Self {
            kind,
            bytes,
            endian,
            nd,
            ni,
            segments,
        })
    }

    /// File family. SPK and CK both use ND=2/NI=6; their data differ.
    pub fn kind(&self) -> DafKind {
        self.kind
    }

    /// Number of double-precision components in each summary.
    pub fn nd(&self) -> usize {
        self.nd
    }

    /// Number of integer components in each summary.
    pub fn ni(&self) -> usize {
        self.ni
    }

    /// Parsed array summaries.
    pub fn segments(&self) -> &[DafSegment] {
        &self.segments
    }

    /// Reads `count` doubles starting at the given 1-based word address.
    pub fn read_doubles(&self, start_word: usize, count: usize) -> Result<Vec<f64>, GeoError> {
        if start_word == 0 {
            return Err(GeoError::BadDaf("word addresses are 1-based"));
        }
        let offset = (start_word - 1)
            .checked_mul(DOUBLE_BYTES)
            .ok_or(GeoError::BadDaf("word address overflow"))?;
        let end = offset
            .checked_add(
                count
                    .checked_mul(DOUBLE_BYTES)
                    .ok_or(GeoError::BadDaf("word count overflow"))?,
            )
            .ok_or(GeoError::BadDaf("word range overflow"))?;
        if end > self.bytes.len() {
            return Err(GeoError::BadDaf("element record is out of range"));
        }

        let mut values = Vec::with_capacity(count);
        for index in 0..count {
            values.push(read_f64(
                &self.bytes,
                offset + index * DOUBLE_BYTES,
                self.endian,
            ));
        }
        Ok(values)
    }
}

fn read_u32(bytes: &[u8], offset: usize, endian: Endian) -> u32 {
    let slice = &bytes[offset..offset + 4];
    match endian {
        Endian::Little => u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]),
        Endian::Big => u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]),
    }
}

fn read_i32(bytes: &[u8], offset: usize, endian: Endian) -> i32 {
    read_u32(bytes, offset, endian) as i32
}

fn read_f64(bytes: &[u8], offset: usize, endian: Endian) -> f64 {
    let slice = &bytes[offset..offset + 8];
    let mut raw = [0u8; 8];
    raw.copy_from_slice(slice);
    match endian {
        Endian::Little => f64::from_le_bytes(raw),
        Endian::Big => f64::from_be_bytes(raw),
    }
}

fn read_name(bytes: &[u8], offset: usize, size: usize) -> String {
    let raw = &bytes[offset..offset + size];
    let end = raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).trim_end().to_string()
}
