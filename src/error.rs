//! Errors returned by `stkcraft-geocalc`.

use core::fmt;

/// Errors returned by this crate.
#[derive(Debug)]
pub enum GeoError {
    /// Reading a kernel file failed.
    Io(std::io::Error),
    /// Spacecraft clock or time conversion failed in the time foundation.
    Time(stkcraft_time::TimeError),
    /// A conversion lies outside the explicit NAIF time contract.
    TimeContract(stkcraft_time::contract::ContractError),
    /// The file is not a SPICE DAF.
    NotDaf,
    /// The file is not a SPICE DAS container.
    NotDas,
    /// The DAS/DLA structure is invalid or corrupted.
    BadDas(&'static str),
    /// The DSK segment type has no supported evaluator.
    UnsupportedDskType(i32),
    /// The DAF structure is invalid or corrupted.
    BadDaf(&'static str),
    /// The segment frame requires an unavailable transformation context.
    UnsupportedFrame(i32),
    /// The SPK segment data type is not supported yet.
    UnsupportedType(i32),
    /// No chain of segments connects the requested bodies.
    NoPath { target: i32, observer: i32 },
    /// No segment covers the requested epoch for this pair.
    TimeOutOfRange { target: i32, center: i32, et: f64 },
    /// The kernel contains no segments.
    EmptyKernel,
    /// A frame name or id could not be resolved.
    FrameNotFound(String),
    /// The FK text could not be parsed.
    FrameParse(&'static str),
    /// The FK uses a frame specification this version does not support.
    UnsupportedFrameSpec(String),
    /// The two frames do not share an ancestor, so no rotation connects them.
    NoFramePath { from: String, to: String },
    /// The aberration correction name is not recognised.
    BadAberration(String),
    /// A feature is not implemented rather than approximated.
    Unsupported(String),
    /// A Geometry Finder search was given invalid options.
    BadSearch(&'static str),
    /// A required PCK value (radii or GM) is not loaded.
    MissingPckData(i32),
}

impl fmt::Display for GeoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeoError::Io(error) => write!(f, "I/O error: {error}"),
            GeoError::Time(error) => write!(f, "time conversion: {error}"),
            GeoError::TimeContract(error) => write!(f, "NAIF time conversion: {error}"),
            GeoError::NotDaf => write!(f, "not a SPICE DAF file"),
            GeoError::NotDas => write!(f, "not a SPICE DAS file"),
            GeoError::BadDas(message)
                if matches!(
                    *message,
                    "SPICE(POINTOFFSURFACE)" | "SPICE(NOLOADEDDSKFILES)" | "SPICE(DSKDATANOTFOUND)"
                ) =>
            {
                f.write_str(message)
            }
            GeoError::BadDas(message) => write!(f, "invalid DAS: {message}"),
            GeoError::UnsupportedDskType(kind) => {
                write!(f, "DSK data type {kind} is not supported")
            }
            GeoError::BadDaf(message) => write!(f, "invalid DAF: {message}"),
            GeoError::UnsupportedFrame(frame) => {
                write!(
                    f,
                    "segment frame {frame} requires a supported transformation context"
                )
            }
            GeoError::UnsupportedType(data_type) => {
                write!(f, "SPK data type {data_type} is not supported")
            }
            GeoError::NoPath { target, observer } => {
                write!(f, "no segment chain from {observer} to {target}")
            }
            GeoError::TimeOutOfRange { target, center, et } => write!(
                f,
                "no coverage for {target} relative to {center} at ET {et}"
            ),
            GeoError::EmptyKernel => write!(f, "the kernel contains no segments"),
            GeoError::FrameNotFound(name) => write!(f, "frame not found: {name}"),
            GeoError::FrameParse(message) => write!(f, "invalid frame kernel: {message}"),
            GeoError::UnsupportedFrameSpec(spec) => {
                write!(f, "unsupported frame specification: {spec}")
            }
            GeoError::NoFramePath { from, to } => {
                write!(f, "no frame path from {from} to {to}")
            }
            GeoError::BadAberration(name) if name == "SPICE(NOTSUPPORTED)" => f.write_str(name),
            GeoError::BadAberration(name) => {
                write!(f, "unrecognised aberration correction: {name}")
            }
            GeoError::Unsupported(message) => write!(f, "unsupported: {message}"),
            GeoError::BadSearch(message)
                if matches!(*message, "SPICE(BODIESNOTDISTINCT)" | "SPICE(INVALIDFRAME)") =>
            {
                f.write_str(message)
            }
            GeoError::BadSearch(message) => write!(f, "invalid search options: {message}"),
            GeoError::MissingPckData(body) => {
                write!(f, "no PCK data for body {body}")
            }
        }
    }
}

impl std::error::Error for GeoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GeoError::Io(error) => Some(error),
            GeoError::Time(error) => Some(error),
            GeoError::TimeContract(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for GeoError {
    fn from(error: std::io::Error) -> Self {
        GeoError::Io(error)
    }
}

impl From<stkcraft_time::TimeError> for GeoError {
    fn from(error: stkcraft_time::TimeError) -> Self {
        Self::Time(error)
    }
}

impl From<stkcraft_time::contract::ContractError> for GeoError {
    fn from(error: stkcraft_time::contract::ContractError) -> Self {
        Self::TimeContract(error)
    }
}
