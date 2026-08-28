//! Binary repository boot loading and error mapping.

use super::cartridge::LoadedCartridges;
use super::{LoaderError, pipeline, repository};
use crate::storage;

#[cfg(feature = "repository-loader")]
#[unsafe(link_section = ".repository_workspace")]
/// Dedicated workspace for the bounded binary repository loader.
static mut BINARY_REPOSITORY_BUFFERS: repository::BinaryRepositoryBuffers =
    repository::BinaryRepositoryBuffers::new();

#[cfg(feature = "repository-loader")]
/// Provides exclusive access to the boot-time repository workspace.
fn with_binary_repository_buffers<R>(
    operation: impl FnOnce(&mut repository::BinaryRepositoryBuffers) -> R,
) -> R {
    // SAFETY: boot storage initialization is single-threaded and completes
    // before application contexts or scheduler interrupts can re-enter this loader.
    unsafe { operation(&mut *core::ptr::addr_of_mut!(BINARY_REPOSITORY_BUFFERS)) }
}

#[cfg(feature = "repository-loader")]
/// Loads and verifies the repository-selected cartridge cartridge.
pub(crate) fn load_repository_cartridge<D>(
    device: D,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
    committed_generation: Option<crate::storage::durable::coordinator::DurableGeneration>,
) -> Result<LoadedCartridges, LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = crate::drivers::StorageError>,
{
    let device = crate::drivers::BlockDeviceRef::new(&device);
    let target = crate::platform::target_profile().ok_or(LoaderError::CurrentAbiCartridge(
        dali_amrn::v2::Error::InvalidHeader,
    ))?;
    let target_profile = dali_metadata::BoundedText::new(target.name)
        .map_err(|_| LoaderError::CurrentAbiCartridge(dali_amrn::v2::Error::InvalidHeader))?;
    let committed_generation = committed_generation
        .map(|generation| {
            dali_metadata::TrustStoreRecord::new(
                generation.version,
                dali_metadata::Sha256Digest(generation.digest),
            )
            .ok_or(repository::BinaryRepositoryError::SecurityState)
        })
        .transpose()
        .map_err(map_repository_error)?;
    let request = repository::RepositoryLoadRequest {
        cartridge_id: None,
        target_profile,
        contract: None,
        now: None,
        committed_generation,
        generation_admission: repository::RepositoryGenerationAdmission::ActiveBoot,
    };
    let mut storage = crate::storage::filesystem::FatRepositoryStorage::new_with_format_and_pet(
        device,
        crate::storage::filesystem::RepositoryMetadataFormat::BinaryV2,
        repository_verification_progress,
    );
    let authorizations = with_binary_repository_buffers(|buffers| {
        repository::load_binary_repository_with_contract(
            &mut storage,
            request,
            crate::platform::trust_anchors(),
            buffers,
            |target| {
                let isolation = target.memory.isolation?;
                let slot = isolation
                    .slots
                    .iter()
                    .copied()
                    .find(|slot| slot.id == target.slot_id)?;
                Some(dali_amrn::v3::Contract {
                    target_id: target.amrn_target_id,
                    code_load_address: slot.code_origin,
                    code_capacity: slot.code_length,
                    data_load_address: slot.data_origin,
                    data_capacity: slot.data_length,
                })
            },
            repository_verification_progress,
        )
    })
    .map_err(map_repository_error)?;
    if authorizations.committed_generation.is_some() {
        crate::logging::info(
            crate::logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY] Committed generation bound to repository load"),
        );
    }
    if authorizations.security_state.is_some() {
        crate::logging::info(
            crate::logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY] Reconstructed trust-store state bound to repository load"),
        );
    }
    let mut loaded = LoadedCartridges::new();
    for authorization in authorizations.iter() {
        let applications = storage::filesystem::with_content_addressed_cartridge(
            device,
            crate::storage::repository::RepositoryCartridgeDigest(authorization.target.sha256.0),
            |file| {
                pipeline::signed::load_file_with_public_key(
                    file,
                    slot_manager,
                    &authorization.developer_public_key,
                )
                .map(LoadedCartridges::single)
            },
        )
        .map_err(LoaderError::Filesystem)??;
        for application in applications.iter().copied() {
            if !loaded.push(application) {
                return Err(LoaderError::CurrentAbiCartridge(
                    dali_amrn::v2::Error::InvalidHeader,
                ));
            }
        }
    }
    Ok(loaded)
}

