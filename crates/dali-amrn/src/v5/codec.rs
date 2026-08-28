//! Encoding and parsing for the signed v5 container.

use super::*;

/// Encodes the signed byte range and returns its length.
pub fn encode_unsigned(
    image: Image<'_>,
    contract: v3::Contract,
    output: &mut [u8],
) -> Result<usize, Error> {
    let payload_size = image_payload_size(image.image)?;
    let signed_size = HEADER_SIZE
        .checked_add(payload_size)
        .ok_or(Error::OutputTooSmall)?;
    if output.len() < signed_size || signed_size > u32::MAX as usize {
        return Err(Error::OutputTooSmall);
    }
    let v3_size = v3::encode(
        image.image,
        contract,
        &mut output[HEADER_SIZE - v3::HEADER_SIZE..],
    )
    .map_err(|_| Error::InvalidPayload)?;
    output.copy_within(
        HEADER_SIZE..HEADER_SIZE + v3_size - v3::HEADER_SIZE,
        HEADER_SIZE,
    );
    let image_header = v3::parse_header(
        &output[HEADER_SIZE - v3::HEADER_SIZE..HEADER_SIZE],
        contract,
    )
    .map_err(|_| Error::InvalidHeader)?;
    write_header(output, image_header, image.metadata, signed_size);
    let checksum = cartridge_checksum(&output[..signed_size]);
    write_u32(output, CARTRIDGE_CRC32_OFFSET, checksum);
    Ok(signed_size)
}

/// Appends a fixed DSIG envelope to an encoded signed range.
pub fn append_signature(
    signed_bytes: &[u8],
    key_id: &[u8; signature::KEY_ID_LENGTH],
    signature_bytes: &[u8; signature::SIGNATURE_LENGTH],
    output: &mut [u8],
) -> Result<usize, Error> {
    if signed_bytes.len() < HEADER_SIZE {
        return Err(Error::TruncatedHeader);
    }
    let total_size = signed_bytes
        .len()
        .checked_add(SIGNATURE_SIZE)
        .ok_or(Error::OutputTooSmall)?;
    if output.len() < total_size || signed_bytes.len() > u32::MAX as usize {
        return Err(Error::OutputTooSmall);
    }
    output[..signed_bytes.len()].copy_from_slice(signed_bytes);
    write_u32(output, SIGNATURE_OFFSET_FIELD, signed_bytes.len() as u32);
    write_u16(output, SIGNATURE_SIZE_FIELD, SIGNATURE_SIZE as u16);
    write_u32(output, SIGNED_SIZE_FIELD, signed_bytes.len() as u32);
    output[RESERVED_START..HEADER_SIZE].fill(0);
    let envelope = &mut output[signed_bytes.len()..total_size];
    envelope[..signature::MAGIC.len()].copy_from_slice(&signature::MAGIC);
    envelope[signature::VERSION_OFFSET] = signature::ENVELOPE_VERSION;
    envelope[signature::ALGORITHM_OFFSET] = signature::ED25519_ALGORITHM;
    envelope[signature::KEY_ID_LENGTH_OFFSET] = signature::KEY_ID_LENGTH as u8;
    envelope[signature::SIGNATURE_LENGTH_OFFSET] = signature::SIGNATURE_LENGTH as u8;
    envelope[signature::KEY_ID_OFFSET..signature::SIGNATURE_OFFSET].copy_from_slice(key_id);
    envelope[signature::SIGNATURE_OFFSET..].copy_from_slice(signature_bytes);
    Ok(total_size)
}

/// Parses and validates a complete signed cartridge against a target contract.
pub fn parse<'a>(cartridge: &'a [u8], contract: v3::Contract) -> Result<Cartridge<'a>, Error> {
    if cartridge.len() < HEADER_SIZE {
        return Err(Error::TruncatedHeader);
    }
    let signed_size = read_u32(cartridge, SIGNED_SIZE_FIELD) as usize;
    let signature_end = signed_size
        .checked_add(SIGNATURE_SIZE)
        .ok_or(Error::InvalidSignature)?;
    if signature_end > cartridge.len() {
        return Err(Error::InvalidSignature);
    }
    let header_bytes: &[u8; HEADER_SIZE] = cartridge[..HEADER_SIZE]
        .try_into()
        .map_err(|_| Error::TruncatedHeader)?;
    let header = parse_header_parts(
        header_bytes,
        &cartridge[signed_size..signature_end],
        contract,
    )?;
    let code_size = header.image.code_size as usize;
    let data_size = header.image.data_init_size as usize;
    let table_size = (header.image.relocation_count as usize)
        .checked_mul(v3::RELOCATION_ENTRY_SIZE)
        .ok_or(Error::InvalidRelocation)?;
    let code_end = HEADER_SIZE
        .checked_add(code_size)
        .ok_or(Error::InvalidPayload)?;
    let data_end = code_end
        .checked_add(data_size)
        .ok_or(Error::InvalidPayload)?;
    let signed_size = data_end
        .checked_add(table_size)
        .ok_or(Error::InvalidPayload)?;
    if header.image.relocation_offset as usize != data_end
        || cartridge.len() != signature_end
        || read_u32(cartridge, SIGNATURE_OFFSET_FIELD) as usize != signed_size
        || read_u32(cartridge, SIGNED_SIZE_FIELD) as usize != signed_size
    {
        return Err(Error::InvalidPayload);
    }
    if cartridge_checksum(&cartridge[..signed_size]) != header.cartridge_crc32 {
        return Err(Error::CrcMismatch);
    }
    let code = &cartridge[HEADER_SIZE..code_end];
    let initialized_data = &cartridge[code_end..data_end];
    let relocation_bytes = &cartridge[data_end..signed_size];
    if payload_checksum(code, initialized_data, relocation_bytes) != header.image.crc32 {
        return Err(Error::CrcMismatch);
    }
    for index in 0..header.image.relocation_count as usize {
        let start = index * v3::RELOCATION_ENTRY_SIZE;
        let relocation =
            v3::decode_relocation(&relocation_bytes[start..start + v3::RELOCATION_ENTRY_SIZE])
                .map_err(|_| Error::InvalidRelocation)?;
        v3::validate_relocation(header.image, contract, relocation)
            .map_err(|_| Error::InvalidRelocation)?;
    }
    Ok(Cartridge {
        header,
        code,
        initialized_data,
        relocation_bytes,
        signed_bytes: &cartridge[..signed_size],
    })
}

