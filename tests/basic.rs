use anchordb::AnchorDB;
use tempfile::NamedTempFile;

#[test]
fn save_and_load() {
    let tmp = NamedTempFile::new().unwrap();
    let mut db = AnchorDB::open(tmp.path()).unwrap();

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
    let mut db = AnchorDB::open(tmp.path()).unwrap();

    assert_eq!(db.load(999).unwrap(), None);
}

#[test]
fn save_multiple_and_load_all() {
    let tmp = NamedTempFile::new().unwrap();
    let mut db = AnchorDB::open(tmp.path()).unwrap();

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
    let mut db = AnchorDB::open(tmp.path()).unwrap();

    let id = db.save("").unwrap();
    assert_eq!(db.load(id).unwrap(), Some("".to_string()));
}
