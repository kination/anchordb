use std::fmt;
use std::io;


#[derive(Debug)]
pub enum AnchorError {
    /// File not having expected magic bytes
    InvalidMagic,
    /// File format version is unsupported or mismatched
    InvalidVersion,
    /// Data integrity check (CRC32) failed
    ChecksumMismatch,
    /// Record is corrupted/malformed
    DataCorrupted,
    /// Record with the specified ID was not found
    RecordNotFound,
    /// Common errors
    Io(io::Error),
}

impl fmt::Display for AnchorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnchorError::InvalidMagic => write!(f, "Invalid magic bytes in file header"),
            AnchorError::InvalidVersion => write!(f, "Unsupported file format version"),
            AnchorError::ChecksumMismatch => write!(f, "Data checksum mismatch (potential corruption)"),
            AnchorError::DataCorrupted => write!(f, "Record data is corrupted or malformed"),
            AnchorError::RecordNotFound => write!(f, "Record not found"),
            AnchorError::Io(err) => write!(f, "Unexpected error: {}", err),
        }
    }
}

impl std::error::Error for AnchorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AnchorError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for AnchorError {
    fn from(err: io::Error) -> Self {
        AnchorError::Io(err)
    }
}

/// Convenience alias for operations returning AnchorError
pub type Result<T> = std::result::Result<T, AnchorError>;
