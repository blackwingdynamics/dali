#![no_std]

//! Hardware-independent AMRN v1 header and payload validation.

mod builder;
mod stream;
pub mod v2;

pub use builder::{BuildError, encode_package};
#[cfg(test)]
use stream::checksum as crc32;
pub use stream::{PayloadValidator, ValidatedPayload};

/// The fixed AMRN v1 header length in bytes.
pub const HEADER_SIZE: usize = 32;
/// The AMRN format revision implemented by this crate.
pub const FORMAT_VERSION: u8 = 1;
/// The STM32F405RGT6 target identifier defined by the current v1 profile.
pub const TARGET_ID: u8 = 0x02;
/// The ABI revision implemented by the current AMRN profile.
pub const ABI_VERSION: u8 = 2;
/// The reserved flags value accepted by the v1 format.
pub const RESERVED_FLAGS: u8 = 0;
/// The reserved 16-bit field value accepted by the v1 format.
pub const RESERVED_U16: u16 = 0;
/// The reserved 32-bit field value accepted by the v1 format.
pub const RESERVED_U32: u32 = 0;
/// The native payload load address defined by the v1 format.
pub const LOAD_ADDRESS: u32 = 0x2000_8000;
/// The maximum native payload size defined by the v1 format.
pub const MAX_PAYLOAD_SIZE: usize = 64 * 1024;
/// The payload offset field position in the fixed header.
pub const PAYLOAD_OFFSET: usize = HEADER_SIZE;
/// The expected AMRN magic bytes.
pub const MAGIC: [u8; 4] = *b"DALI";

const FORMAT_VERSION_OFFSET: usize = 4;
const TARGET_ID_OFFSET: usize = 5;
const HEADER_SIZE_OFFSET: usize = 6;
const PAYLOAD_SIZE_OFFSET: usize = 8;
const LOAD_ADDRESS_OFFSET: usize = 12;
const EXECUTION_OFFSET_OFFSET: usize = 16;
const CRC32_OFFSET: usize = 20;
const ABI_VERSION_OFFSET: usize = 24;
const FLAGS_OFFSET: usize = 25;
const RESERVED_U16_OFFSET: usize = 26;
const RESERVED_U32_OFFSET: usize = 28;
const WORD_ALIGNMENT: u32 = 4;
const CRC32_POLYNOMIAL: u32 = 0xEDB8_8320;
const CRC32_INITIAL: u32 = u32::MAX;

/// A validated AMRN package view into caller-owned bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Package<'a> {
    /// The validated fixed header fields.
    pub header: Header,
    /// The validated native payload bytes.
    pub payload: &'a [u8],
    /// The calculated native entry address before the Thumb bit is applied.
    pub entry_address: u32,
}

/// The explicitly decoded AMRN v1 fixed header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    /// The package magic bytes.
    pub magic: [u8; 4],
    /// The package format revision.
    pub format_version: u8,
    /// The target identifier.
    pub target_id: u8,
    /// The fixed header length.
    pub header_size: u16,
    /// The native payload length.
    pub payload_size: u32,
    /// The native payload load address.
    pub load_address: u32,
    /// The entry offset from the payload start.
    pub execution_offset: u32,
    /// The CRC32 of the payload bytes.
    pub crc32: u32,
    /// The application ABI revision.
    pub abi_version: u8,
    /// Reserved flags.
    pub flags: u8,
    /// Reserved 16-bit field.
    pub reserved_u16: u16,
    /// Reserved 32-bit field.
    pub reserved_u32: u32,
}

/// Reasons an AMRN package failed validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// The input does not contain the fixed header.
    TruncatedHeader,
    /// The four-byte package magic is invalid.
    InvalidMagic,
    /// The format revision is not supported.
    UnsupportedFormatVersion,
    /// The target identifier is not supported.
    UnsupportedTarget,
    /// The header length is not the fixed v1 length.
    InvalidHeaderSize,
    /// The ABI revision is not supported.
    UnsupportedAbiVersion,
    /// A reserved header field is non-zero.
    NonZeroReservedField,
    /// The payload length is zero.
    EmptyPayload,
    /// The payload exceeds the v1 limit.
    PayloadTooLarge,
    /// The payload extends beyond the supplied package bytes.
    PayloadOutsidePackage,
    /// The load address does not match the v1 contract.
    InvalidLoadAddress,
    /// The execution offset is outside the payload.
    InvalidExecutionOffset,
    /// The execution offset is not word-aligned.
    UnalignedExecutionOffset,
    /// The calculated entry address overflowed.
    EntryAddressOverflow,
    /// The payload CRC32 does not match the header.
    CrcMismatch,
}

