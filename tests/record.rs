use anchordb::error::AnchorError;
use anchordb::record::FileHeader;

#[test]
fn test_file_header_serialization() {
    let mut header = FileHeader::new(1);
    header.record_count = 10;
    header.total_records = 15;
    header.data_size = 1024;
    header.next_id = 16;

    let bytes = header.to_bytes();
    let decoded = FileHeader::from_bytes(&bytes).expect("Failed to deserialize");

    assert_eq!(header, decoded);
}

#[test]
fn test_file_header_checksum_mismatch() {
    let mut header = FileHeader::new(1);
    let mut bytes = header.to_bytes();
    
    // Corrupt the data size
    bytes[24] = bytes[24].wrapping_add(1);

    let result = FileHeader::from_bytes(&bytes);
    assert!(matches!(result, Err(AnchorError::ChecksumMismatch)));
}

#[test]
fn test_file_header_magic_mismatch() {
    let mut header = FileHeader::new(1);
    let mut bytes = header.to_bytes();
    
    // Corrupt magic bytes
    bytes[0] = b'B';

    let result = FileHeader::from_bytes(&bytes);
    assert!(matches!(result, Err(AnchorError::InvalidMagic)));
}