/// Parses a v5 fixed header and its separately read DSIG trailer.
pub fn parse_header_parts<'a>(
    header: &[u8; HEADER_SIZE],
    envelope: &'a [u8],
    contract: v3::Contract,
) -> Result<Header<'a>, Error> {
    if header[MAGIC_OFFSET..MAGIC_OFFSET + MAGIC.len()] != MAGIC
        || header[FORMAT_VERSION_OFFSET] != FORMAT_VERSION
        || read_u16(header, HEADER_SIZE_OFFSET) != HEADER_SIZE as u16
    {
        return Err(Error::InvalidHeaderPrefix);
    }
    if read_u16(header, SIGNATURE_SIZE_FIELD) != SIGNATURE_SIZE as u16
        || read_u16(header, SIGNATURE_SIZE_FIELD + 2) != 0
        || header[V4_RESERVED_START..CARTRIDGE_ID_OFFSET]
            .iter()
            .any(|byte| *byte != 0)
        || header[FLAGS_OFFSET] != 0
        || read_u16(header, RESERVED_U16_OFFSET) != 0
        || header[RESERVED_START..HEADER_SIZE]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(Error::InvalidHeaderReserved);
    }
    let cartridge_id = read_array::<{ v4::CARTRIDGE_ID_LENGTH }>(header, CARTRIDGE_ID_OFFSET);
    if cartridge_id.iter().all(|byte| *byte == 0) {
        return Err(Error::InvalidIdentity);
    }
    let image = v3::Header {
        target_id: header[TARGET_ID_OFFSET],
        code_size: read_u32(header, CODE_SIZE_OFFSET),
        data_init_size: read_u32(header, DATA_INIT_SIZE_OFFSET),
        data_zero_size: read_u32(header, DATA_ZERO_SIZE_OFFSET),
        stack_size: read_u32(header, STACK_SIZE_OFFSET),
        linked_code_base: read_u32(header, LINKED_CODE_BASE_OFFSET),
        linked_data_base: read_u32(header, LINKED_DATA_BASE_OFFSET),
        code_load_address: read_u32(header, CODE_LOAD_ADDRESS_OFFSET),
        data_load_address: read_u32(header, DATA_LOAD_ADDRESS_OFFSET),
        execution_offset: read_u32(header, EXECUTION_OFFSET_OFFSET),
        relocation_offset: read_u32(header, RELOCATION_OFFSET_OFFSET),
        relocation_count: read_u32(header, RELOCATION_COUNT_OFFSET),
        crc32: read_u32(header, PAYLOAD_CRC32_OFFSET),
    };
    if image.target_id != contract.target_id
        || read_u16(header, RELOCATION_ENTRY_SIZE_OFFSET) != v3::RELOCATION_ENTRY_SIZE as u16
        || header[ABI_VERSION_OFFSET] != v3::ABI_VERSION
        || read_u16(header, RELOCATION_ENTRY_SIZE_OFFSET + 2) != 0
        || header[ABI_VERSION_OFFSET + 1] != 0
        || read_u16(header, ABI_VERSION_OFFSET + 2) != 0
    {
        return Err(Error::InvalidHeaderEncoding);
    }
    v3::validate_header(image, contract).map_err(|_| Error::InvalidContract)?;
    let signature_offset = read_u32(header, SIGNATURE_OFFSET_FIELD) as usize;
    if signature_offset < HEADER_SIZE
        || signature_offset != read_u32(header, SIGNED_SIZE_FIELD) as usize
    {
        return Err(Error::InvalidSignature);
    }
    let envelope = signature::parse(envelope).map_err(|_| Error::InvalidSignature)?;
    Ok(Header {
        image,
        metadata: v4::Metadata {
            cartridge_id,
            cartridge_version: read_version(header, CARTRIDGE_VERSION_OFFSET),
            minimum_kernel_version: read_version(header, MINIMUM_KERNEL_VERSION_OFFSET),
            required_services: read_u32(header, REQUIRED_SERVICES_OFFSET),
            slot_id: header[SLOT_ID_OFFSET],
        },
        cartridge_crc32: read_u32(header, CARTRIDGE_CRC32_OFFSET),
        signature: envelope,
    })
}
