//! AMRN format version 4 identity metadata for ABI v3 packages.

use crate::{Crc32, MAGIC, v3};

/// Fixed AMRN v4 header length in bytes.
pub const HEADER_SIZE: usize = 128;
/// AMRN format revision represented by this module.
pub const FORMAT_VERSION: u8 = 4;
/// ABI revision retained by the v4 container contract.
pub const ABI_VERSION: u8 = v3::ABI_VERSION;
/// Encoded relocation entry length retained from format v3.
pub const RELOCATION_ENTRY_SIZE: usize = v3::RELOCATION_ENTRY_SIZE;
/// Offset of the target identifier in the fixed header.
pub const TARGET_ID_OFFSET: usize = 5;
/// Offset of the stable package identity in the fixed header.
pub const PACKAGE_ID_OFFSET: usize = 80;
/// Offset of the manifest-owned slot identifier in the extension header.
pub const SLOT_ID_OFFSET: usize = 112;
/// Offset of the package CRC32 field in the extension header.
pub const PACKAGE_CRC32_OFFSET: usize = 116;
/// Offset where the package CRC32 input resumes after the reserved header tail.
pub const RESERVED_BYTES_OFFSET: usize = 120;
const V3_HEADER_SIZE: usize = v3::HEADER_SIZE;
const CODE_SIZE_OFFSET: usize = 8;
const DATA_INIT_SIZE_OFFSET: usize = 12;
const DATA_ZERO_SIZE_OFFSET: usize = 16;
const STACK_SIZE_OFFSET: usize = 20;
const LINKED_CODE_BASE_OFFSET: usize = 24;
const LINKED_DATA_BASE_OFFSET: usize = 28;
const CODE_LOAD_ADDRESS_OFFSET: usize = 32;
const DATA_LOAD_ADDRESS_OFFSET: usize = 36;
const EXECUTION_OFFSET_OFFSET: usize = 40;
const RELOCATION_OFFSET_OFFSET: usize = 44;
const RELOCATION_COUNT_OFFSET: usize = 48;
const RELOCATION_ENTRY_SIZE_OFFSET: usize = 52;
const PAYLOAD_CRC32_OFFSET: usize = 56;
const ABI_VERSION_OFFSET: usize = 60;
const PACKAGE_VERSION_OFFSET: usize = 96;
const MINIMUM_KERNEL_VERSION_OFFSET: usize = 102;
const REQUIRED_SERVICES_OFFSET: usize = 108;
const FLAGS_OFFSET: usize = 113;
const RESERVED_U16_OFFSET: usize = 114;

/// Semantic version represented by three bounded unsigned components.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Version {
    /// Major compatibility component.
    pub major: u16,
    /// Minor feature component.
    pub minor: u16,
    /// Patch correction component.
    pub patch: u16,
}

/// Identity and compatibility metadata carried by one v4 package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metadata {
    /// Stable opaque application identity.
    pub package_id: [u8; 16],
    /// Application package version.
    pub package_version: Version,
    /// Minimum compatible kernel API version.
    pub minimum_kernel_version: Version,
    /// Required kernel service capability bits.
    pub required_services: u32,
    /// Explicit target-manifest slot identifier.
    pub slot_id: u8,
}

/// Input image and metadata used to construct an AMRN v4 package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Image<'a> {
    /// ABI v3 code, data, and relocation image.
    pub image: v3::Image<'a>,
    /// Identity and compatibility metadata.
    pub metadata: Metadata,
}

/// Decoded v4 header combining the v3 image contract and selection metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    /// v3 image and relocation fields retained by the v4 contract.
    pub image: v3::Header,
    /// Package identity and compatibility metadata.
    pub metadata: Metadata,
    /// Integrity checksum covering the v3 payload and v4 metadata.
    pub package_crc32: u32,
}

/// A validated v4 package view into caller-owned bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Package<'a> {
    /// Validated v4 header.
    pub header: Header,
    /// Code segment bytes.
    pub code: &'a [u8],
    /// Initialized data segment bytes.
    pub initialized_data: &'a [u8],
    relocation_bytes: &'a [u8],
}

impl Package<'_> {
    /// Decodes one retained v3 relocation entry.
    pub fn relocation(&self, index: usize) -> Result<v3::Relocation, Error> {
        let offset = index
            .checked_mul(RELOCATION_ENTRY_SIZE)
            .ok_or(Error::InvalidRelocation)?;
        let bytes = self
            .relocation_bytes
            .get(offset..offset + RELOCATION_ENTRY_SIZE)
            .ok_or(Error::InvalidRelocation)?;
        v3::decode_relocation(bytes).map_err(|_| Error::InvalidRelocation)
    }
}

/// Errors returned by v4 parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The fixed header is incomplete.
    TruncatedHeader,
    /// A header field is incompatible with the v4 contract.
    InvalidHeader,
    /// The package identity is empty.
    InvalidIdentity,
    /// The v3 image or relocation payload is malformed.
    InvalidPayload,
    /// The relocation table is malformed.
    InvalidRelocation,
    /// The package checksum is invalid.
    CrcMismatch,
    /// The output buffer cannot contain the encoded package.
    OutputTooSmall,
}

