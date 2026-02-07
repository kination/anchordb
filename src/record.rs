pub const RECORD_HEADER_SIZE: usize = 16;

pub struct RecordHeader {
    pub id: u64,
    pub data_length: u64,
}

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
