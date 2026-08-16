//! AMRN format version 2 for the ABI v3 application contract.

use crate::{MAGIC, stream::checksum_parts};

/// The fixed AMRN v2 header length in bytes.
pub const HEADER_SIZE: usize = 64;
/// The AMRN format revision represented by this module.
pub const FORMAT_VERSION: u8 = 2;
/// The ABI revision represented by this module.
pub const ABI_VERSION: u8 = 3;
const FLAGS: u8 = 0;
const WORD_ALIGNMENT: u32 = 4;
const CODE_SIZE_OFFSET: usize = 8;
const DATA_INIT_SIZE_OFFSET: usize = 12;
const DATA_ZERO_SIZE_OFFSET: usize = 16;
const STACK_SIZE_OFFSET: usize = 20;
const CODE_LOAD_ADDRESS_OFFSET: usize = 24;
const DATA_LOAD_ADDRESS_OFFSET: usize = 28;
const EXECUTION_OFFSET_OFFSET: usize = 32;
const CRC32_OFFSET: usize = 36;
const ABI_VERSION_OFFSET: usize = 40;
const FLAGS_OFFSET: usize = 41;
const RESERVED_U16_OFFSET: usize = 42;
const RESERVED_U32_OFFSET: usize = 44;
const RESERVED_BYTES_OFFSET: usize = 48;

/// Target-owned addresses and capacities required by an ABI v3 package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Contract {
    /// AMRN target identifier.
    pub target_id: u8,
    /// Application code region origin.
    pub code_load_address: u32,
    /// Application code region capacity.
    pub code_capacity: u32,
    /// Application data and PSP region origin.
    pub data_load_address: u32,
    /// Application data and PSP region capacity.
    pub data_capacity: u32,
}

/// The explicitly decoded AMRN v2 header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    /// AMRN target identifier.
    pub target_id: u8,
    /// File code segment length.
    pub code_size: u32,
    /// File initialized-data segment length.
    pub data_init_size: u32,
    /// Runtime zero-initialized data length.
    pub data_zero_size: u32,
    /// Runtime PSP stack reservation.
    pub stack_size: u32,
    /// Application code region origin.
    pub code_load_address: u32,
    /// Application data region origin.
    pub data_load_address: u32,
    /// Entry offset from the code origin.
    pub execution_offset: u32,
    /// CRC32 of code followed by initialized data.
    pub crc32: u32,
}

/// A validated AMRN v2 package view into caller-owned bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Package<'a> {
    /// The validated header.
    pub header: Header,
    /// The file code segment.
    pub code: &'a [u8],
    /// The file initialized-data segment.
    pub initialized_data: &'a [u8],
    /// The validated entry address before the Thumb bit is applied.
    pub entry_address: u32,
    /// The lowest address reserved for the PSP stack.
    pub psp_stack_bottom: u32,
    /// The initial PSP value at the top of the stack reservation.
    pub psp_stack_top: u32,
}

/// Input segments and runtime reservations for v2 package construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Image<'a> {
    /// Native code and read-only data bytes.
    pub code: &'a [u8],
    /// Initialized writable data bytes.
    pub initialized_data: &'a [u8],
    /// Runtime zero-initialized data length.
    pub data_zero_size: u32,
    /// Runtime PSP stack reservation.
    pub stack_size: u32,
    /// Word-aligned entry offset from the code origin.
    pub execution_offset: u32,
}

/// Errors returned by AMRN v2 parsing and construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The fixed header is absent or incomplete.
    TruncatedHeader,
    /// A header field is not compatible with the supplied target contract.
    InvalidHeader,
    /// A segment or runtime reservation exceeds its target region.
    RegionOverflow,
    /// The package does not contain exactly the declared file segments.
    InvalidPayload,
    /// The entry offset is invalid.
    InvalidExecutionOffset,
    /// A checked address calculation overflowed.
    AddressOverflow,
    /// The package checksum is invalid.
    CrcMismatch,
    /// The output buffer cannot contain the package.
    OutputTooSmall,
}

/// Parses and validates an AMRN v2 package against a target contract.
pub fn parse<'a>(package: &'a [u8], contract: Contract) -> Result<Package<'a>, Error> {
    let header = parse_header(package, contract)?;
    let code_size = usize::try_from(header.code_size).map_err(|_| Error::InvalidPayload)?;
    let data_size = usize::try_from(header.data_init_size).map_err(|_| Error::InvalidPayload)?;
    let payload_size = code_size
        .checked_add(data_size)
        .ok_or(Error::InvalidPayload)?;
    let package_size = HEADER_SIZE
        .checked_add(payload_size)
        .ok_or(Error::InvalidPayload)?;
    if package.len() != package_size {
        return Err(Error::InvalidPayload);
    }
    let code_end = HEADER_SIZE
        .checked_add(code_size)
        .ok_or(Error::InvalidPayload)?;
    let code = &package[HEADER_SIZE..code_end];
    let initialized_data = &package[code_end..package_size];
    if checksum_parts(code, initialized_data) != header.crc32 {
        return Err(Error::CrcMismatch);
    }
    let entry_address = header
        .code_load_address
        .checked_add(header.execution_offset)
        .ok_or(Error::AddressOverflow)?;
    let data_init_end = header
        .data_load_address
        .checked_add(header.data_init_size)
        .ok_or(Error::AddressOverflow)?;
    let psp_stack_bottom = data_init_end
        .checked_add(header.data_zero_size)
        .ok_or(Error::AddressOverflow)?;
    let psp_stack_top = psp_stack_bottom
        .checked_add(header.stack_size)
        .ok_or(Error::AddressOverflow)?;
    Ok(Package {
        header,
        code,
        initialized_data,
        entry_address,
        psp_stack_bottom,
        psp_stack_top,
    })
}

