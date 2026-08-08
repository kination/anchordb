use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::record::{FILE_HEADER_SIZE, FileHeader, RECORD_HEADER_SIZE, RecordHeader};
use crate::types::{Priority, VERSION};

pub struct Storage {
    storage_obj: File,
    pub path: PathBuf,
}

/// Storage to handle low-level file operations.
impl Storage {
    pub fn open(path: &Path) -> io::Result<Self> {
        let storage_obj = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        if let Err(e) = storage_obj.try_lock_exclusive() {
            return Err(io::Error::other(format!(
                "Failed to lock file {:?}: {}",
                path, e
            )));
        }

        let mut storage = Self {
            storage_obj,
            path: path.to_path_buf(),
        };

        if storage.storage_obj.metadata()?.len() == 0 {
            storage.write_header()?;
        } else {
            let mut buf = [0u8; FILE_HEADER_SIZE];
            storage.storage_obj.seek(SeekFrom::Start(0))?;
            storage.storage_obj.read_exact(&mut buf)?;
            let header = FileHeader::from_bytes(&buf).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid header: {:?}", e),
                )
            })?;

            if header.version != VERSION {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Invalid header version: expected {}, got {}",
                        VERSION, header.version
                    ),
                ));
            }
        }

        Ok(storage)
    }

    pub fn write_header(&mut self) -> io::Result<()> {
        let mut header = FileHeader::new(VERSION);
        self.storage_obj.seek(SeekFrom::Start(0))?;
        self.storage_obj.write_all(&header.to_bytes())?;
        Ok(())
    }

    pub fn append_record(
        &mut self,
        record_type: u8,
        data_type: u8,
        priority: Priority,
        id: u64,
        tags: &[u8],
        data: &[u8],
    ) -> io::Result<(u64, u64)> {
        let offset = self.storage_obj.seek(SeekFrom::End(0))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if tags.len() > u16::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "tags length exceeds u16::MAX",
            ));
        }

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(tags);
        hasher.update(data);
        let crc32 = hasher.finalize();

        let header = RecordHeader {
            record_type,
            id,
            timestamp,
            priority,
            session_id: 0, // default Session ID
            data_type,
            tags_len: tags.len() as u16,
            data_length: data.len() as u32,
            crc32,
        };
        self.storage_obj.write_all(&header.to_bytes())?;
        if !tags.is_empty() {
            self.storage_obj.write_all(tags)?;
        }
        self.storage_obj.write_all(data)?;

        Ok((offset, timestamp))
    }

    pub fn read_record(&mut self, offset: u64, data_length: u32) -> io::Result<Vec<u8>> {
        let (_tags, data) = self.read_record_full(offset, data_length)?;
        Ok(data)
    }

    pub fn read_record_full(
        &mut self,
        offset: u64,
        data_length: u32,
    ) -> io::Result<(Vec<u8>, Vec<u8>)> {
        self.storage_obj.seek(SeekFrom::Start(offset))?;

        let mut header_buf = [0u8; RECORD_HEADER_SIZE];
        self.storage_obj.read_exact(&mut header_buf)?;
        let header = RecordHeader::from_bytes(&header_buf);

        if header.data_length != data_length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "'Data length' mismatch",
            ));
        }

        let mut tags_buf = vec![0u8; header.tags_len as usize];
        if header.tags_len > 0 {
            self.storage_obj.read_exact(&mut tags_buf)?;
        }

        let mut buf = vec![0u8; data_length as usize];
        self.storage_obj.read_exact(&mut buf)?;

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&tags_buf);
        hasher.update(&buf);
        if hasher.finalize() != header.crc32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CRC32 checksum mismatch",
            ));
        }

        Ok((tags_buf, buf))
    }

    pub fn append_tombstone(&mut self, id: u64) -> io::Result<u64> {
        let offset = self.storage_obj.seek(SeekFrom::End(0))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let hasher = crc32fast::Hasher::new();
        let crc32 = hasher.finalize();

        let header = RecordHeader {
            record_type: crate::types::RECORD_TYPE_TOMBSTONE,
            id,
            timestamp,
            priority: crate::types::Priority::default(),
            session_id: 0,
            data_type: crate::types::DataType::Bytes as u8,
            tags_len: 0,
            data_length: 0,
            crc32,
        };
        self.storage_obj.write_all(&header.to_bytes())?;

        Ok(offset)
    }

    pub fn sync(&self) -> io::Result<()> {
        self.storage_obj.sync_all()
    }

    /// Return `.idx` file path derived from DB file path.
    pub fn idx_path(&self) -> PathBuf {
        let mut idx_path = self.path.clone();
        let mut ext = idx_path.extension().unwrap_or_default().to_os_string();
        ext.push(".idx");
        idx_path.set_extension(ext);
        idx_path
    }

    /// Rebuild index by scanning all records
    ///
    /// - If `.idx` file exists, load it and tail-read only new records
    /// - Include Truncate recovery
    /// - If there's unexpected EOF or CRC32 mismatch, truncate the file
    /// - If data length is greater than file size, truncate the file
    /// - Return latest_id
    pub fn rebuild_index(&mut self, index: &mut crate::index::Index) -> io::Result<u64> {
        let file_len = self.storage_obj.metadata()?.len();
        let mut offset = FILE_HEADER_SIZE as u64;
        let mut latest_id: u64 = 0;

        if file_len < offset {
            return Ok(0);
        }

        // Try loading `.idx` snapshot for quick boot (tail-read optimization)
        let idx_path = self.idx_path();
        if idx_path.exists() {
            match crate::index::Index::deserialize_from_file(&idx_path) {
                Ok((loaded_index, loaded_latest_id, loaded_max_offset)) => {
                    *index = loaded_index;
                    latest_id = loaded_latest_id;
                    offset = loaded_max_offset;
                }
                Err(_) => {
                    // Corrupted .idx file, and fallback to 'full scan'
                }
            }
        }

        self.storage_obj.seek(SeekFrom::Start(offset))?;

        // loop until offset goes over file_len
        loop {
            if offset >= file_len {
                break;
            }

            let valid_offset = offset;
            let mut header_buf = [0u8; RECORD_HEADER_SIZE];

            // Stop recover if file is corrupted with EOF
            if let Err(e) = self.storage_obj.read_exact(&mut header_buf) {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    self.storage_obj.set_len(valid_offset)?;
                    break;
                } else {
                    return Err(e);
                }
            }

            let header = RecordHeader::from_bytes(&header_buf);

            let record_total = RECORD_HEADER_SIZE as u64
                + header.tags_len as u64
                + header.data_length as u64;
            if valid_offset + record_total > file_len {
                self.storage_obj.set_len(valid_offset)?;
                break;
            }

            let mut tags_buf = vec![0u8; header.tags_len as usize];
            if header.tags_len > 0
                && let Err(e) = self.storage_obj.read_exact(&mut tags_buf)
            {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    self.storage_obj.set_len(valid_offset)?;
                    break;
                } else {
                    return Err(e);
                }
            }

            let mut data_buf = vec![0u8; header.data_length as usize];
            if header.data_length > 0
                && let Err(e) = self.storage_obj.read_exact(&mut data_buf)
            {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    self.storage_obj.set_len(valid_offset)?;
                    break;
                } else {
                    return Err(e);
                }
            }

            let mut hasher = crc32fast::Hasher::new();
            hasher.update(&tags_buf);
            hasher.update(&data_buf);
            if hasher.finalize() != header.crc32 {
                self.storage_obj.set_len(valid_offset)?;
                break;
            }

            if header.id > latest_id {
                latest_id = header.id;
            }

            if header.record_type == crate::types::RECORD_TYPE_TOMBSTONE {
                index.remove(header.id);
            } else if header.record_type == crate::types::RECORD_TYPE_DATA {
                let data_type = crate::types::DataType::try_from(header.data_type)
                    .unwrap_or(crate::types::DataType::Bytes);

                index.insert(
                    header.id,
                    crate::index::IndexEntry {
                        offset: valid_offset,
                        data_length: header.data_length,
                        data_type,
                        record_type: header.record_type,
                        session_id: header.session_id,
                        timestamp: header.timestamp,
                        priority: header.priority,
                    },
                );
            }

            offset += record_total;
        }

        Ok(latest_id)
    }
}
