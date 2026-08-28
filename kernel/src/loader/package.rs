//! AMRN package validation and application entry support.

use dali_amrn::{HEADER_SIZE, ParseError, PayloadValidator, ValidatedPayload, parse_header};
use dali_sdk::{LOG_OK, LOG_REJECTED, MAX_LOG_MESSAGE_BYTES, ServiceTable};

#[cfg(feature = "abi-current")]
use super::pipeline;
use super::{LoaderError, read_exact};
use crate::{
    drivers::{BLOCK_SIZE, Block, StorageError},
    storage::{self, filesystem::AmrnFile},
};

#[cfg(feature = "abi-current")]
/// Bounded collection of applications loaded by the current ABI pipeline.
pub(crate) type LoadedPackages =
    pipeline::execution::LoadedApplications<{ storage::filesystem::MAX_ROOT_AMRN_FILES }>;

#[cfg(all(feature = "abi-current", not(feature = "repository-loader")))]
/// Loads the current ABI package set from the storage root.
pub(crate) fn load_current_abi<D>(
    device: D,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<LoadedPackages, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let device = crate::drivers::BlockDeviceRef::new(&device);
    let mut versions = [0; storage::filesystem::MAX_ROOT_AMRN_FILES];
    let mut package_count = 0;
    let result = storage::filesystem::with_amrn_files(device, |file| {
        let mut version = [0; dali_amrn::MAGIC.len() + core::mem::size_of::<u8>()];
        read_exact(&file, &mut version).map_err(LoaderError::Filesystem)?;
        if let Some(version_slot) = versions.get_mut(package_count) {
            *version_slot = version[4];
            package_count += 1;
            Ok(())
        } else {
            Err(LoaderError::CurrentAbiPackage(
                dali_amrn::v2::Error::InvalidHeader,
            ))
        }
    });
    result.map_err(LoaderError::Filesystem)??;

    match package_count {
        0 => Err(LoaderError::Filesystem(embedded_sdmmc::Error::NotFound)),
        1 => storage::filesystem::with_amrn_file(device, |file| {
            load_current_abi_file(file, slot_manager)
        })
        .map_err(LoaderError::Filesystem)?,
        _ => {
            #[cfg(feature = "abi-relocation")]
            if versions[..package_count]
                .iter()
                .all(|version| *version == dali_amrn::v4::FORMAT_VERSION)
            {
                return pipeline::discovery::load_files(&device, slot_manager);
            }
            Err(LoaderError::Filesystem(embedded_sdmmc::Error::Unsupported))
        }
    }
}

#[cfg(all(feature = "abi-current", not(feature = "repository-loader")))]
/// Loads one package using its format-specific parser.
fn load_current_abi_file<D>(
    file: storage::filesystem::AmrnFile<'_, D>,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<LoadedPackages, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut version = [0; dali_amrn::MAGIC.len() + core::mem::size_of::<u8>()];
    read_exact(&file, &mut version).map_err(LoaderError::Filesystem)?;
    file.rewind().map_err(LoaderError::Filesystem)?;
    match version[4] {
        dali_amrn::v2::FORMAT_VERSION => pipeline::execution::load_file(file, slot_manager)
            .map(pipeline::execution::LoadedApplications::single),
        #[cfg(feature = "abi-relocation")]
        dali_amrn::v3::FORMAT_VERSION => pipeline::relocation::load_file(file, slot_manager)
            .map(pipeline::execution::LoadedApplications::single),
        #[cfg(feature = "abi-relocation")]
        dali_amrn::v4::FORMAT_VERSION => pipeline::identity::load_file(file, slot_manager)
            .map(pipeline::execution::LoadedApplications::single),
        #[cfg(feature = "abi-authentication")]
        dali_amrn::v5::FORMAT_VERSION => pipeline::signed::load_file(file, slot_manager)
            .map(pipeline::execution::LoadedApplications::single),
        _ => Err(LoaderError::CurrentAbiPackage(
            dali_amrn::v2::Error::InvalidHeader,
        )),
    }
}

pub fn validate_amrn_file<D>(device: D) -> Result<ValidatedPayload, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    storage::filesystem::with_amrn_file(device, |file| validate_file(&file))
        .map_err(LoaderError::Filesystem)?
}

pub fn load_amrn_file<D>(device: D) -> Result<ValidatedPayload, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    storage::filesystem::with_amrn_file(device, load_file).map_err(LoaderError::Filesystem)?
}

pub fn start_application(payload: ValidatedPayload) -> ! {
    let entry_address = payload.entry_address | 1;
    let entry: unsafe extern "C" fn(*const ServiceTable) -> ! = unsafe {
        // SAFETY: The parser checked the target, load range, entry offset, and alignment;
        // the loader copied the complete CRC-validated payload to that exact SRAM region.
        core::mem::transmute(entry_address as usize)
    };
    unsafe {
        // SAFETY: The AMRN ABI requires a non-returning C entry point, and application
        // interrupts remain kernel-controlled for this MVP.
        entry(&APPLICATION_SERVICES)
    }
}

/// Stores the shared `APPLICATION_SERVICES` state used by this subsystem.
static APPLICATION_SERVICES: ServiceTable = ServiceTable {
    log: application_log,
};

/// Performs the `application_log` operation for this subsystem.
unsafe extern "C" fn application_log(message: *const u8, length: usize) -> u32 {
    let start = message as usize;
    let end = match start.checked_add(length) {
        Some(end) => end,
        None => return LOG_REJECTED,
    };
    let application_start = dali_amrn::LOAD_ADDRESS as usize;
    let application_end = match application_start.checked_add(dali_amrn::MAX_PAYLOAD_SIZE) {
        Some(end) => end,
        None => return LOG_REJECTED,
    };
    if length > MAX_LOG_MESSAGE_BYTES
        || start < application_start
        || end > application_end
        || message.is_null()
    {
        return LOG_REJECTED;
    }
    let bytes = unsafe {
        // SAFETY: The application ABI restricts the pointer and length to the
        // validated native payload region before this slice is created.
        core::slice::from_raw_parts(message, length)
    };
    let Ok(message) = core::str::from_utf8(bytes) else {
        return LOG_REJECTED;
    };
    crate::logging::info(
        crate::logging::APPLICATION_SUBSYSTEM,
        format_args!("{}", message),
    );
    LOG_OK
}

/// Performs the `validate_file` operation for this subsystem.
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

/// Performs the `load_file` operation for this subsystem.
fn load_file<D>(file: AmrnFile<'_, D>) -> Result<ValidatedPayload, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let validated = validate_file(&file)?;
    copy_payload(&file, validated.header.payload_size as usize)?;
    Ok(validated)
}

/// Performs the `copy_payload` operation for this subsystem.
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
