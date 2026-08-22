//! Authorized repository installation and durable generation binding.

use super::{
    InstalledRepository, RepositoryBuffers, RepositoryLoadRequest, RepositoryLoaderError,
    load_repository,
};
use crate::storage::{
    durable::{
        DurableStorageAdapter,
        coordinator::{DurableGeneration, PersistenceCoordinator},
        journal::JournalSlot,
    },
    repository::RepositoryStreamStorage,
};

/// Authorization produced by the verified metadata chain for one installation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageInstallationAuthorization {
    /// Targets record authorized by the signed repository metadata.
    pub target: dali_metadata::TargetPackage,
    /// Developer delegation authorized by the signed repository metadata.
    pub delegation: dali_metadata::DelegationMetadata,
    /// Candidate generation bound to the authenticated package digest.
    generation: DurableGeneration,
}

impl PackageInstallationAuthorization {
    /// Binds a verified package to the supplied signed bundle generation.
    pub fn from_verified(
        verified: dali_metadata::VerifiedRepositoryPackage<'_>,
        bundle_version: u64,
    ) -> Self {
        Self {
            target: verified.target,
            delegation: verified.delegation,
            generation: DurableGeneration {
                version: bundle_version,
                digest: verified.target.sha256.0,
                length: verified.target.length,
            },
        }
    }

    fn generation(self) -> DurableGeneration {
        self.generation
    }
}

/// Verifies, authorizes, stages, reads back, and commits one repository.
pub fn install_repository<S>(
    mut storage: S,
    request: RepositoryLoadRequest,
    buffers: &mut RepositoryBuffers,
    bundle_version: u64,
    active_slot: JournalSlot,
    active_generation: DurableGeneration,
    next_sequence: u64,
) -> Result<InstalledRepository, RepositoryLoaderError<<S as RepositoryStreamStorage>::Error>>
where
    S: RepositoryStreamStorage
        + DurableStorageAdapter<Error = <S as RepositoryStreamStorage>::Error>,
{
    let verified = load_repository(&mut storage, request, buffers)?;
    let authorization = PackageInstallationAuthorization::from_verified(verified, bundle_version);
    let package_length = authorization.target.length as usize;
    if package_length > buffers.package.len() {
        return Err(RepositoryLoaderError::ArtifactTooLarge);
    }
    let mut coordinator =
        PersistenceCoordinator::new(storage, active_slot, active_generation, next_sequence);
    coordinator
        .authorize_candidate(authorization.generation())
        .map_err(RepositoryLoaderError::Persistence)?;
    coordinator
        .write_candidate(
            &buffers.package[..package_length],
            authorization.generation(),
        )
        .map_err(RepositoryLoaderError::Persistence)?;
    coordinator
        .verify_candidate(
            authorization.generation(),
            &buffers.package[..package_length],
            &mut buffers.candidate_readback[..package_length],
        )
        .map_err(RepositoryLoaderError::Persistence)?;
    coordinator
        .commit()
        .map_err(RepositoryLoaderError::Persistence)?;
    Ok(InstalledRepository {
        target: authorization.target,
        delegation: authorization.delegation,
    })
}
