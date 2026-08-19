//! Signed AMRN container for the relocatable ABI v3 image contract.

use crate::{Crc32, MAGIC, signature, v3, v4};

/// Fixed v5 header length in bytes.
pub const HEADER_SIZE: usize = 160;
/// Signed AMRN format revision.
pub const FORMAT_VERSION: u8 = 5;
/// Fixed trailer length supplied by the DSIG envelope contract.
pub const SIGNATURE_SIZE: usize = signature::ENVELOPE_SIZE;
/// Offset of the package checksum retained from v4.
pub const PACKAGE_CRC32_OFFSET: usize = 116;
const MAGIC_OFFSET: usize = 0;
const FORMAT_VERSION_OFFSET: usize = 4;
const TARGET_ID_OFFSET: usize = 5;
const HEADER_SIZE_OFFSET: usize = 6;
const V4_RESERVED_START: usize = 64;
const FLAGS_OFFSET: usize = 113;
const RESERVED_U16_OFFSET: usize = 114;
const CRC32_END_OFFSET: usize = PACKAGE_CRC32_OFFSET + core::mem::size_of::<u32>();
const SIGNATURE_OFFSET_FIELD: usize = 128;
const SIGNATURE_SIZE_FIELD: usize = 132;
const SIGNED_SIZE_FIELD: usize = 136;
const RESERVED_START: usize = 140;
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
const PACKAGE_ID_OFFSET: usize = 80;
const PACKAGE_VERSION_OFFSET: usize = 96;
const MINIMUM_KERNEL_VERSION_OFFSET: usize = 102;
const REQUIRED_SERVICES_OFFSET: usize = 108;
const SLOT_ID_OFFSET: usize = 112;

/// Input image and identity metadata used to construct a signed package.
pub type Image<'a> = v4::Image<'a>;

/// Decoded v5 header and its structurally validated signature envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header<'a> {
    /// Relocatable image fields retained by the v4 contract.
    pub image: v3::Header,
    /// Identity and compatibility metadata.
    pub metadata: v4::Metadata,
    /// CRC32 over the signed range, excluding this field during calculation.
    pub package_crc32: u32,
    /// Structurally validated signature envelope.
    pub signature: signature::Envelope<'a>,
}

/// A validated v5 package view into caller-owned bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Package<'a> {
    /// Validated v5 header.
    pub header: Header<'a>,
    /// Code segment bytes.
    pub code: &'a [u8],
    /// Initialized data segment bytes.
    pub initialized_data: &'a [u8],
    relocation_bytes: &'a [u8],
    signed_bytes: &'a [u8],
}

impl Package<'_> {
    /// Returns the exact bytes covered by the package signature.
    pub fn signed_bytes(&self) -> &[u8] {
        self.signed_bytes
    }

    /// Decodes one retained v3 relocation entry.
    pub fn relocation(&self, index: usize) -> Result<v3::Relocation, Error> {
        let offset = index
            .checked_mul(v3::RELOCATION_ENTRY_SIZE)
            .ok_or(Error::InvalidRelocation)?;
        let bytes = self
            .relocation_bytes
            .get(offset..offset + v3::RELOCATION_ENTRY_SIZE)
            .ok_or(Error::InvalidRelocation)?;
        v3::decode_relocation(bytes).map_err(|_| Error::InvalidRelocation)
    }
}

/// Errors returned by the signed package codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The output or input does not contain a complete fixed header.
    TruncatedHeader,
    /// A fixed header or extension field is invalid.
    InvalidHeader,
    /// The v3 image or relocation payload is malformed.
    InvalidPayload,
    /// The relocation table is malformed.
    InvalidRelocation,
    /// The package identity is empty.
    InvalidIdentity,
    /// The package CRC32 does not match.
    CrcMismatch,
    /// The DSIG trailer is malformed.
    InvalidSignature,
    /// The output buffer is too small.
    OutputTooSmall,
}

mod codec;
pub use codec::{append_signature, encode_unsigned, parse};

fn image_payload_size(image: v3::Image<'_>) -> Result<usize, Error> {
    image
        .code
        .len()
        .checked_add(image.initialized_data.len())
        .and_then(|size| {
            image
                .relocations
                .len()
                .checked_mul(v3::RELOCATION_ENTRY_SIZE)
                .and_then(|table| size.checked_add(table))
        })
        .ok_or(Error::OutputTooSmall)
}

fn payload_checksum(code: &[u8], data: &[u8], relocations: &[u8]) -> u32 {
    let mut checksum = Crc32::new();
    checksum.update(code);
    checksum.update(data);
    checksum.update(relocations);
    checksum.finish()
}

fn package_checksum(bytes: &[u8]) -> u32 {
    let mut checksum = Crc32::new();
    checksum.update(&bytes[..PACKAGE_CRC32_OFFSET]);
    checksum.update(&bytes[CRC32_END_OFFSET..]);
    checksum.finish()
}

fn write_header(output: &mut [u8], image: v3::Header, metadata: v4::Metadata, signed_size: usize) {
    output[..HEADER_SIZE].fill(0);
    output[MAGIC_OFFSET..MAGIC_OFFSET + MAGIC.len()].copy_from_slice(&MAGIC);
    output[FORMAT_VERSION_OFFSET] = FORMAT_VERSION;
    output[TARGET_ID_OFFSET] = image.target_id;
    write_u16(output, HEADER_SIZE_OFFSET, HEADER_SIZE as u16);
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
        image.relocation_offset + (HEADER_SIZE - v3::HEADER_SIZE) as u32,
    );
    write_u32(output, RELOCATION_COUNT_OFFSET, image.relocation_count);
    write_u16(
        output,
        RELOCATION_ENTRY_SIZE_OFFSET,
        v3::RELOCATION_ENTRY_SIZE as u16,
    );
    write_u32(output, PAYLOAD_CRC32_OFFSET, image.crc32);
    output[ABI_VERSION_OFFSET] = v3::ABI_VERSION;
    output[PACKAGE_ID_OFFSET..PACKAGE_ID_OFFSET + v4::PACKAGE_ID_LENGTH]
        .copy_from_slice(&metadata.package_id);
    write_version(output, PACKAGE_VERSION_OFFSET, metadata.package_version);
    write_version(
        output,
        MINIMUM_KERNEL_VERSION_OFFSET,
        metadata.minimum_kernel_version,
    );
    write_u32(output, REQUIRED_SERVICES_OFFSET, metadata.required_services);
    output[SLOT_ID_OFFSET] = metadata.slot_id;
    output[CRC32_END_OFFSET..HEADER_SIZE].fill(0);
    write_u32(output, SIGNATURE_OFFSET_FIELD, signed_size as u32);
    write_u16(output, SIGNATURE_SIZE_FIELD, SIGNATURE_SIZE as u16);
    write_u32(output, SIGNED_SIZE_FIELD, signed_size as u32);
    write_u32(output, PACKAGE_CRC32_OFFSET, 0);
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> [u8; N] {
    let mut value = [0; N];
    value.copy_from_slice(&bytes[offset..offset + N]);
    value
}
fn read_version(bytes: &[u8], offset: usize) -> v4::Version {
    v4::Version {
        major: read_u16(bytes, offset),
        minor: read_u16(bytes, offset + 2),
        patch: read_u16(bytes, offset + 4),
    }
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
fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn write_version(bytes: &mut [u8], offset: usize, version: v4::Version) {
    write_u16(bytes, offset, version.major);
    write_u16(bytes, offset + 2, version.minor);
    write_u16(bytes, offset + 4, version.patch);
}

#[cfg(test)]
mod tests;
