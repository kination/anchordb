use std::collections::HashMap;

pub struct IndexEntry {
    pub offset: u64,
    pub data_length: u64,
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
}
