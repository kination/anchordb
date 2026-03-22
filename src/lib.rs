pub mod error;
pub mod index;
pub mod record;
pub mod storage;
pub mod types;

use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

use index::{Index, IndexEntry};
use storage::Storage;

use types::DataType::Str;
use types::RECORD_TYPE_DATA;

/// Internal 'inner' class for safe multi-thread access
pub struct AnchorDBInner {
    storage: Storage,
    index: Index,
    next_id: u64,
}

#[derive(Clone)]
pub struct AnchorDB {
    inner: Arc<Mutex<AnchorDBInner>>,
}

impl AnchorDB {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut storage = Storage::open(path.as_ref())?;
        let mut index = Index::new();

        let max_id = storage.rebuild_index(&mut index)?;

        let inner = AnchorDBInner {
            storage,
            index,
            next_id: max_id + 1,
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    pub fn save(&self, data: &str) -> io::Result<u64> {
        let mut inner = self.inner.lock().unwrap();
        let id = inner.next_id;
        let bytes = data.as_bytes();

        let (offset, timestamp) =
            inner
                .storage
                .append_record(RECORD_TYPE_DATA, Str as u8, id, bytes)?;

        inner.index.insert(
            id,
            IndexEntry {
                offset,
                data_length: bytes.len() as u32,
                data_type: Str,
                record_type: RECORD_TYPE_DATA,
                session_id: 0,
                timestamp,
            },
        );
        inner.next_id += 1;

        Ok(id)
    }

    pub fn load(&self, id: u64) -> io::Result<Option<String>> {
        let mut inner = self.inner.lock().unwrap();

        let entry = match inner.index.get(id) {
            Some(e) => {
                // Copy values before releasing borrow
                let offset = e.offset;
                let data_length = e.data_length;
                (offset, data_length)
            }
            None => return Ok(None),
        };

        let bytes = inner.storage.read_record(entry.0, entry.1)?;

        Ok(Some(String::from_utf8(bytes).expect("invalid utf-8")))
    }

    /// `true` if active (non-tombstoned) record with given ID exists
    pub fn exists(&self, id: u64) -> bool {
        self.inner.lock().unwrap().index.contains(id)
    }

    /// Return count of active records.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().index.len()
    }

    /// `true` if database has no active records.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().index.is_empty()
    }

    /// Return all active record IDs(order is unspecified)
    pub fn keys(&self) -> Vec<u64> {
        self.inner.lock().unwrap().index.keys()
    }

    /// Saves the in-memory index to a `.idx` file for fast startup on next open.
    pub fn close(&self) -> io::Result<()> {
        //  Make 'local_inner' to avoid holding the lock while doing file I/O
        let local_inner = self.inner.lock().unwrap();
        let latest_id = local_inner.next_id.saturating_sub(1);
        let idx_path = local_inner.storage.idx_path();
        let max_offset = local_inner
            .storage
            .path
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);

        local_inner
            .index
            .serialize_to_file(&idx_path, latest_id, max_offset)
    }
}

// Force saving '.idx' file when AnchorDBInner is dropped, for exceptional case
impl Drop for AnchorDBInner {
    fn drop(&mut self) {
        let idx_path = self.storage.idx_path();
        let max_offset = self.storage.path.metadata().map(|m| m.len()).unwrap_or(0);
        let latest_id = self.next_id.saturating_sub(1);
        let _ = self
            .index
            .serialize_to_file(&idx_path, latest_id, max_offset);
    }
}
