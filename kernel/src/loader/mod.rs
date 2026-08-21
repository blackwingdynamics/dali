//! AMRN package validation owned by the kernel loader boundary.

use dali::{LOG_OK, LOG_REJECTED, MAX_LOG_MESSAGE_BYTES, ServiceTable};
use dali_amrn::{HEADER_SIZE, ParseError, PayloadValidator, ValidatedPayload, parse_header};

use crate::{
    drivers::{BLOCK_SIZE, Block, StorageError},
    storage::{self, filesystem::AmrnFile},
};

mod pipeline;

#[cfg(feature = "repository-loader")]
pub mod repository;

/// Errors reported while validating a root AMRN package.
#[derive(Debug)]
pub enum LoaderError {
    /// The read-only filesystem could not provide the package stream.
    Filesystem(embedded_sdmmc::Error<StorageError>),
    /// The package header or payload failed AMRN validation.
    Package(ParseError),
    /// The ABI v3 package failed target or segment validation.
    #[cfg(feature = "abi-current")]
    CurrentAbiPackage(dali_amrn::v2::Error),
    /// The selected target does not declare an ABI v3 memory contract.
    #[cfg(feature = "abi-current")]
    UnsupportedCurrentAbiTarget,
    /// The kernel could not reserve the package's manifest-declared slot.
    #[cfg(feature = "abi-current")]
    SlotManager(crate::runtime::memory::slots::SlotManagerError),
    /// The application lifecycle could not record the loaded slot ownership.
    #[cfg(feature = "abi-current")]
    Lifecycle(crate::runtime::application::lifecycle::LifecycleError),
    /// The package failed the identity and slot catalog contract.
    #[cfg(feature = "abi-relocation")]
    PackageCatalog(crate::loader_contract::CatalogError),
    /// The relocatable ABI v3 package failed format validation or patching.
    #[cfg(feature = "abi-relocation")]
    V3RelocationPackage(dali_amrn::v3::Error),
    /// The identity-aware ABI v3 package failed format validation or patching.
    #[cfg(feature = "abi-relocation")]
    V4IdentityPackage(dali_amrn::v4::Error),
    /// The signed v5 package failed format validation or authentication.
    #[cfg(feature = "abi-authentication")]
    V5SignedPackage(dali_amrn::v5::Error),
    /// The signed package selected no provisioned target trust anchor.
    #[cfg(feature = "abi-authentication")]
    UnknownTrustAnchor,
    /// The signed package failed cryptographic verification.
    #[cfg(feature = "abi-authentication")]
    SignatureVerification(dali_crypto::VerificationError),
    /// The package requested a service not exposed by the current kernel.
    #[cfg(feature = "abi-relocation")]
    UnsupportedServices(u32),
}

#[cfg(feature = "abi-current")]
type LoadedPackages =
    pipeline::execution::LoadedApplications<{ storage::filesystem::MAX_ROOT_AMRN_FILES }>;

#[cfg(all(feature = "abi-current", not(feature = "repository-loader")))]
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
    })
    .map_err(LoaderError::Filesystem)?;
    result?;

    match package_count {
        0 => Err(LoaderError::Filesystem(embedded_sdmmc::Error::NotFound)),
        1 => storage::filesystem::with_amrn_file(device, |file| {
            load_current_abi_file(file, slot_manager)
        })
        .map_err(LoaderError::Filesystem)?,
        _ => {
            #[cfg(feature = "abi-relocation")]
            {
                if versions[..package_count]
                    .iter()
                    .all(|version| *version == dali_amrn::v4::FORMAT_VERSION)
                {
                    return pipeline::discovery::load_files(&device, slot_manager);
                }
            }
            Err(LoaderError::Filesystem(embedded_sdmmc::Error::Unsupported))
        }
    }
}

#[cfg(all(feature = "abi-current", feature = "repository-loader"))]
pub(crate) fn load_repository_package<D>(
    device: D,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<LoadedPackages, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let device = crate::drivers::BlockDeviceRef::new(&device);
    let target_profile = dali_metadata::BoundedText::new(crate::platform::TARGET_PROFILE.name)
        .map_err(|_| LoaderError::CurrentAbiPackage(dali_amrn::v2::Error::InvalidHeader))?;
    let request = repository::RepositoryLoadRequest {
        package_id: None,
        target_profile,
        contract: None,
        now: None,
    };
    let mut storage = crate::storage::filesystem::FatRepositoryStorage::new_with_format(
        device,
        crate::storage::filesystem::RepositoryMetadataFormat::BinaryV2,
    );
    let mut buffers = repository::BinaryRepositoryBuffers::new();
    let authorizations = repository::load_binary_repository_with_contract(
        &mut storage,
        request,
        crate::platform::TRUST_ANCHORS,
        &mut buffers,
        |target| {
            let isolation = crate::platform::TARGET_PROFILE.memory.isolation?;
            let slot = isolation
                .slots
                .iter()
                .copied()
                .find(|slot| slot.id == target.slot_id)?;
            Some(dali_amrn::v3::Contract {
                target_id: crate::platform::TARGET_PROFILE.amrn_target_id,
                code_load_address: slot.code_origin,
                code_capacity: slot.code_length,
                data_load_address: slot.data_origin,
                data_capacity: slot.data_length,
            })
        },
    )
    .map_err(map_repository_error)?;
    let mut loaded = LoadedPackages::new();
    for authorization in authorizations.iter() {
        let applications = storage::filesystem::with_content_addressed_package(
            device,
            crate::storage::repository::RepositoryPackageDigest(authorization.target.sha256.0),
            |file| load_current_abi_file(file, slot_manager),
        )
        .map_err(LoaderError::Filesystem)??;
        for application in applications.iter().copied() {
            if !loaded.push(application) {
                return Err(LoaderError::CurrentAbiPackage(
                    dali_amrn::v2::Error::InvalidHeader,
                ));
            }
        }
    }
    Ok(loaded)
}

#[cfg(all(feature = "abi-current", feature = "repository-loader"))]
fn map_repository_error(
    error: repository::BinaryRepositoryError<embedded_sdmmc::Error<StorageError>>,
) -> LoaderError {
    match error {
        repository::BinaryRepositoryError::Storage(error)
        | repository::BinaryRepositoryError::RoleStorage(error) => LoaderError::Filesystem(error),
        repository::BinaryRepositoryError::Revoked | repository::BinaryRepositoryError::Package => {
            LoaderError::V5SignedPackage(dali_amrn::v5::Error::InvalidSignature)
        }
        repository::BinaryRepositoryError::MissingRecord
        | repository::BinaryRepositoryError::ReferenceMismatch
        | repository::BinaryRepositoryError::DelegationMismatch
        | repository::BinaryRepositoryError::UnknownTrustAnchor
        | repository::BinaryRepositoryError::RoleDecode
        | repository::BinaryRepositoryError::RoleSignature => {
            LoaderError::V5SignedPackage(dali_amrn::v5::Error::InvalidHeader)
        }
    }
}

#[cfg(feature = "abi-current")]
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

static APPLICATION_SERVICES: ServiceTable = ServiceTable {
    log: application_log,
};

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
