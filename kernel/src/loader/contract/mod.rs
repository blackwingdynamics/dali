//! Hardware-neutral streaming contracts shared by the kernel loader and tests.

use dali_amrn::{Crc32, v3, v4};
use dali_targets::IsolationSlot;

mod catalog;

#[cfg(test)]
mod tests;

pub use catalog::{CatalogError, DiscoveredPackage, PackageCatalog};

/// A bounded package reader used by the streaming validation core.
pub trait PackageReader {
    /// The storage error returned by the reader.
    type Error;

    /// Reads the next portion of the package.
    fn read(&self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

    /// Rewinds the package to its beginning.
    fn rewind(&self) -> Result<(), Self::Error>;
}

/// Errors returned while validating the v4 stream.
#[derive(Debug, Eq, PartialEq)]
pub enum StreamError<E> {
    /// The reader could not provide the requested bytes.
    Read(E),
    /// The package contains an invalid relocation entry or target range.
    InvalidRelocation,
    /// The payload or package checksum does not match the header.
    CrcMismatch,
}

/// Selects the manifest slot named by a validated v4 header.
pub fn select_slot(
    bytes: &[u8; v4::HEADER_SIZE],
    target_id: u8,
    slots: &[IsolationSlot],
) -> Result<(v4::Header, v3::Contract, IsolationSlot), SlotSelectionError> {
    let mut last_error = SlotSelectionError::InvalidHeader;
    for slot in slots.iter().copied() {
        let contract = v3::Contract {
            target_id,
            code_load_address: slot.code_origin,
            code_capacity: slot.code_length,
            data_load_address: slot.data_origin,
            data_capacity: slot.data_length,
        };
        match v4::parse_header(bytes, contract) {
            Ok(header) if header.metadata.slot_id == slot.id => {
                return Ok((header, contract, slot));
            }
            Ok(_) => last_error = SlotSelectionError::UndeclaredSlot,
            Err(_) => last_error = SlotSelectionError::InvalidHeader,
        }
    }
    Err(last_error)
}

/// Validates v4 payload and package checksums from a bounded reader.
pub fn validate_stream<R>(
    reader: &R,
    header: v4::Header,
    contract: v3::Contract,
) -> Result<(), StreamError<R::Error>>
where
    R: PackageReader,
{
    reader.rewind().map_err(StreamError::Read)?;
    let mut header_bytes = [0; v4::HEADER_SIZE];
    read_exact(reader, &mut header_bytes)?;
    let mut payload_checksum = Crc32::new();
    let mut package_checksum = Crc32::new();
    package_checksum.update(&header_bytes[v4::RESERVED_BYTES_OFFSET..]);
    let mut chunk = [0; crate::drivers::BLOCK_SIZE];
    read_checksum_bytes(
        reader,
        header.image.code_size,
        &mut chunk,
        &mut payload_checksum,
        &mut package_checksum,
    )?;
    read_checksum_bytes(
        reader,
        header.image.data_init_size,
        &mut chunk,
        &mut payload_checksum,
        &mut package_checksum,
    )?;
    let mut entry = [0; v4::RELOCATION_ENTRY_SIZE];
    for _ in 0..header.image.relocation_count {
        read_exact(reader, &mut entry)?;
        payload_checksum.update(&entry);
        package_checksum.update(&entry);
        let relocation =
            v3::decode_relocation(&entry).map_err(|_| StreamError::InvalidRelocation)?;
        v3::validate_relocation(header.image, contract, relocation)
            .map_err(|_| StreamError::InvalidRelocation)?;
    }
    package_checksum.update(&header_bytes[..v4::PACKAGE_CRC32_OFFSET]);
    if payload_checksum.finish() != header.image.crc32
        || package_checksum.finish() != header.package_crc32
    {
        return Err(StreamError::CrcMismatch);
    }
    Ok(())
}

fn read_checksum_bytes<R>(
    reader: &R,
    size: u32,
    chunk: &mut [u8],
    payload_checksum: &mut Crc32,
    package_checksum: &mut Crc32,
) -> Result<(), StreamError<R::Error>>
where
    R: PackageReader,
{
    let mut remaining = usize::try_from(size).map_err(|_| StreamError::CrcMismatch)?;
    while remaining > 0 {
        let chunk_size = remaining.min(chunk.len());
        read_exact(reader, &mut chunk[..chunk_size])?;
        payload_checksum.update(&chunk[..chunk_size]);
        package_checksum.update(&chunk[..chunk_size]);
        remaining -= chunk_size;
    }
    Ok(())
}

fn read_exact<R>(reader: &R, buffer: &mut [u8]) -> Result<(), StreamError<R::Error>>
where
    R: PackageReader,
{
    let mut offset = 0;
    while offset < buffer.len() {
        let read = reader
            .read(&mut buffer[offset..])
            .map_err(StreamError::Read)?;
        if read == 0 || read > buffer.len() - offset {
            return Err(StreamError::CrcMismatch);
        }
        offset += read;
    }
    Ok(())
}

/// Errors returned when a package cannot be matched to a declared slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotSelectionError {
    /// No declared slot accepted the target and image contract.
    InvalidHeader,
    /// The header is valid but names no declared slot.
    UndeclaredSlot,
}