/// Decodes and validates the AMRN v2 header without reading file segments.
pub fn parse_header(bytes: &[u8], contract: Contract) -> Result<Header, Error> {
    if bytes.len() < HEADER_SIZE {
        return Err(Error::TruncatedHeader);
    }
    let valid = bytes[..MAGIC.len()] == MAGIC
        && bytes[4] == FORMAT_VERSION
        && bytes[5] == contract.target_id
        && read_u16(bytes, 6) == HEADER_SIZE as u16
        && bytes[ABI_VERSION_OFFSET] == ABI_VERSION
        && bytes[FLAGS_OFFSET] == FLAGS
        && read_u16(bytes, RESERVED_U16_OFFSET) == 0
        && read_u32(bytes, RESERVED_U32_OFFSET) == 0
        && bytes[RESERVED_BYTES_OFFSET..HEADER_SIZE]
            .iter()
            .all(|byte| *byte == 0);
    if !valid {
        return Err(Error::InvalidHeader);
    }
    let header = Header {
        target_id: bytes[5],
        code_size: read_u32(bytes, CODE_SIZE_OFFSET),
        data_init_size: read_u32(bytes, DATA_INIT_SIZE_OFFSET),
        data_zero_size: read_u32(bytes, DATA_ZERO_SIZE_OFFSET),
        stack_size: read_u32(bytes, STACK_SIZE_OFFSET),
        code_load_address: read_u32(bytes, CODE_LOAD_ADDRESS_OFFSET),
        data_load_address: read_u32(bytes, DATA_LOAD_ADDRESS_OFFSET),
        execution_offset: read_u32(bytes, EXECUTION_OFFSET_OFFSET),
        crc32: read_u32(bytes, CRC32_OFFSET),
    };
    validate_header(header, contract)?;
    Ok(header)
}

/// Encodes an AMRN v2 package into caller-provided storage.
pub fn encode(image: Image<'_>, contract: Contract, output: &mut [u8]) -> Result<usize, Error> {
    let code_size = u32::try_from(image.code.len()).map_err(|_| Error::RegionOverflow)?;
    let data_init_size =
        u32::try_from(image.initialized_data.len()).map_err(|_| Error::RegionOverflow)?;
    let header = Header {
        target_id: contract.target_id,
        code_size,
        data_init_size,
        data_zero_size: image.data_zero_size,
        stack_size: image.stack_size,
        code_load_address: contract.code_load_address,
        data_load_address: contract.data_load_address,
        execution_offset: image.execution_offset,
        crc32: checksum_parts(image.code, image.initialized_data),
    };
    validate_header(header, contract)?;
    let payload_size = image
        .code
        .len()
        .checked_add(image.initialized_data.len())
        .ok_or(Error::OutputTooSmall)?;
    let package_size = HEADER_SIZE
        .checked_add(payload_size)
        .ok_or(Error::OutputTooSmall)?;
    if output.len() < package_size {
        return Err(Error::OutputTooSmall);
    }
    write_header(output, header);
    let code_end = HEADER_SIZE + image.code.len();
    output[HEADER_SIZE..code_end].copy_from_slice(image.code);
    output[code_end..package_size].copy_from_slice(image.initialized_data);
    Ok(package_size)
}

fn validate_header(header: Header, contract: Contract) -> Result<(), Error> {
    if header.code_load_address != contract.code_load_address
        || header.data_load_address != contract.data_load_address
        || header.code_size == 0
        || header.stack_size == 0
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
    if header.execution_offset >= header.code_size
        || !header.execution_offset.is_multiple_of(WORD_ALIGNMENT)
    {
        return Err(Error::InvalidExecutionOffset);
    }
    Ok(())
}

fn write_header(output: &mut [u8], header: Header) {
    output[..MAGIC.len()].copy_from_slice(&MAGIC);
    output[4] = FORMAT_VERSION;
    output[5] = header.target_id;
    output[6..8].copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
    write_u32(output, CODE_SIZE_OFFSET, header.code_size);
    write_u32(output, DATA_INIT_SIZE_OFFSET, header.data_init_size);
    write_u32(output, DATA_ZERO_SIZE_OFFSET, header.data_zero_size);
    write_u32(output, STACK_SIZE_OFFSET, header.stack_size);
    write_u32(output, CODE_LOAD_ADDRESS_OFFSET, header.code_load_address);
    write_u32(output, DATA_LOAD_ADDRESS_OFFSET, header.data_load_address);
    write_u32(output, EXECUTION_OFFSET_OFFSET, header.execution_offset);
    write_u32(output, CRC32_OFFSET, header.crc32);
    output[ABI_VERSION_OFFSET] = ABI_VERSION;
    output[FLAGS_OFFSET] = FLAGS;
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + core::mem::size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
#[path = "v2_tests.rs"]
mod tests;
