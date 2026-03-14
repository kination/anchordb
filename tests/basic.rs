use anchordb::AnchorDB;
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
