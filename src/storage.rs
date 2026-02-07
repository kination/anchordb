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

    pub fn append_record(&mut self, id: u64, data: &[u8]) -> io::Result<u64> {
        let offset = self.file.seek(SeekFrom::End(0))?;

        let header = RecordHeader {
            id,
            data_length: data.len() as u64,
        };
        self.file.write_all(&header.to_bytes())?;
        self.file.write_all(data)?;

        Ok(offset)
    }

    pub fn read_record(&mut self, offset: u64, data_length: u64) -> io::Result<Vec<u8>> {
        self.file.seek(SeekFrom::Start(offset + RECORD_HEADER_SIZE as u64))?;

        let mut buf = vec![0u8; data_length as usize];
        self.file.read_exact(&mut buf)?;

        Ok(buf)
    }
}