/// Keeps repository verification progressing when no interrupt-owned service
/// is available during the bounded boot pass.
#[cfg(feature = "repository-loader")]
fn repository_verification_progress() -> bool {
    true
}

#[cfg(feature = "repository-loader")]
/// Maps repository verification failures into loader failures and diagnostics.
fn map_repository_error(
    error: repository::BinaryRepositoryError<embedded_sdmmc::Error<crate::drivers::StorageError>>,
) -> LoaderError {
    crate::logging::error(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!(
            "[LOADER] Binary v2 repository verification failed: {}\r\n",
            repository_error_label(&error)
        ),
    );
    if matches!(
        &error,
        repository::BinaryRepositoryError::BundleGenerationRollback
    ) {
        crate::logging::error(
            crate::logging::SECURITY_SUBSYSTEM,
            format_args!(
                "[SECURITY] Rejection: cartridge generation older than committed generation\r\n"
            ),
        );
    }
    match error {
        repository::BinaryRepositoryError::Storage(error)
        | repository::BinaryRepositoryError::RoleStorage(error)
        | repository::BinaryRepositoryError::BundleRoleStorage(error)
        | repository::BinaryRepositoryError::CartridgeStorage(error) => {
            LoaderError::Filesystem(error)
        }
        repository::BinaryRepositoryError::Revoked
        | repository::BinaryRepositoryError::CartridgeSignature => {
            LoaderError::V5SignedCartridge(dali_amrn::v5::Error::InvalidSignature)
        }
        repository::BinaryRepositoryError::CartridgeCrc => {
            LoaderError::V5SignedCartridge(dali_amrn::v5::Error::CrcMismatch)
        }
        repository::BinaryRepositoryError::BundleGenerationRollback
        | repository::BinaryRepositoryError::BundleGenerationAhead => {
            LoaderError::V5SignedCartridge(dali_amrn::v5::Error::InvalidHeader)
        }
        repository::BinaryRepositoryError::CartridgeLengthMismatch
        | repository::BinaryRepositoryError::CartridgeContract
        | repository::BinaryRepositoryError::CartridgeDigestMismatch => {
            LoaderError::V5SignedCartridge(dali_amrn::v5::Error::InvalidHeader)
        }
        repository::BinaryRepositoryError::CartridgeInvalidHeader(error) => {
            LoaderError::V5SignedCartridge(error)
        }
        repository::BinaryRepositoryError::MissingRecord
        | repository::BinaryRepositoryError::SecurityState
        | repository::BinaryRepositoryError::TargetsParse(_)
        | repository::BinaryRepositoryError::TargetsReferenceMismatch
        | repository::BinaryRepositoryError::SnapshotReferenceMismatch
        | repository::BinaryRepositoryError::RevocationReferenceMismatch
        | repository::BinaryRepositoryError::DelegationReferenceMismatch
        | repository::BinaryRepositoryError::DelegationMismatch
        | repository::BinaryRepositoryError::BundleRoleDecode
        | repository::BinaryRepositoryError::BundleRoleSignature
        | repository::BinaryRepositoryError::BundleTargetMismatch
        | repository::BinaryRepositoryError::UnknownTrustAnchor
        | repository::BinaryRepositoryError::RoleDecode
        | repository::BinaryRepositoryError::RoleSignature => {
            LoaderError::V5SignedCartridge(dali_amrn::v5::Error::InvalidHeader)
        }
    }
}

