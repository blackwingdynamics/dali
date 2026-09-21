use super::*;

const TEST_PAYLOAD: &[u8] = b"123456789";
const TEST_EXECUTION_OFFSET: u32 = 0;
const TEST_CARTRIDGE_SIZE: usize = HEADER_SIZE + TEST_PAYLOAD.len();

fn valid_cartridge() -> [u8; TEST_CARTRIDGE_SIZE] {
    let mut cartridge = [0; TEST_CARTRIDGE_SIZE];
    cartridge[..MAGIC.len()].copy_from_slice(&MAGIC);
    cartridge[FORMAT_VERSION_OFFSET] = FORMAT_VERSION;
    cartridge[TARGET_ID_OFFSET] = TARGET_ID;
    cartridge[HEADER_SIZE_OFFSET..PAYLOAD_SIZE_OFFSET]
        .copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
    cartridge[PAYLOAD_SIZE_OFFSET..LOAD_ADDRESS_OFFSET]
        .copy_from_slice(&(TEST_PAYLOAD.len() as u32).to_le_bytes());
    cartridge[LOAD_ADDRESS_OFFSET..EXECUTION_OFFSET_OFFSET]
        .copy_from_slice(&LOAD_ADDRESS.to_le_bytes());
    cartridge[EXECUTION_OFFSET_OFFSET..CRC32_OFFSET]
        .copy_from_slice(&TEST_EXECUTION_OFFSET.to_le_bytes());
    cartridge[CRC32_OFFSET..ABI_VERSION_OFFSET].copy_from_slice(&crc32(TEST_PAYLOAD).to_le_bytes());
    cartridge[ABI_VERSION_OFFSET] = ABI_VERSION;
    cartridge[FLAGS_OFFSET] = RESERVED_FLAGS;
    cartridge[RESERVED_U16_OFFSET..RESERVED_U32_OFFSET]
        .copy_from_slice(&RESERVED_U16.to_le_bytes());
    cartridge[RESERVED_U32_OFFSET..HEADER_SIZE].copy_from_slice(&RESERVED_U32.to_le_bytes());
    cartridge[PAYLOAD_OFFSET..].copy_from_slice(TEST_PAYLOAD);
    cartridge
}

#[test]
fn parses_valid_cartridge_and_payload() {
    let cartridge = valid_cartridge();
    let parsed = parse(&cartridge).unwrap();
    assert_eq!(parsed.header.magic, MAGIC);
    assert_eq!(parsed.payload, TEST_PAYLOAD);
    assert_eq!(parsed.entry_address, LOAD_ADDRESS);
}

#[test]
fn validates_header_and_payload_separately() {
    let cartridge = valid_cartridge();
    let header = parse_header(&cartridge[..HEADER_SIZE]).unwrap();
    let parsed = validate_payload(header, &cartridge[PAYLOAD_OFFSET..]).unwrap();
    assert_eq!(parsed.payload, TEST_PAYLOAD);
    assert_eq!(parsed.entry_address, LOAD_ADDRESS);
}

#[test]
fn rejects_payload_length_mismatch_after_header_validation() {
    let cartridge = valid_cartridge();
    let header = parse_header(&cartridge[..HEADER_SIZE]).unwrap();
    assert_eq!(
        validate_payload(header, &cartridge[PAYLOAD_OFFSET..PAYLOAD_OFFSET + 1]),
        Err(ParseError::PayloadOutsideCartridge)
    );
}

#[test]
fn validates_payload_in_bounded_chunks() {
    let cartridge = valid_cartridge();
    let header = parse_header(&cartridge[..HEADER_SIZE]).unwrap();
    let mut validator = PayloadValidator::new(header).unwrap();
    validator.update(&TEST_PAYLOAD[..3]).unwrap();
    validator.update(&TEST_PAYLOAD[3..]).unwrap();
    assert_eq!(
        validator.finish(),
        Ok(ValidatedPayload {
            header,
            entry_address: LOAD_ADDRESS,
        })
    );
}

#[test]
fn encodes_a_cartridge_that_the_parser_accepts() {
    let mut encoded = [0; HEADER_SIZE + TEST_PAYLOAD.len()];
    let size = encode_cartridge(TEST_PAYLOAD, TEST_EXECUTION_OFFSET, &mut encoded).unwrap();
    assert_eq!(parse(&encoded[..size]).unwrap().payload, TEST_PAYLOAD);
}

#[test]
fn rejects_an_output_buffer_that_is_too_small() {
    let mut encoded = [0; HEADER_SIZE + TEST_PAYLOAD.len() - 1];
    assert_eq!(
        encode_cartridge(TEST_PAYLOAD, TEST_EXECUTION_OFFSET, &mut encoded),
        Err(BuildError::OutputTooSmall)
    );
}

#[test]
fn rejects_payload_chunk_beyond_declared_size() {
    let cartridge = valid_cartridge();
    let header = parse_header(&cartridge[..HEADER_SIZE]).unwrap();
    let mut validator = PayloadValidator::new(header).unwrap();
    let oversized = [0; TEST_PAYLOAD.len() + 1];
    assert_eq!(
        validator.update(&oversized),
        Err(ParseError::PayloadOutsideCartridge)
    );
}