/// Parses and validates an AMRN v1 package without copying its payload.
pub fn parse(package: &[u8]) -> Result<Package<'_>, ParseError> {
    if package.len() < HEADER_SIZE {
        return Err(ParseError::TruncatedHeader);
    }
    let header = parse_header(&package[..HEADER_SIZE])?;

    let payload_size = header.payload_size as usize;
    let payload_end = PAYLOAD_OFFSET
        .checked_add(payload_size)
        .ok_or(ParseError::PayloadOutsidePackage)?;
    let payload = package
        .get(PAYLOAD_OFFSET..payload_end)
        .ok_or(ParseError::PayloadOutsidePackage)?;

    validate_payload(header, payload)
}

/// Decodes and validates the fixed AMRN header without reading the payload.
pub fn parse_header(header_bytes: &[u8]) -> Result<Header, ParseError> {
    if header_bytes.len() < HEADER_SIZE {
        return Err(ParseError::TruncatedHeader);
    }
    let header = Header {
        magic: read_magic(header_bytes),
        format_version: header_bytes[FORMAT_VERSION_OFFSET],
        target_id: header_bytes[TARGET_ID_OFFSET],
        header_size: read_u16(header_bytes, HEADER_SIZE_OFFSET),
        payload_size: read_u32(header_bytes, PAYLOAD_SIZE_OFFSET),
        load_address: read_u32(header_bytes, LOAD_ADDRESS_OFFSET),
        execution_offset: read_u32(header_bytes, EXECUTION_OFFSET_OFFSET),
        crc32: read_u32(header_bytes, CRC32_OFFSET),
        abi_version: header_bytes[ABI_VERSION_OFFSET],
        flags: header_bytes[FLAGS_OFFSET],
        reserved_u16: read_u16(header_bytes, RESERVED_U16_OFFSET),
        reserved_u32: read_u32(header_bytes, RESERVED_U32_OFFSET),
    };
    validate_header(header)
}

/// Validates payload bytes against a previously decoded AMRN header.
pub fn validate_payload<'a>(header: Header, payload: &'a [u8]) -> Result<Package<'a>, ParseError> {
    if payload.len() != header.payload_size as usize {
        return Err(ParseError::PayloadOutsidePackage);
    }
    validate_execution_offset(header.execution_offset, payload.len())?;
    let mut validator = PayloadValidator::new(header)?;
    validator.update(payload)?;
    let validated = validator.finish()?;
    Ok(Package {
        header: validated.header,
        payload,
        entry_address: validated.entry_address,
    })
}

fn read_magic(bytes: &[u8]) -> [u8; MAGIC.len()] {
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

fn validate_header(header: Header) -> Result<Header, ParseError> {
    if header.magic != MAGIC {
        return Err(ParseError::InvalidMagic);
    }
    if header.format_version != FORMAT_VERSION {
        return Err(ParseError::UnsupportedFormatVersion);
    }
    if header.target_id != TARGET_ID {
        return Err(ParseError::UnsupportedTarget);
    }
    if usize::from(header.header_size) != HEADER_SIZE {
        return Err(ParseError::InvalidHeaderSize);
    }
    if header.abi_version != ABI_VERSION {
        return Err(ParseError::UnsupportedAbiVersion);
    }
    if header.flags != RESERVED_FLAGS
        || header.reserved_u16 != RESERVED_U16
        || header.reserved_u32 != RESERVED_U32
    {
        return Err(ParseError::NonZeroReservedField);
    }
    if header.payload_size == 0 {
        return Err(ParseError::EmptyPayload);
    }
    if header.payload_size as usize > MAX_PAYLOAD_SIZE {
        return Err(ParseError::PayloadTooLarge);
    }
    if header.load_address != LOAD_ADDRESS {
        return Err(ParseError::InvalidLoadAddress);
    }
    validate_execution_offset(header.execution_offset, header.payload_size as usize)?;
    Ok(header)
}

fn validate_execution_offset(offset: u32, payload_size: usize) -> Result<(), ParseError> {
    if usize::try_from(offset).map_or(true, |value| value >= payload_size) {
        return Err(ParseError::InvalidExecutionOffset);
    }
    if !offset.is_multiple_of(WORD_ALIGNMENT) {
        return Err(ParseError::UnalignedExecutionOffset);
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    // Callers validate the fixed header length before reading any field.
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    // Callers validate the fixed header length before reading any field.
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

#[cfg(test)]
mod tests;
