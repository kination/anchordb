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
use types::{AnchorData, DataType, MemoryInput, MemoryRecord, Priority, RECORD_TYPE_DATA};

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
        self.save_with_opts(data.as_bytes(), Str, Priority::default(), &[])
    }

    /// Save raw bytes with explicit data_type, priority, and tags payload.
    pub fn save_with_opts(
        &self,
        data: &[u8],
        data_type: DataType,
        priority: Priority,
        tags: &[u8],
    ) -> io::Result<u64> {
        let mut inner = self.inner.lock().unwrap();
        let id = inner.next_id;

        let (offset, timestamp) = inner.storage.append_record(
            RECORD_TYPE_DATA,
            data_type as u8,
            priority,
            id,
            tags,
            data,
        )?;

        inner.index.insert(
            id,
            IndexEntry {
                offset,
                data_length: data.len() as u32,
                data_type,
                record_type: RECORD_TYPE_DATA,
                session_id: 0,
                timestamp,
                priority,
            },
        );
        inner.next_id += 1;

        Ok(id)
    }

    /// Append a `MemoryInput` and return assigned ID.
    pub fn append_memory(&self, input: &MemoryInput) -> io::Result<u64> {
        let tags_bytes = encode_tags(&input.tags);
        self.save_with_opts(
            input.data.as_bytes(),
            input.data.data_type(),
            input.priority,
            &tags_bytes,
        )
    }

    /// Fetch a record by ID as a `MemoryRecord`. Returns `None` if missing.
    pub fn fetch_memory(&self, id: u64) -> io::Result<Option<MemoryRecord>> {
        let mut inner = self.inner.lock().unwrap();
        let (offset, data_length, data_type, priority, timestamp) = match inner.index.get(id) {
            Some(e) => (e.offset, e.data_length, e.data_type, e.priority, e.timestamp),
            None => return Ok(None),
        };

        let (tags_bytes, data_bytes) = inner.storage.read_record_full(offset, data_length)?;

        let tags = decode_tags(&tags_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let data = match data_type {
            DataType::Bytes => AnchorData::Bytes(data_bytes),
            DataType::Str => AnchorData::Str(
                String::from_utf8(data_bytes)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            ),
            DataType::Json => AnchorData::Json(
                String::from_utf8(data_bytes)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            ),
        };

        Ok(Some(MemoryRecord::from_parts(
            id, tags, data, priority, timestamp,
        )))
    }

    /// Mark a record as deleted by appending a tombstone.
    /// Returns `true` if a record with given ID existed and was removed.
    pub fn delete(&self, id: u64) -> io::Result<bool> {
        let mut inner = self.inner.lock().unwrap();
        if inner.index.remove(id).is_none() {
            return Ok(false);
        }
        inner.storage.append_tombstone(id)?;
        Ok(true)
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

        let s = String::from_utf8(bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(s))
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

/// Tags encoding: sequence of `[len: u16 LE][utf8 bytes]` per tag.
fn encode_tags(tags: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for t in tags {
        let bytes = t.as_bytes();
        let len = bytes.len().min(u16::MAX as usize) as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&bytes[..len as usize]);
    }
    out
}

fn decode_tags(buf: &[u8]) -> Result<Vec<String>, &'static str> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < buf.len() {
        if i + 2 > buf.len() {
            return Err("truncated tag length");
        }
        let len = u16::from_le_bytes([buf[i], buf[i + 1]]) as usize;
        i += 2;
        if i + len > buf.len() {
            return Err("truncated tag bytes");
        }
        let s = std::str::from_utf8(&buf[i..i + len])
            .map_err(|_| "tag is not valid utf-8")?
            .to_string();
        out.push(s);
        i += len;
    }
    Ok(out)
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