#[cfg(feature = "repository-loader")]
/// Returns the stable diagnostic label for a repository verification error.
fn repository_error_label<E>(error: &repository::BinaryRepositoryError<E>) -> &'static str {
    match error {
        repository::BinaryRepositoryError::Storage(_) => "storage",
        repository::BinaryRepositoryError::RoleStorage(_) => "role-storage",
        repository::BinaryRepositoryError::BundleRoleStorage(_) => "bundle-role-storage",
        repository::BinaryRepositoryError::MissingRecord => "missing-record",
        repository::BinaryRepositoryError::TargetsParse(error) => match error {
            repository::streaming::StreamingTargetsError::InvalidEnvelope => "targets-envelope",
            repository::streaming::StreamingTargetsError::UnexpectedEnd => "targets-unexpected-end",
            repository::streaming::StreamingTargetsError::RecordTooLarge => {
                "targets-record-too-large"
            }
            repository::streaming::StreamingTargetsError::InvalidRecord => "targets-invalid-record",
            repository::streaming::StreamingTargetsError::TrailingBytes => "targets-trailing-bytes",
            repository::streaming::StreamingTargetsError::TooManyMatchingRecords => {
                "targets-too-many-records"
            }
            repository::streaming::StreamingTargetsError::MultipleMatchingRecords => {
                "targets-multiple-records"
            }
        },
        repository::BinaryRepositoryError::TargetsReferenceMismatch => "targets-reference-mismatch",
        repository::BinaryRepositoryError::SnapshotReferenceMismatch => {
            "snapshot-reference-mismatch"
        }
        repository::BinaryRepositoryError::RevocationReferenceMismatch => {
            "revocation-reference-mismatch"
        }
        repository::BinaryRepositoryError::DelegationReferenceMismatch => {
            "delegation-reference-mismatch"
        }
        repository::BinaryRepositoryError::DelegationMismatch => "delegation-mismatch",
        repository::BinaryRepositoryError::UnknownTrustAnchor => "unknown-trust-anchor",
        repository::BinaryRepositoryError::RoleDecode => "role-decode",
        repository::BinaryRepositoryError::RoleSignature => "role-signature",
        repository::BinaryRepositoryError::BundleRoleDecode => "bundle-role-decode",
        repository::BinaryRepositoryError::BundleRoleSignature => "bundle-role-signature",
        repository::BinaryRepositoryError::BundleTargetMismatch => "bundle-target-mismatch",
        repository::BinaryRepositoryError::BundleGenerationRollback => "bundle-rollback",
        repository::BinaryRepositoryError::BundleGenerationAhead => "bundle-ahead",
        repository::BinaryRepositoryError::Revoked => "revoked",
        repository::BinaryRepositoryError::SecurityState => "security-state",
        repository::BinaryRepositoryError::CartridgeStorage(_) => "cartridge-storage",
        repository::BinaryRepositoryError::CartridgeLengthMismatch => "cartridge-length-mismatch",
        repository::BinaryRepositoryError::CartridgeContract => "cartridge-contract",
        repository::BinaryRepositoryError::CartridgeInvalidHeader(error) => match error {
            dali_amrn::v5::Error::InvalidSignature => "cartridge-signature-envelope",
            dali_amrn::v5::Error::InvalidContract => "cartridge-contract",
            dali_amrn::v5::Error::InvalidHeader => "cartridge-static-header",
            dali_amrn::v5::Error::InvalidHeaderPrefix => "cartridge-header-prefix",
            dali_amrn::v5::Error::InvalidHeaderReserved => "cartridge-header-reserved",
            dali_amrn::v5::Error::InvalidHeaderEncoding => "cartridge-header-encoding",
            dali_amrn::v5::Error::InvalidPayload => "cartridge-payload-layout",
            dali_amrn::v5::Error::InvalidRelocation => "cartridge-relocation",
            dali_amrn::v5::Error::InvalidIdentity => "cartridge-identity",
            dali_amrn::v5::Error::CrcMismatch => "cartridge-crc",
            dali_amrn::v5::Error::TruncatedHeader => "cartridge-truncated-header",
            dali_amrn::v5::Error::OutputTooSmall => "cartridge-output-size",
        },
        repository::BinaryRepositoryError::CartridgeDigestMismatch => "cartridge-digest-mismatch",
        repository::BinaryRepositoryError::CartridgeSignature => "cartridge-signature",
        repository::BinaryRepositoryError::CartridgeCrc => "cartridge-crc",
    }
}
