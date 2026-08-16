use super::*;

pub(super) fn write_header(output: &mut [u8], header: Header) {
    output[..MAGIC.len()].copy_from_slice(&MAGIC);
    output[4] = FORMAT_VERSION;
    output[5] = header.target_id;
    output[6..8].copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
    write_u32(output, CODE_SIZE_OFFSET, header.code_size);
    write_u32(output, DATA_INIT_SIZE_OFFSET, header.data_init_size);
    write_u32(output, DATA_ZERO_SIZE_OFFSET, header.data_zero_size);
    write_u32(output, STACK_SIZE_OFFSET, header.stack_size);
    write_u32(output, LINKED_CODE_BASE_OFFSET, header.linked_code_base);
    write_u32(output, LINKED_DATA_BASE_OFFSET, header.linked_data_base);
    write_u32(output, CODE_LOAD_ADDRESS_OFFSET, header.code_load_address);
    write_u32(output, DATA_LOAD_ADDRESS_OFFSET, header.data_load_address);
    write_u32(output, EXECUTION_OFFSET_OFFSET, header.execution_offset);
    write_u32(output, RELOCATION_OFFSET_OFFSET, header.relocation_offset);
    write_u32(output, RELOCATION_COUNT_OFFSET, header.relocation_count);
    output[RELOCATION_ENTRY_SIZE_OFFSET..RELOCATION_ENTRY_SIZE_OFFSET + 2]
        .copy_from_slice(&(RELOCATION_ENTRY_SIZE as u16).to_le_bytes());
    output[RELOCATION_RESERVED_U16_OFFSET..CRC32_OFFSET].fill(0);
    write_u32(output, CRC32_OFFSET, header.crc32);
    output[ABI_VERSION_OFFSET] = ABI_VERSION;
    output[FLAGS_OFFSET] = FLAGS;
    output[RESERVED_U16_OFFSET..RESERVED_BYTES_OFFSET].fill(0);
    output[RESERVED_BYTES_OFFSET..HEADER_SIZE].fill(0);
}

pub(super) fn parse_header(bytes: &[u8], contract: Contract) -> Result<Header, Error> {
    if bytes.len() < HEADER_SIZE {
        return Err(Error::TruncatedHeader);
    }
    if bytes[..MAGIC.len()] != MAGIC
        || bytes[4] != FORMAT_VERSION
        || bytes[5] != contract.target_id
        || read_u16(bytes, 6) != HEADER_SIZE as u16
        || read_u16(bytes, RELOCATION_ENTRY_SIZE_OFFSET) != RELOCATION_ENTRY_SIZE as u16
        || bytes[ABI_VERSION_OFFSET] != ABI_VERSION
        || bytes[FLAGS_OFFSET] != FLAGS
        || read_u16(bytes, RELOCATION_RESERVED_U16_OFFSET) != 0
        || read_u16(bytes, RESERVED_U16_OFFSET) != 0
        || bytes[RESERVED_BYTES_OFFSET..HEADER_SIZE]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(Error::InvalidHeader);
    }
    let header = Header {
        target_id: bytes[5],
        code_size: read_u32(bytes, CODE_SIZE_OFFSET),
        data_init_size: read_u32(bytes, DATA_INIT_SIZE_OFFSET),
        data_zero_size: read_u32(bytes, DATA_ZERO_SIZE_OFFSET),
        stack_size: read_u32(bytes, STACK_SIZE_OFFSET),
        linked_code_base: read_u32(bytes, LINKED_CODE_BASE_OFFSET),
        linked_data_base: read_u32(bytes, LINKED_DATA_BASE_OFFSET),
        code_load_address: read_u32(bytes, CODE_LOAD_ADDRESS_OFFSET),
        data_load_address: read_u32(bytes, DATA_LOAD_ADDRESS_OFFSET),
        execution_offset: read_u32(bytes, EXECUTION_OFFSET_OFFSET),
        relocation_offset: read_u32(bytes, RELOCATION_OFFSET_OFFSET),
        relocation_count: read_u32(bytes, RELOCATION_COUNT_OFFSET),
        crc32: read_u32(bytes, CRC32_OFFSET),
    };
    super::codec::validate_header(header, contract)?;
    Ok(header)
}

pub(super) fn checksum(code: &[u8], data: &[u8], relocations: &[u8]) -> u32 {
    let mut checksum = Crc32::new();
    checksum.update(code);
    checksum.update(data);
    checksum.update(relocations);
    checksum.finish()
}

pub(super) fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

pub(super) fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

pub(super) fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + core::mem::size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
}
