use crate::error::AnchorError;
use crate::types::{
    FIELD_CRC32_SIZE, FIELD_DATA_LENGTH_SIZE, FIELD_DATA_TYPE_SIZE, FIELD_PRIORITY_SIZE, FIELD_ID_SIZE,
    FIELD_RECORD_TYPE_SIZE, FIELD_SESSION_ID_SIZE, FIELD_TAGS_LEN_SIZE, FIELD_TIMESTAMP_SIZE,
    MAGIC_BYTES, Priority
};
use crc32fast::Hasher;

pub const RECORD_HEADER_SIZE: usize = FIELD_RECORD_TYPE_SIZE
    + FIELD_ID_SIZE
    + FIELD_TIMESTAMP_SIZE
    + FIELD_SESSION_ID_SIZE
    + FIELD_DATA_TYPE_SIZE
    + FIELD_PRIORITY_SIZE
    + FIELD_TAGS_LEN_SIZE
    + FIELD_DATA_LENGTH_SIZE
    + FIELD_CRC32_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordHeader {
    pub record_type: u8,
    pub id: u64,
    pub timestamp: u64,
    pub session_id: u64,
    pub data_type: u8,
    pub priority: Priority,
    pub tags_len: u16,
    pub data_length: u32,
    pub crc32: u32,
}

/// Record binary layout (36 bytes fixed parts, Little-Endian):
///
/// ```text
/// Offset  Size  Field          Type      Description
/// ------  ----  -----------    -------   -----------
/// 0       1     record_type    u8        0x01=Data, 0x02=Tombstone, etc.
/// 1       8     id             u64 LE    Record ID
/// 9       8     timestamp      u64 LE    Epoch millis
/// 17      8     session_id     u64 LE    Session ID
/// 25      1     data_type      u8        0x00=Bytes, 0x01=Str, 0x02=JSON
/// 26      1     priority       u8        
/// 27      2     tags_len       u16 LE    Tags length
/// 29      4     data_length    u32 LE    Payload length
/// 33      4     crc32          u32 LE    Payload + Tags CRC32
/// ```
impl RecordHeader {
    pub fn to_bytes(&self) -> [u8; RECORD_HEADER_SIZE] {
        let mut buf = [0u8; RECORD_HEADER_SIZE];
        let mut offset = 0;

        buf[offset] = self.record_type;
        offset += FIELD_RECORD_TYPE_SIZE;

        buf[offset..offset + FIELD_ID_SIZE].copy_from_slice(&self.id.to_le_bytes());
        offset += FIELD_ID_SIZE;

        buf[offset..offset + FIELD_TIMESTAMP_SIZE].copy_from_slice(&self.timestamp.to_le_bytes());
        offset += FIELD_TIMESTAMP_SIZE;

        buf[offset..offset + FIELD_SESSION_ID_SIZE].copy_from_slice(&self.session_id.to_le_bytes());
        offset += FIELD_SESSION_ID_SIZE;

        buf[offset] = self.data_type;
        offset += FIELD_DATA_TYPE_SIZE;
        
        buf[offset] = self.priority as u8;
        offset += FIELD_PRIORITY_SIZE;

        buf[offset..offset + FIELD_TAGS_LEN_SIZE].copy_from_slice(&self.tags_len.to_le_bytes());
        offset += FIELD_TAGS_LEN_SIZE;

        buf[offset..offset + FIELD_DATA_LENGTH_SIZE]
            .copy_from_slice(&self.data_length.to_le_bytes());
        offset += FIELD_DATA_LENGTH_SIZE;

        buf[offset..offset + FIELD_CRC32_SIZE].copy_from_slice(&self.crc32.to_le_bytes());

        buf
    }

    pub fn from_bytes(buf: &[u8; RECORD_HEADER_SIZE]) -> Self {
        let mut offset = 0;

        let record_type = buf[offset];
        offset += FIELD_RECORD_TYPE_SIZE;

        let id = u64::from_le_bytes(buf[offset..offset + FIELD_ID_SIZE].try_into().unwrap());
        offset += FIELD_ID_SIZE;

        let timestamp = u64::from_le_bytes(
            buf[offset..offset + FIELD_TIMESTAMP_SIZE]
                .try_into()
                .unwrap(),
        );
        offset += FIELD_TIMESTAMP_SIZE;

        let session_id = u64::from_le_bytes(
            buf[offset..offset + FIELD_SESSION_ID_SIZE]
                .try_into()
                .unwrap(),
        );
        offset += FIELD_SESSION_ID_SIZE;

        let data_type = buf[offset];
        offset += FIELD_DATA_TYPE_SIZE;
        
        let priority = Priority::try_from(buf[offset]).unwrap();
        
        offset += FIELD_PRIORITY_SIZE;

        let tags_len = u16::from_le_bytes(
            buf[offset..offset + FIELD_TAGS_LEN_SIZE]
                .try_into()
                .unwrap(),
        );
        offset += FIELD_TAGS_LEN_SIZE;

        let data_length = u32::from_le_bytes(
            buf[offset..offset + FIELD_DATA_LENGTH_SIZE]
                .try_into()
                .unwrap(),
        );
        offset += FIELD_DATA_LENGTH_SIZE;

        let crc32 = u32::from_le_bytes(buf[offset..offset + FIELD_CRC32_SIZE].try_into().unwrap());

        Self {
            record_type,
            id,
            timestamp,
            session_id,
            data_type,
            priority,
            tags_len,
            data_length,
            crc32,
        }
    }
}
pub const FILE_MAGIC_SIZE: usize = 4;
pub const FILE_VERSION_SIZE: usize = 2;
pub const FILE_FLAGS_SIZE: usize = 2;
pub const FILE_RECORD_COUNT_SIZE: usize = 8;
pub const FILE_TOTAL_RECORDS_SIZE: usize = 8;
pub const FILE_DATA_SIZE_SIZE: usize = 8;
pub const FILE_NEXT_ID_SIZE: usize = 8;
pub const FILE_CHECKSUM_SIZE: usize = 4;
pub const FILE_HEADER_SIZE: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub record_count: u64,
    pub total_records: u64,
    pub data_size: u64,
    pub next_id: u64,
    pub checksum: u32,
    // _reserved: [u8; 20] is handled internally
}

