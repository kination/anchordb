use anchordb::AnchorDB;
use anchordb::types::{AnchorData, DataType, MemoryInput, Priority};
use tempfile::NamedTempFile;

#[test]
fn save_and_load() {
    let tmp = NamedTempFile::new().unwrap();
    let db = AnchorDB::open(tmp.path()).unwrap();

    let id1 = db.save("hello").unwrap();
    let id2 = db.save("world").unwrap();

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(db.load(id1).unwrap(), Some("hello".to_string()));
    assert_eq!(db.load(id2).unwrap(), Some("world".to_string()));
}

#[test]
fn load_missing_id() {
    let tmp = NamedTempFile::new().unwrap();
    let db = AnchorDB::open(tmp.path()).unwrap();

    assert_eq!(db.load(999).unwrap(), None);
}

#[test]
fn save_multiple_and_load_all() {
    let tmp = NamedTempFile::new().unwrap();
    let db = AnchorDB::open(tmp.path()).unwrap();

    let mut ids = vec![];
    for i in 0..10 {
        ids.push(db.save(&format!("data-{i}")).unwrap());
    }

    for (i, id) in ids.iter().enumerate() {
        assert_eq!(db.load(*id).unwrap(), Some(format!("data-{i}")));
    }
}

#[test]
fn save_empty_string() {
    let tmp = NamedTempFile::new().unwrap();
    let db = AnchorDB::open(tmp.path()).unwrap();

    let id = db.save("").unwrap();
    assert_eq!(db.load(id).unwrap(), Some("".to_string()));
}

#[test]
fn close_creates_idx_and_reopen_loads_it() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    // Write data and close explicitly
    {
        let db = AnchorDB::open(&db_path).unwrap();
        db.save("alpha").unwrap();
        db.save("beta").unwrap();
        db.save("gamma").unwrap();
        db.close().unwrap();
    }

    // Verify .idx file was created
    let mut idx_path = db_path.clone();
    let mut ext = idx_path.extension().unwrap_or_default().to_os_string();
    ext.push(".idx");
    idx_path.set_extension(ext);
    assert!(idx_path.exists(), ".idx file should exist after close()");

    // Reopen — should load from .idx and still access all data
    let db2 = AnchorDB::open(&db_path).unwrap();
    assert_eq!(db2.load(1).unwrap(), Some("alpha".to_string()));
    assert_eq!(db2.load(2).unwrap(), Some("beta".to_string()));
    assert_eq!(db2.load(3).unwrap(), Some("gamma".to_string()));

    // New saves should continue from next_id=4
    let id4 = db2.save("delta").unwrap();
    assert_eq!(id4, 4);
    assert_eq!(db2.load(id4).unwrap(), Some("delta".to_string()));
}

#[test]
fn idx_file_deserializes_successfully() {
    // Direct check that the .idx file written by close() can be read back.
    // Before the fix, INDEX_ENTRY_SIZE mismatched actual on-disk layout and
    // deserialize_from_file returned UnexpectedEof, silently falling back to
    // full scan.
    use anchordb::index::Index;

    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    {
        let db = AnchorDB::open(&db_path).unwrap();
        db.save("alpha").unwrap();
        db.save("beta").unwrap();
        db.save("gamma").unwrap();
        db.close().unwrap();
    }

    let mut idx_path = db_path.clone();
    let mut ext = idx_path.extension().unwrap_or_default().to_os_string();
    ext.push(".idx");
    idx_path.set_extension(ext);

    let (index, latest_id, _max_offset) =
        Index::deserialize_from_file(&idx_path).expect(".idx must deserialize");
    assert_eq!(index.len(), 3);
    assert_eq!(latest_id, 3);
    assert!(index.contains(1));
    assert!(index.contains(2));
    assert!(index.contains(3));
}

