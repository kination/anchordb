use crate::error::AnchorError;
use crc32fast::Hasher;

pub const RECORD_HEADER_SIZE: usize = 16;
pub const FILE_HEADER_MAGIC: [u8; 4] = *b"ANCH";
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
            magic: FILE_HEADER_MAGIC,
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

        buf[0..4].copy_from_slice(&self.magic);
        buf[4..6].copy_from_slice(&self.version.to_le_bytes());
        buf[6..8].copy_from_slice(&self.flags.to_le_bytes());
        buf[8..16].copy_from_slice(&self.record_count.to_le_bytes());
        buf[16..24].copy_from_slice(&self.total_records.to_le_bytes());
        buf[24..32].copy_from_slice(&self.data_size.to_le_bytes());
        buf[32..40].copy_from_slice(&self.next_id.to_le_bytes());
        buf[40..44].copy_from_slice(&self.checksum.to_le_bytes());
        // Bytes 44..64 are reserved and left as 0

        buf
    }

    pub fn from_bytes(buf: &[u8; FILE_HEADER_SIZE]) -> Result<Self, AnchorError> {
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&buf[0..4]);

        if magic != FILE_HEADER_MAGIC {
            return Err(AnchorError::InvalidMagic);
        }

        let version = u16::from_le_bytes(buf[4..6].try_into().unwrap());
        let flags = u16::from_le_bytes(buf[6..8].try_into().unwrap());
        let record_count = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        let total_records = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        let data_size = u64::from_le_bytes(buf[24..32].try_into().unwrap());
        let next_id = u64::from_le_bytes(buf[32..40].try_into().unwrap());
        let checksum = u32::from_le_bytes(buf[40..44].try_into().unwrap());

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


pub struct RecordHeader {
    pub id: u64,
    pub data_length: u64,
}

/// Record binary layout (16 bytes, Little-Endian):
///
/// ```text
/// Offset  Size  Field
/// ------  ----  -----------
/// 0       8     id           (u64 LE) - Record ID
/// 8       8     data_length  (u64 LE) - Payload size in bytes
/// 16      N     payload      (N = data_length, not part of header)
/// ```
impl RecordHeader {
    pub fn to_bytes(&self) -> [u8; RECORD_HEADER_SIZE] {
        let mut buf = [0u8; RECORD_HEADER_SIZE];
        buf[0..8].copy_from_slice(&self.id.to_le_bytes());
        buf[8..16].copy_from_slice(&self.data_length.to_le_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8; RECORD_HEADER_SIZE]) -> Self {
        Self {
            id: u64::from_le_bytes(buf[0..8].try_into().unwrap()),
            data_length: u64::from_le_bytes(buf[8..16].try_into().unwrap()),
        }
    }
}