impl FileHeader {
    pub fn new(version: u16) -> Self {
        Self {
            magic: MAGIC_BYTES,
            version,
            flags: 0,
            record_count: 0,
            total_records: 0,
            data_size: 0,
            next_id: 1,
            checksum: 0,
        }
    }

    /// Calculates CRC32 over the first 40 bytes of the header.
    fn calculate_checksum(&self) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(&self.magic);
        hasher.update(&self.version.to_le_bytes());
        hasher.update(&self.flags.to_le_bytes());
        hasher.update(&self.record_count.to_le_bytes());
        hasher.update(&self.total_records.to_le_bytes());
        hasher.update(&self.data_size.to_le_bytes());
        hasher.update(&self.next_id.to_le_bytes());
        hasher.finalize()
    }

    pub fn to_bytes(&mut self) -> [u8; FILE_HEADER_SIZE] {
        self.checksum = self.calculate_checksum();
        let mut buf = [0u8; FILE_HEADER_SIZE];
        let mut offset = 0;

        buf[offset..offset + FILE_MAGIC_SIZE].copy_from_slice(&self.magic);
        offset += FILE_MAGIC_SIZE;

        buf[offset..offset + FILE_VERSION_SIZE].copy_from_slice(&self.version.to_le_bytes());
        offset += FILE_VERSION_SIZE;

        buf[offset..offset + FILE_FLAGS_SIZE].copy_from_slice(&self.flags.to_le_bytes());
        offset += FILE_FLAGS_SIZE;

        buf[offset..offset + FILE_RECORD_COUNT_SIZE]
            .copy_from_slice(&self.record_count.to_le_bytes());
        offset += FILE_RECORD_COUNT_SIZE;

        buf[offset..offset + FILE_TOTAL_RECORDS_SIZE]
            .copy_from_slice(&self.total_records.to_le_bytes());
        offset += FILE_TOTAL_RECORDS_SIZE;

        buf[offset..offset + FILE_DATA_SIZE_SIZE].copy_from_slice(&self.data_size.to_le_bytes());
        offset += FILE_DATA_SIZE_SIZE;

        buf[offset..offset + FILE_NEXT_ID_SIZE].copy_from_slice(&self.next_id.to_le_bytes());
        offset += FILE_NEXT_ID_SIZE;

        buf[offset..offset + FILE_CHECKSUM_SIZE].copy_from_slice(&self.checksum.to_le_bytes());
        // Bytes 44..64 are reserved and left as 0

        buf
    }

    pub fn from_bytes(buf: &[u8; FILE_HEADER_SIZE]) -> Result<Self, AnchorError> {
        let mut offset = 0;

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&buf[offset..offset + FILE_MAGIC_SIZE]);
        offset += FILE_MAGIC_SIZE;

        if magic != MAGIC_BYTES {
            return Err(AnchorError::InvalidMagic);
        }

        let version =
            u16::from_le_bytes(buf[offset..offset + FILE_VERSION_SIZE].try_into().unwrap());
        offset += FILE_VERSION_SIZE;

        let flags = u16::from_le_bytes(buf[offset..offset + FILE_FLAGS_SIZE].try_into().unwrap());
        offset += FILE_FLAGS_SIZE;

        let record_count = u64::from_le_bytes(
            buf[offset..offset + FILE_RECORD_COUNT_SIZE]
                .try_into()
                .unwrap(),
        );
        offset += FILE_RECORD_COUNT_SIZE;

        let total_records = u64::from_le_bytes(
            buf[offset..offset + FILE_TOTAL_RECORDS_SIZE]
                .try_into()
                .unwrap(),
        );
        offset += FILE_TOTAL_RECORDS_SIZE;

        let data_size = u64::from_le_bytes(
            buf[offset..offset + FILE_DATA_SIZE_SIZE]
                .try_into()
                .unwrap(),
        );
        offset += FILE_DATA_SIZE_SIZE;

        let next_id =
            u64::from_le_bytes(buf[offset..offset + FILE_NEXT_ID_SIZE].try_into().unwrap());
        offset += FILE_NEXT_ID_SIZE;

        let checksum =
            u32::from_le_bytes(buf[offset..offset + FILE_CHECKSUM_SIZE].try_into().unwrap());

        let header = Self {
            magic,
            version,
            flags,
            record_count,
            total_records,
            data_size,
            next_id,
            checksum,
        };

        if header.calculate_checksum() != checksum {
            return Err(AnchorError::ChecksumMismatch);
        }

        Ok(header)
    }
}
