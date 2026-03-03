pub mod error;
pub mod types;
pub mod record;
pub mod index;
pub mod storage;

use std::io;
use std::path::Path;

use index::{Index, IndexEntry};
use storage::Storage;

use types::RECORD_TYPE_DATA;
use types::DataType::Str;


pub struct AnchorDB {
    storage: Storage,
    index: Index,
    next_id: u64,
}

impl AnchorDB {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let storage = Storage::open(path.as_ref())?;

        Ok(Self {
            storage,
            index: Index::new(),
            next_id: 1,
        })
    }

    pub fn save(&mut self, data: &str) -> io::Result<u64> {
        let id = self.next_id;
        let bytes = data.as_bytes();

        let (offset, timestamp) = self.storage.append_record(
            RECORD_TYPE_DATA,
            Str as u8,
            id,
            bytes
        )?;

        self.index.insert(id, IndexEntry {
            offset,
            data_length: bytes.len() as u32,
            data_type: Str,
            record_type: RECORD_TYPE_DATA,
            session_id: 0,
            timestamp,
        });
        self.next_id += 1;

        Ok(id)
    }

    pub fn load(&mut self, id: u64) -> io::Result<Option<String>> {
        let entry = match self.index.get(id) {
            Some(e) => e,
            None => return Ok(None),
        };

        let offset = entry.offset;
        let data_length = entry.data_length;
        let bytes = self.storage.read_record(offset, data_length)?;

        Ok(Some(String::from_utf8(bytes).expect("invalid utf-8")))
    }
}
