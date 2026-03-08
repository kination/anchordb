/// Magic Bytes ("ACDB")
pub const MAGIC_BYTES: [u8; 4] = *b"ACDB";

/// File Format Version
pub const VERSION: u16 = 1;

/// Common Field Sizes for Binary Layout
pub const FIELD_ID_SIZE: usize = 8;
pub const FIELD_OFFSET_SIZE: usize = 8;
pub const FIELD_TIMESTAMP_SIZE: usize = 8;
pub const FIELD_SESSION_ID_SIZE: usize = 8;
pub const FIELD_DATA_LENGTH_SIZE: usize = 4;
pub const FIELD_TAGS_LEN_SIZE: usize = 2;
pub const FIELD_CRC32_SIZE: usize = 4;
pub const FIELD_DATA_TYPE_SIZE: usize = 1;
pub const FIELD_RECORD_TYPE_SIZE: usize = 1;

/// Record Types
pub const RECORD_TYPE_DATA: u8 = 0x01;
pub const RECORD_TYPE_TOMBSTONE: u8 = 0x02;
pub const RECORD_TYPE_SUMMARY: u8 = 0x03;
pub const RECORD_TYPE_SNAPSHOT: u8 = 0x04;
pub const RECORD_TYPE_SCRATCHPAD: u8 = 0x05;

/// 'Priority level' of data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Priority {
    Ephemeral = 0x00,
    Normal = 0x01,
    Important = 0x02,
    Critical = 0x03,
}

impl TryFrom<u8> for Priority {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Priority::Ephemeral),
            0x01 => Ok(Priority::Normal),
            0x02 => Ok(Priority::Important),
            0x03 => Ok(Priority::Critical),
            _ => Err(()),
        }
    }
}

/// Data Types supported by DB payload
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DataType {
    Bytes = 0x00,
    Str = 0x01,
    Json = 0x02,
}

impl TryFrom<u8> for DataType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(DataType::Bytes),
            0x01 => Ok(DataType::Str),
            0x02 => Ok(DataType::Json),
            _ => Err(()),
        }
    }
}

/// Wrapper for data ensuring type safety
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorData {
    Bytes(Vec<u8>),
    Str(String),
    Json(String),
}

impl AnchorData {
    pub fn data_type(&self) -> DataType {
        match self {
            AnchorData::Bytes(_) => DataType::Bytes,
            AnchorData::Str(_) => DataType::Str,
            AnchorData::Json(_) => DataType::Json,
        }
    }

    /// Return bytes for storage
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            AnchorData::Bytes(b) => b,
            AnchorData::Str(s) => s.as_bytes(),
            AnchorData::Json(j) => j.as_bytes(),
        }
    }
}
