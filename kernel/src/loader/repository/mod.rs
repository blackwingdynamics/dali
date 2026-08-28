//! Board-agnostic repository loading and durable publication.

mod amrn;
mod chain;
pub mod discovery;
mod installation;
mod io;
mod trust;

use dali_amrn::v3::Contract;
use dali_metadata::{
    MAX_DELEGATION_BYTES, MAX_ENVELOPE_BYTES, PackageId, RepositoryPackageDocuments, TargetPackage,
    VerifiedRepositoryPackage, parse_signed_envelope, parse_targets_signed,
    verify_repository_package,
};

use crate::storage::durable::coordinator::PersistenceError;
use crate::storage::repository::{
    RepositoryDocument, RepositoryPackageDigest, RepositoryStreamStorage,
};

use io::{read_metadata, read_package};

pub use chain::{
    BinaryRepositoryAuthorization, BinaryRepositoryAuthorizations, BinaryRepositoryBuffers,
    BinaryRepositoryError, load_binary_repository, load_binary_repository_with_contract,
};
pub use installation::{PackageInstallationAuthorization, install_repository};

/// Generation rule applied to a signed bundle during repository loading.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryGenerationAdmission {
    /// Load the bundle selected by the durable active-generation record.
    ActiveBoot,
    /// Accept only an explicitly authorized candidate newer than the active record.
    AuthorizedCandidate,
}

/// Caller-owned bounded buffers for one repository verification pass.
pub struct RepositoryBuffers {
    /// Shared chunk used to move bytes from storage into role buffers.
    pub stream_chunk: [u8; streaming::STREAMING_METADATA_CHUNK_BYTES],
    /// Root role envelope.
    pub root: [u8; MAX_ENVELOPE_BYTES],
    /// Timestamp role envelope.
    pub timestamp: [u8; MAX_ENVELOPE_BYTES],
    /// Snapshot role envelope.
    pub snapshot: [u8; MAX_ENVELOPE_BYTES],
    /// Targets role envelope.
    pub targets: [u8; MAX_ENVELOPE_BYTES],
    /// Revocation role envelope.
    pub revocations: [u8; MAX_ENVELOPE_BYTES],
    /// Selected delegation envelope.
    pub delegation: [u8; MAX_DELEGATION_BYTES + 2 * 1024],
    /// Complete AMRN package.
    pub package: [u8; dali_metadata::MAX_BUNDLE_BYTES],
    /// Read-back buffer used before durable commit.
    pub candidate_readback: [u8; dali_metadata::MAX_BUNDLE_BYTES],
}

impl RepositoryBuffers {
    /// Creates zeroed caller-owned storage for one bounded verification pass.
    pub const fn new() -> Self {
        Self {
            stream_chunk: [0; streaming::STREAMING_METADATA_CHUNK_BYTES],
            root: [0; MAX_ENVELOPE_BYTES],
            timestamp: [0; MAX_ENVELOPE_BYTES],
            snapshot: [0; MAX_ENVELOPE_BYTES],
            targets: [0; MAX_ENVELOPE_BYTES],
            revocations: [0; MAX_ENVELOPE_BYTES],
            delegation: [0; MAX_DELEGATION_BYTES + 2 * 1024],
            package: [0; dali_metadata::MAX_BUNDLE_BYTES],
            candidate_readback: [0; dali_metadata::MAX_BUNDLE_BYTES],
        }
    }
}

impl Default for RepositoryBuffers {
    fn default() -> Self {
        Self::new()
    }
}

/// Inputs that are stable for one repository load.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepositoryLoadRequest {
    /// Optional package identity for an explicit host-side selection.
    pub package_id: Option<PackageId>,
    /// Target profile used by boot-time bounded executable discovery.
    pub target_profile: dali_metadata::BoundedText<{ dali_metadata::MAX_TARGET_PROFILE_BYTES }>,
    /// Optional fixed AMRN v5 contract for explicit host-side verification.
    pub contract: Option<Contract>,
    /// Optional trusted wall-clock value for expiry checks.
    pub now: Option<u64>,
    /// Durable trust-store generation selected during boot recovery.
    pub committed_generation: Option<dali_metadata::TrustStoreRecord>,
    /// Context that determines whether equality is an active boot or a candidate.
    pub generation_admission: RepositoryGenerationAdmission,
}

/// Copy-only result returned after a repository was durably published.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstalledRepository {
    /// Package authorization record accepted by the chain.
    pub target: TargetPackage,
    /// Developer delegation accepted by the chain.
    pub delegation: dali_metadata::DelegationMetadata,
}

