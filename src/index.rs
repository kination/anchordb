use crate::types::DataType;
use std::io::{BufReader, BufWriter, Read, Write};
use std::{collections::HashMap, fs::File, io, path::Path};

pub struct IndexEntry {
    pub offset: u64,
    pub data_length: u32,
    pub data_type: DataType,
    pub record_type: u8,
    pub session_id: u64,
    pub timestamp: u64,
}

pub struct Index {
    entries: HashMap<u64, IndexEntry>,
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

    // TODO: [3.2 시작 최적화] 인덱스 스냅샷 직렬화/역직렬화 구현 가이드
    // 1. `pub fn serialize_to_file(&self, path: &Path, latest_id: u64, max_offset: u64) -> io::Result<()>`
    //    - 메모리의 `entries`를 순회하며 각각의 `IndexEntry` 프로퍼티들을 바이너리로 직렬화하여 `.idx` 파일에 저장합니다.
    //    - 파일 맨 앞(또는 맨 뒤)에는 복원 시 필요한 전역 정보인 `latest_id`와 데이터 파일의 `max_offset` 정보도 함께 적어 넣습니다.
    // 2. `pub fn deserialize_from_file(path: &Path) -> io::Result<(Index, u64, u64)>`
    //    - `.idx` 파일을 읽어서 역직렬화한 뒤, 복원된 `Index` 해시맵 구조체와 `latest_id`, 그리고 재개를 위한 `max_offset` 세 가지를 반환합니다.
    //    - 구조체 필드가 고정 크기이므로 직접 버퍼를 파싱하거나 `bincode` 같은 경량 직렬화 라이브러리를 고민해 볼 수 있습니다.

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

        let mut entry_buf = [0u8; 38];

        for _ in 0..count {
            reader.read_exact(&mut entry_buf)?;

            let id = u64::from_le_bytes(entry_buf[0..8].try_into().unwrap());
            let offset = u64::from_le_bytes(entry_buf[8..16].try_into().unwrap());
            let data_length = u32::from_le_bytes(entry_buf[16..20].try_into().unwrap());
            let data_type = DataType::try_from(entry_buf[20]).unwrap_or(DataType::Bytes);
            let record_type = entry_buf[21];
            let session_id = u64::from_le_bytes(entry_buf[22..30].try_into().unwrap());
            let timestamp = u64::from_le_bytes(entry_buf[30..38].try_into().unwrap());

            index.insert(
                id,
                IndexEntry {
                    offset,
                    data_length,
                    data_type,
                    record_type,
                    session_id,
                    timestamp,
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
            writer.write_all(&entry.record_type.to_le_bytes())?;
            writer.write_all(&entry.session_id.to_le_bytes())?;
            writer.write_all(&entry.timestamp.to_le_bytes())?;
        }

        writer.flush()?;
        Ok(())
    }
}
