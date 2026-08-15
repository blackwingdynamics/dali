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
    storage::filesystem::with_amrn_file(device, |file| validate_file(&file))
        .map_err(LoaderError::Filesystem)?
}

/// Validates one root package and copies its payload into the reserved SRAM.
pub fn load_amrn_file<D>(device: D) -> Result<ValidatedPayload, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    storage::filesystem::with_amrn_file(device, load_file).map_err(LoaderError::Filesystem)?
}

/// Transfers control to a previously validated and loaded native application.
pub fn start_application(payload: ValidatedPayload) -> ! {
    let entry_address = payload.entry_address | 1;
    let entry: unsafe extern "C" fn() -> ! = unsafe {
        // SAFETY: The parser checked the target, load range, entry offset, and alignment;
        // the loader copied the complete CRC-validated payload to that exact SRAM region.
        core::mem::transmute(entry_address as usize)
    };
    unsafe {
        // SAFETY: The AMRN ABI requires a non-returning C entry point, and application
        // interrupts remain kernel-controlled for this MVP.
        entry()
    }
}

fn validate_file<D>(file: &AmrnFile<'_, D>) -> Result<ValidatedPayload, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut header_bytes = [0; HEADER_SIZE];
    read_exact(file, &mut header_bytes).map_err(LoaderError::Filesystem)?;
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
        read_exact(file, &mut chunk[..chunk_size]).map_err(LoaderError::Filesystem)?;
        validator
            .update(&chunk[..chunk_size])
            .map_err(LoaderError::Package)?;
        remaining -= chunk_size;
    }
    validator.finish().map_err(LoaderError::Package)
}

fn load_file<D>(file: AmrnFile<'_, D>) -> Result<ValidatedPayload, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let validated = validate_file(&file)?;
    copy_payload(&file, validated.header.payload_size as usize)?;
    Ok(validated)
}

fn copy_payload<D>(file: &AmrnFile<'_, D>, payload_size: usize) -> Result<(), LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    file.rewind().map_err(LoaderError::Filesystem)?;
    let mut header = [0; HEADER_SIZE];
    read_exact(file, &mut header).map_err(LoaderError::Filesystem)?;
    let destination = dali_amrn::LOAD_ADDRESS as *mut u8;
    let mut offset = 0;
    let mut chunk = [0; BLOCK_SIZE];
    while offset < payload_size {
        let chunk_size = (payload_size - offset).min(chunk.len());
        read_exact(file, &mut chunk[..chunk_size]).map_err(LoaderError::Filesystem)?;
        let target = unsafe {
            // SAFETY: AMRN validation bounds payload_size to the reserved application SRAM
            // region, and offset is advanced only within that validated size.
            core::slice::from_raw_parts_mut(destination.add(offset), chunk_size)
        };
        target.copy_from_slice(&chunk[..chunk_size]);
        offset += chunk_size;
    }
    Ok(())
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