/// Loader errors preserve storage, parsing, and chain-verification boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryLoaderError<E> {
    /// Repository storage failed to provide a document or package.
    Storage(E),
    /// A repository artifact exceeded its caller-owned buffer.
    ArtifactTooLarge,
    /// The adapter-reported length did not match delivered bytes.
    StorageLengthMismatch,
    /// A signed metadata envelope could not be parsed.
    Decode,
    /// The selected package or its delegation was not present.
    MissingRecord,
    /// The complete metadata-to-AMRN chain rejected the repository.
    Verification(dali_metadata::ChainVerificationError),
    /// Durable publication failed after verification.
    Persistence(PersistenceError<E>),
}

/// Loads and verifies the complete repository chain through streaming storage.
pub fn load_repository<'a, S>(
    storage: &mut S,
    request: RepositoryLoadRequest,
    buffers: &'a mut RepositoryBuffers,
) -> Result<
    VerifiedRepositoryPackage<'a>,
    RepositoryLoaderError<<S as RepositoryStreamStorage>::Error>,
>
where
    S: RepositoryStreamStorage,
{
    let root_length = read_metadata(
        storage,
        RepositoryDocument::Root,
        &mut buffers.root,
        &mut buffers.stream_chunk,
    )?;
    let timestamp_length = read_metadata(
        storage,
        RepositoryDocument::Timestamp,
        &mut buffers.timestamp,
        &mut buffers.stream_chunk,
    )?;
    let snapshot_length = read_metadata(
        storage,
        RepositoryDocument::Snapshot,
        &mut buffers.snapshot,
        &mut buffers.stream_chunk,
    )?;
    let targets_length = read_metadata(
        storage,
        RepositoryDocument::Targets,
        &mut buffers.targets,
        &mut buffers.stream_chunk,
    )?;
    let revocations_length = read_metadata(
        storage,
        RepositoryDocument::Revocations,
        &mut buffers.revocations,
        &mut buffers.stream_chunk,
    )?;

    let targets_envelope = parse_signed_envelope(&buffers.targets[..targets_length])
        .map_err(|_| RepositoryLoaderError::Decode)?;
    let targets =
        parse_targets_signed(targets_envelope.signed).map_err(|_| RepositoryLoaderError::Decode)?;
    let package_id = request
        .package_id
        .ok_or(RepositoryLoaderError::MissingRecord)?;
    let target = find_target(&targets, package_id)?;
    let delegation_id = target
        .delegation_id
        .as_str()
        .ok_or(RepositoryLoaderError::MissingRecord)?;
    let delegation_length = read_metadata(
        storage,
        RepositoryDocument::Delegation(delegation_id),
        &mut buffers.delegation,
        &mut buffers.stream_chunk,
    )?;
    let package_length = read_package(
        storage,
        RepositoryPackageDigest(target.sha256.0),
        &mut buffers.package,
        &mut buffers.stream_chunk,
    )?;

    let documents = RepositoryPackageDocuments {
        root: parse_envelope(&buffers.root[..root_length])?,
        timestamp: parse_envelope(&buffers.timestamp[..timestamp_length])?,
        snapshot: parse_envelope(&buffers.snapshot[..snapshot_length])?,
        targets: targets_envelope,
        revocations: parse_envelope(&buffers.revocations[..revocations_length])?,
        delegation: parse_envelope(&buffers.delegation[..delegation_length])?,
        package: &buffers.package[..package_length],
    };
    let contract = request
        .contract
        .ok_or(RepositoryLoaderError::MissingRecord)?;
    verify_repository_package(documents, package_id, contract, request.now)
        .map_err(RepositoryLoaderError::Verification)
}

/// Internal helper for `parse_envelope`.
fn parse_envelope<'a, E>(
    bytes: &'a [u8],
) -> Result<dali_metadata::SignedEnvelope<'a>, RepositoryLoaderError<E>> {
    parse_signed_envelope(bytes).map_err(|_| RepositoryLoaderError::Decode)
}

/// Internal helper for `find_target`.
fn find_target<E>(
    targets: &dali_metadata::TargetsMetadata,
    package_id: PackageId,
) -> Result<TargetPackage, RepositoryLoaderError<E>> {
    targets
        .packages
        .iter()
        .take(usize::from(targets.package_count))
        .find(|target| target.package_id == package_id)
        .copied()
        .ok_or(RepositoryLoaderError::MissingRecord)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[path = "streaming.rs"]
pub mod streaming;
