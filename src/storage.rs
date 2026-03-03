use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::record::{RecordHeader, RECORD_HEADER_SIZE};

pub struct Storage {
    file: File,
    pub path: PathBuf,
}

/// Storage to handle low-level file operations.
impl Storage {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        Ok(Self {
            file,
            path: path.to_path_buf(),
        })
    }

    pub fn append_record(&mut self, record_type: u8, data_type: u8, id: u64, data: &[u8]) -> io::Result<(u64, u64)> {
        let offset = self.file.seek(SeekFrom::End(0))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(data);
        let crc32 = hasher.finalize();

        let header = RecordHeader {
            record_type,
            id,
            timestamp,
            session_id: 0, // default Session ID
            data_type,
            tags_len: 0,
            data_length: data.len() as u32,
            crc32,
        };
        self.file.write_all(&header.to_bytes())?;
        self.file.write_all(data)?;

        Ok((offset, timestamp))
    }

    pub fn read_record(&mut self, offset: u64, data_length: u32) -> io::Result<Vec<u8>> {
        self.file.seek(SeekFrom::Start(offset + RECORD_HEADER_SIZE as u64))?;

        let mut buf = vec![0u8; data_length as usize];
        self.file.read_exact(&mut buf)?;

        Ok(buf)
    }
}
