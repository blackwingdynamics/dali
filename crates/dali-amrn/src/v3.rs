//! AMRN format version 3 relocation metadata for movable ABI v3 applications.

use crate::{MAGIC, stream::Crc32};

#[path = "v3_codec.rs"]
mod codec;
#[path = "v3_wire.rs"]
mod wire;
pub use codec::{encode, parse};

/// The fixed AMRN v3 header length in bytes.
pub const HEADER_SIZE: usize = 80;
/// The AMRN format revision represented by this module.
pub const FORMAT_VERSION: u8 = 3;
/// The ABI revision represented by this module.
pub const ABI_VERSION: u8 = 3;
/// The encoded relocation entry length in bytes.
pub const RELOCATION_ENTRY_SIZE: usize = 16;
/// The maximum number of relocation entries accepted by the format.
pub const MAX_RELOCATION_ENTRIES: usize = 1024;
const WORD_ALIGNMENT: u32 = 4;
const FLAGS: u8 = 0;
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
const RELOCATION_RESERVED_U16_OFFSET: usize = 54;
const CRC32_OFFSET: usize = 56;
const ABI_VERSION_OFFSET: usize = 60;
const FLAGS_OFFSET: usize = 61;
const RESERVED_U16_OFFSET: usize = 62;
const RESERVED_BYTES_OFFSET: usize = 64;

/// The segment containing a relocation patch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Segment {
    /// The executable code segment.
    Code,
    /// The initialized writable-data segment.
    Data,
}

impl Segment {
    const fn raw(self) -> u8 {
        match self {
            Self::Code => 0,
            Self::Data => 1,
        }
    }

    const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Code),
            1 => Some(Self::Data),
            _ => None,
        }
    }
}

/// Relocation kinds emitted by the first supported ARM fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelocationKind {
    /// A 32-bit absolute address field.
    Abs32,
    /// A Thumb call or jump instruction pair.
    ThmCall,
    /// The lower half of a Thumb absolute address.
    ThmMovwAbsNc,
    /// The upper half of a Thumb absolute address.
    ThmMovtAbs,
}

impl RelocationKind {
    const fn raw(self) -> u8 {
        match self {
            Self::Abs32 => 1,
            Self::ThmCall => 2,
            Self::ThmMovwAbsNc => 3,
            Self::ThmMovtAbs => 4,
        }
    }

    const fn from_raw(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Abs32),
            2 => Some(Self::ThmCall),
            3 => Some(Self::ThmMovwAbsNc),
            4 => Some(Self::ThmMovtAbs),
            _ => None,
        }
    }

    const fn patch_width(self) -> u32 {
        4
    }

    const fn patch_alignment(self) -> u32 {
        match self {
            Self::Abs32 => 4,
            Self::ThmCall | Self::ThmMovwAbsNc | Self::ThmMovtAbs => 2,
        }
    }
}

/// A relocation patch and the linked address it must resolve to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Relocation {
    /// Segment containing the patch location.
    pub segment: Segment,
    /// Architecture relocation operation.
    pub kind: RelocationKind,
    /// Byte offset of the patch within the segment.
    pub patch_offset: u32,
    /// Symbol address from the linked image before slot placement.
    pub linked_target: u32,
    /// Format-defined signed relocation addend.
    pub addend: i32,
}

/// Target load boundaries accepted by a movable package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Contract {
    /// AMRN target identifier.
    pub target_id: u8,
    /// Selected code segment origin.
    pub code_load_address: u32,
    /// Code segment capacity.
    pub code_capacity: u32,
    /// Selected data segment origin.
    pub data_load_address: u32,
    /// Data segment capacity.
    pub data_capacity: u32,
}

/// Relocatable image segments and relocation records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Image<'a> {
    /// Linked code and read-only data bytes.
    pub code: &'a [u8],
    /// Linked initialized writable-data bytes.
    pub initialized_data: &'a [u8],
    /// Runtime zero-initialized data length.
    pub data_zero_size: u32,
    /// Runtime PSP stack reservation.
    pub stack_size: u32,
    /// Linked code origin used when producing relocation records.
    pub linked_code_base: u32,
    /// Linked data origin used when producing relocation records.
    pub linked_data_base: u32,
    /// Word-aligned entry offset from the linked code origin.
    pub execution_offset: u32,
    /// Relocation records retained from the linked image.
    pub relocations: &'a [Relocation],
}

/// The decoded AMRN v3 header.
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
    /// Linked code origin.
    pub linked_code_base: u32,
    /// Linked data origin.
    pub linked_data_base: u32,
    /// Selected code load origin.
    pub code_load_address: u32,
    /// Selected data load origin.
    pub data_load_address: u32,
    /// Entry offset from the linked code origin.
    pub execution_offset: u32,
    /// Absolute package offset of the relocation table.
    pub relocation_offset: u32,
    /// Number of relocation entries.
    pub relocation_count: u32,
    /// CRC32 of code, initialized data, and relocation bytes.
    pub crc32: u32,
}

/// A validated AMRN v3 package view into caller-owned bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Package<'a> {
    /// The validated package header.
    pub header: Header,
    /// The code segment bytes.
    pub code: &'a [u8],
    /// The initialized-data segment bytes.
    pub initialized_data: &'a [u8],
    relocation_bytes: &'a [u8],
}

impl<'a> Package<'a> {
    /// Decodes one validated relocation entry by index.
    pub fn relocation(&self, index: usize) -> Result<Relocation, Error> {
        let offset = index
            .checked_mul(RELOCATION_ENTRY_SIZE)
            .ok_or(Error::InvalidRelocation)?;
        let bytes = self
            .relocation_bytes
            .get(offset..offset + RELOCATION_ENTRY_SIZE)
            .ok_or(Error::InvalidRelocation)?;
        codec::decode_relocation(bytes)
    }
}

/// Errors returned by AMRN v3 parsing and construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The fixed header is incomplete.
    TruncatedHeader,
    /// A header field is incompatible with the contract.
    InvalidHeader,
    /// A segment or runtime reservation exceeds its capacity.
    RegionOverflow,
    /// The package does not contain exactly the declared segments and table.
    InvalidPayload,
    /// The relocation table is malformed or contains an unsupported operation.
    InvalidRelocation,
    /// The entry offset is invalid.
    InvalidExecutionOffset,
    /// A checked address calculation overflowed.
    AddressOverflow,
    /// The package checksum is invalid.
    CrcMismatch,
    /// The output buffer cannot contain the package.
    OutputTooSmall,
}

#[cfg(test)]
#[path = "v3_tests.rs"]
mod tests;
