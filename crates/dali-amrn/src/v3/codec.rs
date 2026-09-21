use super::wire;
use super::*;

/// Parses and validates an AMRN v3 cartridge against a target contract.
pub fn parse<'a>(cartridge: &'a [u8], contract: Contract) -> Result<Cartridge<'a>, Error> {
    let header = wire::parse_header(cartridge, contract)?;
    let code_size = usize::try_from(header.code_size).map_err(|_| Error::InvalidPayload)?;
    let data_size = usize::try_from(header.data_init_size).map_err(|_| Error::InvalidPayload)?;
    let table_size = relocation_table_size(header.relocation_count)?;
    let code_end = HEADER_SIZE
        .checked_add(code_size)
        .ok_or(Error::InvalidPayload)?;
    let data_end = code_end
        .checked_add(data_size)
        .ok_or(Error::InvalidPayload)?;
    let cartridge_size = data_end
        .checked_add(table_size)
        .ok_or(Error::InvalidPayload)?;
    if header.relocation_offset as usize != data_end || cartridge.len() != cartridge_size {
        return Err(Error::InvalidPayload);
    }
    let code = &cartridge[HEADER_SIZE..code_end];
    let initialized_data = &cartridge[code_end..data_end];
    let relocation_bytes = &cartridge[data_end..cartridge_size];
    if wire::checksum(code, initialized_data, relocation_bytes) != header.crc32 {
        return Err(Error::CrcMismatch);
    }
    validate_relocations(header, contract, relocation_bytes)?;
    Ok(Cartridge {
        header,
        code,
        initialized_data,
        relocation_bytes,
    })
}

/// Encodes an AMRN v3 cartridge into caller-provided storage.
pub fn encode(image: Image<'_>, contract: Contract, output: &mut [u8]) -> Result<usize, Error> {
    let header = header_from_image(image, contract)?;
    let table_size = relocation_table_size(header.relocation_count)?;
    let code_end = HEADER_SIZE
        .checked_add(image.code.len())
        .ok_or(Error::OutputTooSmall)?;
    let data_end = code_end
        .checked_add(image.initialized_data.len())
        .ok_or(Error::OutputTooSmall)?;
    let cartridge_size = data_end
        .checked_add(table_size)
        .ok_or(Error::OutputTooSmall)?;
    if output.len() < cartridge_size {
        return Err(Error::OutputTooSmall);
    }
    output[HEADER_SIZE..code_end].copy_from_slice(image.code);
    output[code_end..data_end].copy_from_slice(image.initialized_data);
    let relocation_bytes = &mut output[data_end..cartridge_size];
    encode_relocations(image.relocations, relocation_bytes)?;
    let mut header = header;
    header.crc32 = wire::checksum(image.code, image.initialized_data, relocation_bytes);
    wire::write_header(output, header);
    Ok(cartridge_size)
}

fn header_from_image(image: Image<'_>, contract: Contract) -> Result<Header, Error> {
    let code_size = u32::try_from(image.code.len()).map_err(|_| Error::RegionOverflow)?;
    let data_init_size =
        u32::try_from(image.initialized_data.len()).map_err(|_| Error::RegionOverflow)?;
    let relocation_count =
        u32::try_from(image.relocations.len()).map_err(|_| Error::InvalidRelocation)?;
    let header = Header {
        target_id: contract.target_id,
        code_size,
        data_init_size,
        data_zero_size: image.data_zero_size,
        stack_size: image.stack_size,
        linked_code_base: image.linked_code_base,
        linked_data_base: image.linked_data_base,
        code_load_address: contract.code_load_address,
        data_load_address: contract.data_load_address,
        execution_offset: image.execution_offset,
        relocation_offset: u32::try_from(HEADER_SIZE)
            .ok()
            .and_then(|offset| offset.checked_add(code_size))
            .and_then(|offset| offset.checked_add(data_init_size))
            .ok_or(Error::AddressOverflow)?,
        relocation_count,
        crc32: 0,
    };
    validate_header(header, contract)?;
    for relocation in image.relocations {
        validate_relocation(header, contract, *relocation)?;
    }
    Ok(header)
}

