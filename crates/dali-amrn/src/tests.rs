use super::*;

const TEST_PAYLOAD: &[u8] = b"123456789";
const TEST_EXECUTION_OFFSET: u32 = 0;
const TEST_PACKAGE_SIZE: usize = HEADER_SIZE + TEST_PAYLOAD.len();

fn valid_package() -> [u8; TEST_PACKAGE_SIZE] {
    let mut package = [0; TEST_PACKAGE_SIZE];
    package[..MAGIC.len()].copy_from_slice(&MAGIC);
    package[FORMAT_VERSION_OFFSET] = FORMAT_VERSION;
    package[TARGET_ID_OFFSET] = TARGET_ID;
    package[HEADER_SIZE_OFFSET..PAYLOAD_SIZE_OFFSET]
        .copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
    package[PAYLOAD_SIZE_OFFSET..LOAD_ADDRESS_OFFSET]
        .copy_from_slice(&(TEST_PAYLOAD.len() as u32).to_le_bytes());
    package[LOAD_ADDRESS_OFFSET..EXECUTION_OFFSET_OFFSET]
        .copy_from_slice(&LOAD_ADDRESS.to_le_bytes());
    package[EXECUTION_OFFSET_OFFSET..CRC32_OFFSET]
        .copy_from_slice(&TEST_EXECUTION_OFFSET.to_le_bytes());
    package[CRC32_OFFSET..ABI_VERSION_OFFSET].copy_from_slice(&crc32(TEST_PAYLOAD).to_le_bytes());
    package[ABI_VERSION_OFFSET] = ABI_VERSION;
    package[FLAGS_OFFSET] = RESERVED_FLAGS;
    package[RESERVED_U16_OFFSET..RESERVED_U32_OFFSET].copy_from_slice(&RESERVED_U16.to_le_bytes());
    package[RESERVED_U32_OFFSET..HEADER_SIZE].copy_from_slice(&RESERVED_U32.to_le_bytes());
    package[PAYLOAD_OFFSET..].copy_from_slice(TEST_PAYLOAD);
    package
}

#[test]
fn parses_valid_package_and_payload() {
    let package = valid_package();
    let parsed = parse(&package).unwrap();
    assert_eq!(parsed.header.magic, MAGIC);
    assert_eq!(parsed.payload, TEST_PAYLOAD);
    assert_eq!(parsed.entry_address, LOAD_ADDRESS);
}

#[test]
fn rejects_invalid_magic() {
    let mut package = valid_package();
    package[0] = b'X';
    assert_eq!(parse(&package), Err(ParseError::InvalidMagic));
}

#[test]
fn rejects_unsupported_format_version() {
    let mut package = valid_package();
    package[FORMAT_VERSION_OFFSET] = FORMAT_VERSION.saturating_add(1);
    assert_eq!(parse(&package), Err(ParseError::UnsupportedFormatVersion));
}

#[test]
fn rejects_unsupported_target() {
    let mut package = valid_package();
    package[TARGET_ID_OFFSET] = TARGET_ID.saturating_add(1);
    assert_eq!(parse(&package), Err(ParseError::UnsupportedTarget));
}

#[test]
fn rejects_invalid_header_size() {
    let mut package = valid_package();
    package[HEADER_SIZE_OFFSET..PAYLOAD_SIZE_OFFSET]
        .copy_from_slice(&(HEADER_SIZE as u16 - 1).to_le_bytes());
    assert_eq!(parse(&package), Err(ParseError::InvalidHeaderSize));
}

#[test]
fn rejects_unsupported_abi_version() {
    let mut package = valid_package();
    package[ABI_VERSION_OFFSET] = ABI_VERSION.saturating_add(1);
    assert_eq!(parse(&package), Err(ParseError::UnsupportedAbiVersion));
}

#[test]
fn rejects_non_zero_reserved_fields() {
    let mut package = valid_package();
    package[FLAGS_OFFSET] = 1;
    assert_eq!(parse(&package), Err(ParseError::NonZeroReservedField));
}

#[test]
fn rejects_empty_payload() {
    let mut package = valid_package();
    package[PAYLOAD_SIZE_OFFSET..LOAD_ADDRESS_OFFSET].copy_from_slice(&RESERVED_U32.to_le_bytes());
    assert_eq!(parse(&package), Err(ParseError::EmptyPayload));
}

#[test]
fn rejects_oversized_payload() {
    let mut package = valid_package();
    let oversized = (MAX_PAYLOAD_SIZE as u32).saturating_add(1);
    package[PAYLOAD_SIZE_OFFSET..LOAD_ADDRESS_OFFSET].copy_from_slice(&oversized.to_le_bytes());
    assert_eq!(parse(&package), Err(ParseError::PayloadTooLarge));
}

#[test]
fn rejects_truncated_header() {
    assert_eq!(
        parse(&[0; HEADER_SIZE - 1]),
        Err(ParseError::TruncatedHeader)
    );
}

#[test]
fn rejects_payload_outside_package() {
    let mut package = valid_package();
    let outside_size = (TEST_PAYLOAD.len() as u32).saturating_add(1);
    package[PAYLOAD_SIZE_OFFSET..LOAD_ADDRESS_OFFSET].copy_from_slice(&outside_size.to_le_bytes());
    assert_eq!(parse(&package), Err(ParseError::PayloadOutsidePackage));
}

#[test]
fn rejects_invalid_load_address() {
    let mut package = valid_package();
    let invalid_address = LOAD_ADDRESS.saturating_add(WORD_ALIGNMENT);
    package[LOAD_ADDRESS_OFFSET..EXECUTION_OFFSET_OFFSET]
        .copy_from_slice(&invalid_address.to_le_bytes());
    assert_eq!(parse(&package), Err(ParseError::InvalidLoadAddress));
}

#[test]
fn rejects_invalid_execution_offset() {
    let mut package = valid_package();
    let invalid_offset = TEST_PAYLOAD.len() as u32;
    package[EXECUTION_OFFSET_OFFSET..CRC32_OFFSET].copy_from_slice(&invalid_offset.to_le_bytes());
    assert_eq!(parse(&package), Err(ParseError::InvalidExecutionOffset));
}

#[test]
fn rejects_unaligned_execution_offset() {
    let mut package = valid_package();
    let unaligned_offset: u32 = 1;
    package[EXECUTION_OFFSET_OFFSET..CRC32_OFFSET].copy_from_slice(&unaligned_offset.to_le_bytes());
    assert_eq!(parse(&package), Err(ParseError::UnalignedExecutionOffset));
}

#[test]
fn rejects_crc_mismatch() {
    let mut package = valid_package();
    package[CRC32_OFFSET] ^= 1;
    assert_eq!(parse(&package), Err(ParseError::CrcMismatch));
}