#[test]
fn drop_saves_idx_automatically() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    // Write data, let Drop handle .idx saving
    {
        let db = AnchorDB::open(&db_path).unwrap();
        db.save("one").unwrap();
        db.save("two").unwrap();
        // no explicit close() — Drop should save .idx
    }

    // Reopen and verify data is accessible
    let db2 = AnchorDB::open(&db_path).unwrap();
    assert_eq!(db2.load(1).unwrap(), Some("one".to_string()));
    assert_eq!(db2.load(2).unwrap(), Some("two".to_string()));
}

#[test]
fn auxiliary_on_empty_db() {
    let tmp = NamedTempFile::new().unwrap();
    let db = AnchorDB::open(tmp.path()).unwrap();

    assert!(db.is_empty());
    assert_eq!(db.len(), 0);
    assert!(db.keys().is_empty());
    assert!(!db.exists(1));
}

#[test]
fn auxiliary_after_saves() {
    let tmp = NamedTempFile::new().unwrap();
    let db = AnchorDB::open(tmp.path()).unwrap();

    let id1 = db.save("alpha").unwrap();
    let id2 = db.save("beta").unwrap();
    let id3 = db.save("gamma").unwrap();

    assert!(!db.is_empty());
    assert_eq!(db.len(), 3);

    assert!(db.exists(id1));
    assert!(db.exists(id2));
    assert!(db.exists(id3));
    assert!(!db.exists(999));

    let mut keys = db.keys();
    keys.sort();
    assert_eq!(keys, vec![id1, id2, id3]);
}

#[test]
fn delete_removes_record_and_persists_across_reopen() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    let id;
    {
        let db = AnchorDB::open(&db_path).unwrap();
        id = db.save("to-be-deleted").unwrap();
        db.save("survivor").unwrap();

        assert!(db.delete(id).unwrap());
        assert!(!db.exists(id));
        assert_eq!(db.load(id).unwrap(), None);

        // Second delete on same id is a no-op.
        assert!(!db.delete(id).unwrap());
        db.close().unwrap();
    }

    // Reopen — tombstone must be honored.
    let db2 = AnchorDB::open(&db_path).unwrap();
    assert!(!db2.exists(id));
    assert_eq!(db2.load(id).unwrap(), None);
    assert_eq!(db2.load(2).unwrap(), Some("survivor".to_string()));
}

#[test]
fn save_with_opts_round_trips_priority_and_data_type() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    let id;
    {
        let db = AnchorDB::open(&db_path).unwrap();
        id = db
            .save_with_opts(b"\x00\x01\x02", DataType::Bytes, Priority::Critical, &[])
            .unwrap();
        db.close().unwrap();
    }

    let db2 = AnchorDB::open(&db_path).unwrap();
    let rec = db2.fetch_memory(id).unwrap().expect("record exists");
    assert_eq!(rec.priority, Priority::Critical);
    assert_eq!(rec.data, AnchorData::Bytes(vec![0x00, 0x01, 0x02]));
    assert!(rec.tags.is_empty());
}

#[test]
fn append_memory_round_trips_tags() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    let input = MemoryInput {
        tags: vec!["src/foo.rs".to_string(), "lines:10-20".to_string()],
        data: AnchorData::Str("body".to_string()),
        priority: Priority::Important,
    };

    let id;
    {
        let db = AnchorDB::open(&db_path).unwrap();
        id = db.append_memory(&input).unwrap();
        db.close().unwrap();
    }

    let db2 = AnchorDB::open(&db_path).unwrap();
    let rec = db2.fetch_memory(id).unwrap().expect("record exists");
    assert_eq!(rec.tags, input.tags);
    assert_eq!(rec.data, input.data);
    assert_eq!(rec.priority, Priority::Important);
}

#[test]
fn load_returns_invalid_data_on_non_utf8() {
    let tmp = NamedTempFile::new().unwrap();
    let db = AnchorDB::open(tmp.path()).unwrap();

    let id = db
        .save_with_opts(&[0xFF, 0xFE, 0xFD], DataType::Bytes, Priority::Normal, &[])
        .unwrap();

    let err = db.load(id).expect_err("non-utf8 must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}