/// Encodes an ABI v3 relocation image with v4 identity metadata.
pub fn encode(image: Image<'_>, contract: v3::Contract, output: &mut [u8]) -> Result<usize, Error> {
    if image.metadata.package_id.iter().all(|byte| *byte == 0) {
        return Err(Error::InvalidIdentity);
    }
    let payload_size = image
        .image
        .code
        .len()
        .checked_add(image.image.initialized_data.len())
        .and_then(|size| {
            image
                .image
                .relocations
                .len()
                .checked_mul(RELOCATION_ENTRY_SIZE)
                .and_then(|table| size.checked_add(table))
        })
        .ok_or(Error::InvalidPayload)?;
    let v3_size = V3_HEADER_SIZE
        .checked_add(payload_size)
        .ok_or(Error::OutputTooSmall)?;
    let package_size = HEADER_SIZE
        .checked_add(payload_size)
        .ok_or(Error::OutputTooSmall)?;
    if output.len() < package_size {
        return Err(Error::OutputTooSmall);
    }
    let written = v3::encode(image.image, contract, output).map_err(|error| match error {
        v3::Error::OutputTooSmall => Error::OutputTooSmall,
        v3::Error::InvalidRelocation => Error::InvalidRelocation,
        _ => Error::InvalidPayload,
    })?;
    if written != v3_size {
        return Err(Error::InvalidPayload);
    }
    output.copy_within(V3_HEADER_SIZE..v3_size, HEADER_SIZE);
    let image_header =
        v3::parse_header(&output[..V3_HEADER_SIZE], contract).map_err(|_| Error::InvalidHeader)?;
    write_header(output, image_header, image.metadata);
    let package_crc = package_checksum(output);
    write_u32(output, PACKAGE_CRC32_OFFSET, package_crc);
    Ok(package_size)
}

/// Parses and validates a v4 package against a target slot contract.
pub fn parse<'a>(package: &'a [u8], contract: v3::Contract) -> Result<Package<'a>, Error> {
    let header = parse_header(package, contract)?;
    let code_size = usize::try_from(header.image.code_size).map_err(|_| Error::InvalidPayload)?;
    let data_size =
        usize::try_from(header.image.data_init_size).map_err(|_| Error::InvalidPayload)?;
    let table_size = usize::try_from(header.image.relocation_count)
        .ok()
        .and_then(|count| count.checked_mul(RELOCATION_ENTRY_SIZE))
        .ok_or(Error::InvalidRelocation)?;
    let code_end = HEADER_SIZE
        .checked_add(code_size)
        .ok_or(Error::InvalidPayload)?;
    let data_end = code_end
        .checked_add(data_size)
        .ok_or(Error::InvalidPayload)?;
    let package_end = data_end
        .checked_add(table_size)
        .ok_or(Error::InvalidPayload)?;
    if header.image.relocation_offset as usize != data_end || package.len() != package_end {
        return Err(Error::InvalidPayload);
    }
    let code = &package[HEADER_SIZE..code_end];
    let initialized_data = &package[code_end..data_end];
    let relocation_bytes = &package[data_end..package_end];
    if payload_checksum(code, initialized_data, relocation_bytes) != header.image.crc32
        || package_checksum(package) != header.package_crc32
    {
        return Err(Error::CrcMismatch);
    }
    for index in 0..header.image.relocation_count as usize {
        let start = index * RELOCATION_ENTRY_SIZE;
        let relocation =
            v3::decode_relocation(&relocation_bytes[start..start + RELOCATION_ENTRY_SIZE])
                .map_err(|_| Error::InvalidRelocation)?;
        v3::validate_relocation(header.image, contract, relocation)
            .map_err(|_| Error::InvalidRelocation)?;
    }
    Ok(Package {
        header,
        code,
        initialized_data,
        relocation_bytes,
    })
}