pub fn validate_header(header: Header, contract: Contract) -> Result<(), Error> {
    if header.code_size == 0
        || header.stack_size == 0
        || header.code_load_address != contract.code_load_address
        || header.data_load_address != contract.data_load_address
        || !header.linked_code_base.is_multiple_of(WORD_ALIGNMENT)
        || !header.linked_data_base.is_multiple_of(WORD_ALIGNMENT)
        || header.execution_offset >= header.code_size
        || !header.execution_offset.is_multiple_of(WORD_ALIGNMENT)
        || header.code_size > contract.code_capacity
    {
        return Err(Error::InvalidHeader);
    }
    let data_used = header
        .data_init_size
        .checked_add(header.data_zero_size)
        .and_then(|size| size.checked_add(header.stack_size))
        .ok_or(Error::RegionOverflow)?;
    if data_used > contract.data_capacity {
        return Err(Error::RegionOverflow);
    }
    contract
        .code_load_address
        .checked_add(header.code_size)
        .ok_or(Error::RegionOverflow)?;
    contract
        .data_load_address
        .checked_add(data_used)
        .ok_or(Error::RegionOverflow)?;
    header
        .linked_code_base
        .checked_add(header.code_size)
        .ok_or(Error::RegionOverflow)?;
    header
        .linked_data_base
        .checked_add(data_used)
        .ok_or(Error::RegionOverflow)?;
    if header.relocation_count as usize > MAX_RELOCATION_ENTRIES
        || header.relocation_offset < HEADER_SIZE as u32
    {
        return Err(Error::InvalidHeader);
    }
    Ok(())
}

fn validate_relocations(header: Header, contract: Contract, bytes: &[u8]) -> Result<(), Error> {
    let expected = relocation_table_size(header.relocation_count)?;
    if bytes.len() != expected {
        return Err(Error::InvalidRelocation);
    }
    for index in 0..header.relocation_count as usize {
        let start = index * RELOCATION_ENTRY_SIZE;
        let relocation = decode_relocation(&bytes[start..start + RELOCATION_ENTRY_SIZE])?;
        validate_relocation(header, contract, relocation)?;
    }
    Ok(())
}

pub fn validate_relocation(
    header: Header,
    contract: Contract,
    relocation: Relocation,
) -> Result<(), Error> {
    let (segment_base, segment_size) = match relocation.segment {
        Segment::Code => (header.code_load_address, header.code_size),
        Segment::Data => (contract.data_load_address, header.data_init_size),
    };
    let patch_end = relocation
        .patch_offset
        .checked_add(relocation.kind.patch_width())
        .ok_or(Error::InvalidRelocation)?;
    if !relocation
        .patch_offset
        .is_multiple_of(relocation.kind.patch_alignment())
        || patch_end > segment_size
    {
        return Err(Error::InvalidRelocation);
    }
    let code_end = header
        .linked_code_base
        .checked_add(header.code_size)
        .ok_or(Error::AddressOverflow)?;
    let data_end = header
        .linked_data_base
        .checked_add(header.data_init_size)
        .and_then(|end| end.checked_add(header.data_zero_size))
        .and_then(|end| end.checked_add(header.stack_size))
        .ok_or(Error::AddressOverflow)?;
    let target = relocation.linked_target;
    let target_in_code = target >= header.linked_code_base && target < code_end;
    let target_in_data = target >= header.linked_data_base && target < data_end;
    if !target_in_code && !target_in_data {
        return Err(Error::InvalidRelocation);
    }
    segment_base
        .checked_add(relocation.patch_offset)
        .ok_or(Error::AddressOverflow)?;
    Ok(())
}

fn relocation_table_size(count: u32) -> Result<usize, Error> {
    usize::try_from(count)
        .ok()
        .and_then(|count| count.checked_mul(RELOCATION_ENTRY_SIZE))
        .ok_or(Error::InvalidRelocation)
}

fn encode_relocations(relocations: &[Relocation], output: &mut [u8]) -> Result<(), Error> {
    if relocations.len() > MAX_RELOCATION_ENTRIES
        || output.len() != relocations.len() * RELOCATION_ENTRY_SIZE
    {
        return Err(Error::InvalidRelocation);
    }
    for (index, relocation) in relocations.iter().enumerate() {
        let start = index * RELOCATION_ENTRY_SIZE;
        let bytes = &mut output[start..start + RELOCATION_ENTRY_SIZE];
        bytes[0] = relocation.segment.raw();
        bytes[1] = relocation.kind.raw();
        bytes[2..4].fill(0);
        bytes[4..8].copy_from_slice(&relocation.patch_offset.to_le_bytes());
        bytes[8..12].copy_from_slice(&relocation.linked_target.to_le_bytes());
        bytes[12..16].copy_from_slice(&relocation.addend.to_le_bytes());
    }
    Ok(())
}

pub fn decode_relocation(bytes: &[u8]) -> Result<Relocation, Error> {
    if bytes.len() != RELOCATION_ENTRY_SIZE || bytes[2] != 0 || bytes[3] != 0 {
        return Err(Error::InvalidRelocation);
    }
    Ok(Relocation {
        segment: Segment::from_raw(bytes[0]).ok_or(Error::InvalidRelocation)?,
        kind: RelocationKind::from_raw(bytes[1]).ok_or(Error::InvalidRelocation)?,
        patch_offset: wire::read_u32(bytes, 4),
        linked_target: wire::read_u32(bytes, 8),
        addend: i32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
    })
}