#[test]
fn rejects_invalid_magic() {
    let mut cartridge = valid_cartridge();
    cartridge[0] = b'X';
    assert_eq!(parse(&cartridge), Err(ParseError::InvalidMagic));
}

#[test]
fn rejects_unsupported_format_version() {
    let mut cartridge = valid_cartridge();
    cartridge[FORMAT_VERSION_OFFSET] = FORMAT_VERSION.saturating_add(1);
    assert_eq!(parse(&cartridge), Err(ParseError::UnsupportedFormatVersion));
}

#[test]
fn rejects_unsupported_target() {
    let mut cartridge = valid_cartridge();
    cartridge[TARGET_ID_OFFSET] = TARGET_ID.saturating_add(1);
    assert_eq!(parse(&cartridge), Err(ParseError::UnsupportedTarget));
}

#[test]
fn rejects_invalid_header_size() {
    let mut cartridge = valid_cartridge();
    cartridge[HEADER_SIZE_OFFSET..PAYLOAD_SIZE_OFFSET]
        .copy_from_slice(&(HEADER_SIZE as u16 - 1).to_le_bytes());
    assert_eq!(parse(&cartridge), Err(ParseError::InvalidHeaderSize));
}

#[test]
fn rejects_unsupported_abi_version() {
    let mut cartridge = valid_cartridge();
    cartridge[ABI_VERSION_OFFSET] = ABI_VERSION.saturating_add(1);
    assert_eq!(parse(&cartridge), Err(ParseError::UnsupportedAbiVersion));
}

#[test]
fn rejects_non_zero_reserved_fields() {
    let mut cartridge = valid_cartridge();
    cartridge[FLAGS_OFFSET] = 1;
    assert_eq!(parse(&cartridge), Err(ParseError::NonZeroReservedField));
}

#[test]
fn rejects_empty_payload() {
    let mut cartridge = valid_cartridge();
    cartridge[PAYLOAD_SIZE_OFFSET..LOAD_ADDRESS_OFFSET]
        .copy_from_slice(&RESERVED_U32.to_le_bytes());
    assert_eq!(parse(&cartridge), Err(ParseError::EmptyPayload));
}

#[test]
fn rejects_oversized_payload() {
    let mut cartridge = valid_cartridge();
    let oversized = (MAX_PAYLOAD_SIZE as u32).saturating_add(1);
    cartridge[PAYLOAD_SIZE_OFFSET..LOAD_ADDRESS_OFFSET].copy_from_slice(&oversized.to_le_bytes());
    assert_eq!(parse(&cartridge), Err(ParseError::PayloadTooLarge));
}

#[test]
fn rejects_truncated_header() {
    assert_eq!(
        parse(&[0; HEADER_SIZE - 1]),
        Err(ParseError::TruncatedHeader)
    );
}

#[test]
fn rejects_payload_outside_cartridge() {
    let mut cartridge = valid_cartridge();
    let outside_size = (TEST_PAYLOAD.len() as u32).saturating_add(1);
    cartridge[PAYLOAD_SIZE_OFFSET..LOAD_ADDRESS_OFFSET]
        .copy_from_slice(&outside_size.to_le_bytes());
    assert_eq!(parse(&cartridge), Err(ParseError::PayloadOutsideCartridge));
}

#[test]
fn rejects_invalid_load_address() {
    let mut cartridge = valid_cartridge();
    let invalid_address = LOAD_ADDRESS.saturating_add(WORD_ALIGNMENT);
    cartridge[LOAD_ADDRESS_OFFSET..EXECUTION_OFFSET_OFFSET]
        .copy_from_slice(&invalid_address.to_le_bytes());
    assert_eq!(parse(&cartridge), Err(ParseError::InvalidLoadAddress));
}

#[test]
fn rejects_invalid_execution_offset() {
    let mut cartridge = valid_cartridge();
    let invalid_offset = TEST_PAYLOAD.len() as u32;
    cartridge[EXECUTION_OFFSET_OFFSET..CRC32_OFFSET].copy_from_slice(&invalid_offset.to_le_bytes());
    assert_eq!(parse(&cartridge), Err(ParseError::InvalidExecutionOffset));
}

#[test]
fn rejects_unaligned_execution_offset() {
    let mut cartridge = valid_cartridge();
    let unaligned_offset: u32 = 1;
    cartridge[EXECUTION_OFFSET_OFFSET..CRC32_OFFSET]
        .copy_from_slice(&unaligned_offset.to_le_bytes());
    assert_eq!(parse(&cartridge), Err(ParseError::UnalignedExecutionOffset));
}

#[test]
fn rejects_crc_mismatch() {
    let mut cartridge = valid_cartridge();
    cartridge[CRC32_OFFSET] ^= 1;
    assert_eq!(parse(&cartridge), Err(ParseError::CrcMismatch));
}