/// Decodes and validates a v4 header without reading its payload.
pub fn parse_header(bytes: &[u8], contract: v3::Contract) -> Result<Header, Error> {
    if bytes.len() < HEADER_SIZE {
        return Err(Error::TruncatedHeader);
    }
    if bytes[..MAGIC.len()] != MAGIC
        || bytes[4] != FORMAT_VERSION
        || read_u16(bytes, 6) != HEADER_SIZE as u16
        || bytes[ABI_VERSION_OFFSET] != ABI_VERSION
        || bytes[FLAGS_OFFSET] != 0
        || read_u16(bytes, 54) != 0
        || read_u16(bytes, RESERVED_U16_OFFSET) != 0
        || bytes[64..80].iter().any(|byte| *byte != 0)
        || bytes[RESERVED_BYTES_OFFSET..HEADER_SIZE]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(Error::InvalidHeader);
    }
    let package_id = read_array::<16>(bytes, PACKAGE_ID_OFFSET);
    if package_id.iter().all(|byte| *byte == 0) {
        return Err(Error::InvalidIdentity);
    }
    let image = v3::Header {
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
        crc32: read_u32(bytes, PAYLOAD_CRC32_OFFSET),
    };
    if image.target_id != contract.target_id
        || image.relocation_offset < HEADER_SIZE as u32
        || read_u16(bytes, RELOCATION_ENTRY_SIZE_OFFSET) != RELOCATION_ENTRY_SIZE as u16
    {
        return Err(Error::InvalidHeader);
    }
    v3::validate_header(image, contract).map_err(|_| Error::InvalidHeader)?;
    Ok(Header {
        image,
        metadata: Metadata {
            package_id,
            package_version: read_version(bytes, PACKAGE_VERSION_OFFSET),
            minimum_kernel_version: read_version(bytes, MINIMUM_KERNEL_VERSION_OFFSET),
            required_services: read_u32(bytes, REQUIRED_SERVICES_OFFSET),
            slot_id: bytes[SLOT_ID_OFFSET],
        },
        package_crc32: read_u32(bytes, PACKAGE_CRC32_OFFSET),
    })
}

fn payload_checksum(code: &[u8], data: &[u8], relocations: &[u8]) -> u32 {
    let mut checksum = Crc32::new();
    checksum.update(code);
    checksum.update(data);
    checksum.update(relocations);
    checksum.finish()
}

fn package_checksum(package: &[u8]) -> u32 {
    let mut checksum = Crc32::new();
    checksum.update(&package[RESERVED_BYTES_OFFSET..]);
    checksum.update(&package[..PACKAGE_CRC32_OFFSET]);
    checksum.finish()
}

fn read_version(bytes: &[u8], offset: usize) -> Version {
    Version {
        major: read_u16(bytes, offset),
        minor: read_u16(bytes, offset + 2),
        patch: read_u16(bytes, offset + 4),
    }
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> [u8; N] {
    let mut result = [0; N];
    result.copy_from_slice(&bytes[offset..offset + N]);
    result
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

fn write_header(output: &mut [u8], image: v3::Header, metadata: Metadata) {
    output[..MAGIC.len()].copy_from_slice(&MAGIC);
    output[4] = FORMAT_VERSION;
    output[5] = image.target_id;
    write_u16(output, 6, HEADER_SIZE as u16);
    write_u32(output, CODE_SIZE_OFFSET, image.code_size);
    write_u32(output, DATA_INIT_SIZE_OFFSET, image.data_init_size);
    write_u32(output, DATA_ZERO_SIZE_OFFSET, image.data_zero_size);
    write_u32(output, STACK_SIZE_OFFSET, image.stack_size);
    write_u32(output, LINKED_CODE_BASE_OFFSET, image.linked_code_base);
    write_u32(output, LINKED_DATA_BASE_OFFSET, image.linked_data_base);
    write_u32(output, CODE_LOAD_ADDRESS_OFFSET, image.code_load_address);
    write_u32(output, DATA_LOAD_ADDRESS_OFFSET, image.data_load_address);
    write_u32(output, EXECUTION_OFFSET_OFFSET, image.execution_offset);
    write_u32(
        output,
        RELOCATION_OFFSET_OFFSET,
        image.relocation_offset + (HEADER_SIZE - V3_HEADER_SIZE) as u32,
    );
    write_u32(output, RELOCATION_COUNT_OFFSET, image.relocation_count);
    write_u16(
        output,
        RELOCATION_ENTRY_SIZE_OFFSET,
        RELOCATION_ENTRY_SIZE as u16,
    );
    write_u32(output, PAYLOAD_CRC32_OFFSET, image.crc32);
    output[ABI_VERSION_OFFSET] = ABI_VERSION;
    output[PACKAGE_ID_OFFSET..PACKAGE_ID_OFFSET + metadata.package_id.len()]
        .copy_from_slice(&metadata.package_id);
    write_version(output, PACKAGE_VERSION_OFFSET, metadata.package_version);
    write_version(
        output,
        MINIMUM_KERNEL_VERSION_OFFSET,
        metadata.minimum_kernel_version,
    );
    write_u32(output, REQUIRED_SERVICES_OFFSET, metadata.required_services);
    output[SLOT_ID_OFFSET] = metadata.slot_id;
    output[FLAGS_OFFSET] = 0;
    write_u16(output, RESERVED_U16_OFFSET, 0);
    write_u32(output, PACKAGE_CRC32_OFFSET, 0);
    output[RESERVED_BYTES_OFFSET..HEADER_SIZE].fill(0);
}

fn write_version(bytes: &mut [u8], offset: usize, version: Version) {
    write_u16(bytes, offset, version.major);
    write_u16(bytes, offset + 2, version.minor);
    write_u16(bytes, offset + 4, version.patch);
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
#[cfg(test)]
mod tests;
