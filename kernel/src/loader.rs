//! AMRN package validation owned by the kernel loader boundary.

use dali_amrn::{HEADER_SIZE, ParseError, PayloadValidator, ValidatedPayload, parse_header};

use crate::storage::{self, BLOCK_SIZE, Block, StorageError, filesystem::AmrnFile};

/// Errors reported while validating a root AMRN package.
#[derive(Debug)]
pub enum LoaderError {
    /// The read-only filesystem could not provide the package stream.
    Filesystem(embedded_sdmmc::Error<StorageError>),
    /// The package header or payload failed AMRN validation.
    Package(ParseError),
}

/// Reads and validates the single root AMRN package without copying it.
pub fn validate_amrn_file<D>(device: D) -> Result<ValidatedPayload, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    storage::filesystem::with_amrn_file(device, validate_file).map_err(LoaderError::Filesystem)?
}

fn validate_file<D>(file: AmrnFile<'_, D>) -> Result<ValidatedPayload, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut header_bytes = [0; HEADER_SIZE];
    read_exact(&file, &mut header_bytes).map_err(LoaderError::Filesystem)?;
    let header = parse_header(&header_bytes).map_err(LoaderError::Package)?;
    let expected_length = u64::from(HEADER_SIZE as u32)
        .checked_add(u64::from(header.payload_size))
        .ok_or(LoaderError::Package(ParseError::PayloadOutsidePackage))?;
    if u64::from(file.length()) != expected_length {
        return Err(LoaderError::Package(ParseError::PayloadOutsidePackage));
    }

    let mut validator = PayloadValidator::new(header).map_err(LoaderError::Package)?;
    let mut chunk: Block = [0; BLOCK_SIZE];
    let mut remaining = header.payload_size as usize;
    while remaining > 0 {
        let chunk_size = remaining.min(chunk.len());
        read_exact(&file, &mut chunk[..chunk_size]).map_err(LoaderError::Filesystem)?;
        validator
            .update(&chunk[..chunk_size])
            .map_err(LoaderError::Package)?;
        remaining -= chunk_size;
    }
    validator.finish().map_err(LoaderError::Package)
}

fn read_exact<D>(
    file: &AmrnFile<'_, D>,
    buffer: &mut [u8],
) -> Result<(), embedded_sdmmc::Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut offset = 0;
    while offset < buffer.len() {
        let read = file.read(&mut buffer[offset..])?;
        if read == 0 {
            return Err(embedded_sdmmc::Error::EndOfFile);
        }
        offset += read;
    }
    Ok(())
}
