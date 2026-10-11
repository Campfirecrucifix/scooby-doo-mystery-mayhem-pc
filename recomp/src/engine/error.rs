//! Shared error type for engine load functions (rule 4 of tools/engineport/common.md).
//! Runtime functions keep the original 0/NULL/bool returns; only explicit load-time entry points
//! return `Result<_, EngineError>`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// A file the original would have opened with DTFopen does not exist (DTFopen < 0).
    FileNotFound(String),
    /// A DT stream error code (0xfffffffb = -5 short read/EOF, -2 read error, -3 open/bad name,
    /// -4 bad string quoting, -7 PrePersist skip).
    Stream(i32),
    /// Data does not have the layout the original code assumes (the original would crash or misread).
    Format(String),
    /// Host I/O failure (ISO / directory read).
    Io(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::FileNotFound(p) => write!(f, "file not found: {p}"),
            EngineError::Stream(c) => write!(f, "DT stream error {c}"),
            EngineError::Format(m) => write!(f, "bad data: {m}"),
            EngineError::Io(m) => write!(f, "I/O: {m}"),
        }
    }
}

impl std::error::Error for EngineError {}
