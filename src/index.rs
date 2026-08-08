use crate::types::{
    DataType, FIELD_DATA_LENGTH_SIZE, FIELD_DATA_TYPE_SIZE, FIELD_ID_SIZE, FIELD_OFFSET_SIZE,
    FIELD_PRIORITY_SIZE, FIELD_RECORD_TYPE_SIZE, FIELD_SESSION_ID_SIZE, FIELD_TIMESTAMP_SIZE,
    Priority,
};
use std::io::{BufReader, BufWriter, Read, Write};
use std::{collections::HashMap, fs::File, io, path::Path};

pub const INDEX_ENTRY_SIZE: usize = FIELD_ID_SIZE
    + FIELD_OFFSET_SIZE
    + FIELD_DATA_LENGTH_SIZE
    + FIELD_DATA_TYPE_SIZE
    + FIELD_PRIORITY_SIZE
    + FIELD_RECORD_TYPE_SIZE
    + FIELD_SESSION_ID_SIZE
    + FIELD_TIMESTAMP_SIZE;

pub struct IndexEntry {
    pub offset: u64,
    pub data_length: u32,
    pub data_type: DataType,
    pub record_type: u8,
    pub session_id: u64,
    pub timestamp: u64,
    pub priority: Priority,
}

pub struct Index {
    entries: HashMap<u64, IndexEntry>,
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Index {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: u64, entry: IndexEntry) {
        self.entries.insert(id, entry);
    }

    pub fn get(&self, id: u64) -> Option<&IndexEntry> {
        self.entries.get(&id)
    }

    pub fn remove(&mut self, id: u64) -> Option<IndexEntry> {
        self.entries.remove(&id)
    }

    pub fn contains(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn keys(&self) -> Vec<u64> {
        self.entries.keys().copied().collect()
    }

    pub fn deserialize_from_file(path: &Path) -> io::Result<(Index, u64, u64)> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut u64_buf = [0u8; 8];

        // Read headers
        reader.read_exact(&mut u64_buf)?;
        let latest_id = u64::from_le_bytes(u64_buf);

        reader.read_exact(&mut u64_buf)?;
        let max_offset = u64::from_le_bytes(u64_buf);

        reader.read_exact(&mut u64_buf)?;
        let count = u64::from_le_bytes(u64_buf);

        let mut index = Index::new();

        let mut entry_buf = [0u8; INDEX_ENTRY_SIZE];

        for _ in 0..count {
            reader.read_exact(&mut entry_buf)?;

            let mut offset = 0;

            let id = u64::from_le_bytes(
                entry_buf[offset..offset + FIELD_ID_SIZE]
                    .try_into()
                    .unwrap(),
            );
            offset += FIELD_ID_SIZE;

            let entry_offset = u64::from_le_bytes(
                entry_buf[offset..offset + FIELD_OFFSET_SIZE]
                    .try_into()
                    .unwrap(),
            );
            offset += FIELD_OFFSET_SIZE;

            let data_length = u32::from_le_bytes(
                entry_buf[offset..offset + FIELD_DATA_LENGTH_SIZE]
                    .try_into()
                    .unwrap(),
            );
            offset += FIELD_DATA_LENGTH_SIZE;

            let data_type = DataType::try_from(entry_buf[offset]).unwrap_or(DataType::Bytes);
            offset += FIELD_DATA_TYPE_SIZE;

            let priority = Priority::try_from(entry_buf[offset]).unwrap_or(Priority::Normal);
            offset += FIELD_PRIORITY_SIZE;

            let record_type = entry_buf[offset];
            offset += FIELD_RECORD_TYPE_SIZE;

            let session_id = u64::from_le_bytes(
                entry_buf[offset..offset + FIELD_SESSION_ID_SIZE]
                    .try_into()
                    .unwrap(),
            );
            offset += FIELD_SESSION_ID_SIZE;

            let timestamp = u64::from_le_bytes(
                entry_buf[offset..offset + FIELD_TIMESTAMP_SIZE]
                    .try_into()
                    .unwrap(),
            );

            index.insert(
                id,
                IndexEntry {
                    offset: entry_offset,
                    data_length,
                    data_type,
                    record_type,
                    session_id,
                    timestamp,
                    priority,
                },
            );
        }

        Ok((index, latest_id, max_offset))
    }
    pub fn serialize_to_file(
        &self,
        path: &Path,
        latest_id: u64,
        max_offset: u64,
    ) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write header: latest_id, max_offset, and number of entries
        writer.write_all(&latest_id.to_le_bytes())?;
        writer.write_all(&max_offset.to_le_bytes())?;

        let count = self.entries.len() as u64;
        writer.write_all(&count.to_le_bytes())?;

        // Write entries
        for (id, entry) in &self.entries {
            writer.write_all(&id.to_le_bytes())?;
            writer.write_all(&entry.offset.to_le_bytes())?;
            writer.write_all(&entry.data_length.to_le_bytes())?;
            writer.write_all(&(entry.data_type as u8).to_le_bytes())?;
            writer.write_all(&(entry.priority as u8).to_le_bytes())?;
            writer.write_all(&entry.record_type.to_le_bytes())?;
            writer.write_all(&entry.session_id.to_le_bytes())?;
            writer.write_all(&entry.timestamp.to_le_bytes())?;
        }

        writer.flush()?;
        Ok(())
    }
}
