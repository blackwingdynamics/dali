//! Bounded Binary Metadata v2 role-chain primitives.

use dali_metadata::{
    BinaryDelegationBodyStreamParser, BinaryEnvelopeStreamParser, BinaryRevocationBodyStreamParser,
    BinaryRoleBodyParser, BinarySnapshotBodyStreamParser, BinaryTimestampBodyStreamParser,
    DecodeError, MetadataRole, RoleDefinition, RoleKey, RootMetadata, Sha256Digest,
    StreamedEnvelope, StreamingRoleVerifier,
};

use super::{RepositoryLoadRequest, amrn, streaming};
use crate::storage::repository::{
    RepositoryDocument, RepositoryPackageDigest, RepositoryStreamStorage,
};

/// Result retained for one streamed metadata document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StreamedRole<M> {
    /// Typed role body parsed from the bounded stream.
    pub metadata: M,
    /// Envelope authentication metadata collected during the first pass.
    pub envelope: StreamedEnvelope,
    /// Exact serialized document length reported by storage.
    pub length: u32,
    /// SHA-256 digest of the complete serialized document.
    pub digest: Sha256Digest,
}

/// Errors returned by bounded metadata streaming.
#[derive(Debug)]
pub(crate) enum StreamedRoleError<E> {
    /// The repository adapter failed while producing a chunk.
    Storage(E),
    /// The envelope or typed body was malformed.
    Decode,
    /// The storage-reported length did not match delivered bytes.
    LengthMismatch,
    /// The role signature threshold was not met.
    Signature,
    /// No target-provisioned anchor was declared by Root metadata.
    UnknownTrustAnchor,
}

/// Caller-owned buffers for one Binary v2 repository chain pass.
pub struct BinaryRepositoryBuffers {
    /// Shared bounded metadata and package transport chunk.
    chunk: [u8; streaming::STREAMING_METADATA_CHUNK_BYTES],
    /// Fixed AMRN header and signature trailer retained between passes.
    amrn: amrn::AmrnStreamBuffers,
}

impl BinaryRepositoryBuffers {
    /// Creates zeroed storage for the streaming chain.
    pub const fn new() -> Self {
        Self {
            chunk: [0; streaming::STREAMING_METADATA_CHUNK_BYTES],
            amrn: amrn::AmrnStreamBuffers::new(),
        }
    }
}

impl Default for BinaryRepositoryBuffers {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a complete metadata-to-AMRN streamed authorization pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryRepositoryAuthorization {
    /// Target package record accepted by Targets metadata.
    pub target: dali_metadata::TargetPackage,
    /// Developer delegation accepted by the delegated role.
    pub delegation: dali_metadata::DelegationMetadata,
}

/// Errors returned by the complete streamed repository chain.
#[derive(Debug)]
pub enum BinaryRepositoryError<E> {
    /// Storage failed while producing one document or package.
    Storage(E),
    /// A role document failed bounded parsing or signature verification.
    RoleDecode,
    /// A role stream could not satisfy its signature policy.
    RoleSignature,
    /// A role stream failed in the storage adapter.
    RoleStorage(E),
    /// The root document did not contain a target-provisioned trust anchor.
    UnknownTrustAnchor,
    /// A required role definition or record was absent.
    MissingRecord,
    /// A signed metadata reference did not match the streamed document.
    ReferenceMismatch,
    /// The selected delegation did not authorize the selected package.
    DelegationMismatch,
    /// The selected developer key was revoked.
    Revoked,
    /// The package failed streamed AMRN validation.
    Package,
}

/// Verifies Root -> Timestamp -> Snapshot -> Targets -> Delegation ->
/// Revocation -> AMRN through the board-agnostic streaming contract.
pub fn load_binary_repository<S>(
    storage: &mut S,
    request: RepositoryLoadRequest,
    anchors: &[dali_targets::TrustAnchorProfile],
    buffers: &mut BinaryRepositoryBuffers,
) -> Result<BinaryRepositoryAuthorization, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let Some(contract) = request.contract else {
        return Err(BinaryRepositoryError::Package);
    };
    load_binary_repository_with_contract(storage, request, anchors, buffers, |_| Some(contract))
}

/// Verifies a repository and resolves the AMRN memory contract after target selection.
pub fn load_binary_repository_with_contract<S, F>(
    storage: &mut S,
    request: RepositoryLoadRequest,
    anchors: &[dali_targets::TrustAnchorProfile],
    buffers: &mut BinaryRepositoryBuffers,
    contract_for: F,
) -> Result<BinaryRepositoryAuthorization, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
    F: FnOnce(dali_metadata::TargetPackage) -> Option<dali_amrn::v3::Contract>,
{
    let root =
        stream_verified_root(storage, anchors, &mut buffers.chunk).map_err(map_root_error)?;
    let keys = root.metadata.keys;
    let timestamp = verify_role_from_root(
        storage,
        RepositoryDocument::Timestamp,
        MetadataRole::Timestamp,
        BinaryTimestampBodyStreamParser::new(),
        root.metadata,
        &keys,
        &mut buffers.chunk,
    )?;
    let snapshot = verify_role_from_root(
        storage,
        RepositoryDocument::Snapshot,
        MetadataRole::Snapshot,
        BinarySnapshotBodyStreamParser::new(),
        root.metadata,
        &keys,
        &mut buffers.chunk,
    )?;
    if !same_reference(
        timestamp.metadata.snapshot_version,
        timestamp.metadata.snapshot_length,
        timestamp.metadata.snapshot_sha256,
        snapshot.metadata.header.version,
        snapshot.length,
        snapshot.digest,
    ) {
        return Err(BinaryRepositoryError::ReferenceMismatch);
    }
    let targets_role = role(root.metadata, MetadataRole::Targets)?;
    let targets = streaming::select_unique_verified_binary_target(
        storage,
        request.target_profile,
        targets_role,
        &keys[..usize::from(root.metadata.key_count)],
        &mut buffers.chunk,
    )
    .map_err(|_| BinaryRepositoryError::Package)?;
    if !same_reference(
        snapshot.metadata.targets.version,
        snapshot.metadata.targets.length,
        snapshot.metadata.targets.sha256,
        targets.version,
        targets.length,
        targets.digest,
    ) {
        return Err(BinaryRepositoryError::ReferenceMismatch);
    }
    let revocations = verify_role_from_root(
        storage,
        RepositoryDocument::Revocations,
        MetadataRole::Revocation,
        BinaryRevocationBodyStreamParser::new(),
        root.metadata,
        &keys,
        &mut buffers.chunk,
    )?;
    if !same_reference(
        snapshot.metadata.revocations.version,
        snapshot.metadata.revocations.length,
        snapshot.metadata.revocations.sha256,
        revocations.metadata.header.version,
        revocations.length,
        revocations.digest,
    ) {
        return Err(BinaryRepositoryError::ReferenceMismatch);
    }
    let delegation_id = targets
        .target
        .delegation_id
        .as_str()
        .ok_or(BinaryRepositoryError::MissingRecord)?;
    let delegation_reference = snapshot
        .metadata
        .delegations
        .iter()
        .take(usize::from(snapshot.metadata.delegation_count))
        .find(|reference| reference.id.as_str() == Some(delegation_id))
        .copied()
        .ok_or(BinaryRepositoryError::MissingRecord)?;
    let delegation = verify_role_from_root(
        storage,
        RepositoryDocument::Delegation(delegation_id),
        MetadataRole::Delegation,
        BinaryDelegationBodyStreamParser::new(),
        root.metadata,
        &keys,
        &mut buffers.chunk,
    )?;
    if !same_reference(
        delegation_reference.version,
        delegation_reference.length,
        delegation_reference.sha256,
        delegation.metadata.header.version,
        delegation.length,
        delegation.digest,
    ) {
        return Err(BinaryRepositoryError::ReferenceMismatch);
    }
    validate_target_delegation(targets.target, delegation.metadata)?;
    if is_revoked(&revocations.metadata, delegation.metadata, targets.version) {
        return Err(BinaryRepositoryError::Revoked);
    }
    let contract = contract_for(targets.target).ok_or(BinaryRepositoryError::Package)?;
    amrn::verify_streamed_amrn(
        storage,
        RepositoryPackageDigest(targets.target.sha256.0),
        delegation.metadata,
        contract,
        &mut buffers.chunk,
        &mut buffers.amrn,
    )
    .map_err(|_| BinaryRepositoryError::Package)?;
    Ok(BinaryRepositoryAuthorization {
        target: targets.target,
        delegation: delegation.metadata,
    })
}

fn verify_role_from_root<S, P>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    parser: P,
    root: RootMetadata,
    keys: &[RoleKey; dali_metadata::MAX_ROOT_KEYS],
    chunk: &mut [u8],
) -> Result<StreamedRole<P::Output>, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
    P: BinaryRoleBodyParser,
{
    let policy = role(root, expected_role)?;
    stream_verified_role(
        storage,
        document,
        expected_role,
        policy,
        &keys[..usize::from(root.key_count)],
        chunk,
        parser,
    )
    .map_err(map_role_error)
}

fn role<E>(
    root: RootMetadata,
    expected_role: MetadataRole,
) -> Result<RoleDefinition, BinaryRepositoryError<E>> {
    root.roles
        .iter()
        .take(usize::from(root.role_count))
        .find(|definition| definition.role == expected_role)
        .copied()
        .ok_or(BinaryRepositoryError::MissingRecord)
}

fn map_root_error<E>(error: StreamedRoleError<E>) -> BinaryRepositoryError<E> {
    match error {
        StreamedRoleError::UnknownTrustAnchor => BinaryRepositoryError::UnknownTrustAnchor,
        other => map_role_error(other),
    }
}

fn map_role_error<E>(error: StreamedRoleError<E>) -> BinaryRepositoryError<E> {
    match error {
        StreamedRoleError::Storage(error) => BinaryRepositoryError::RoleStorage(error),
        StreamedRoleError::Signature => BinaryRepositoryError::RoleSignature,
        StreamedRoleError::Decode | StreamedRoleError::LengthMismatch => {
            BinaryRepositoryError::RoleDecode
        }
        StreamedRoleError::UnknownTrustAnchor => BinaryRepositoryError::UnknownTrustAnchor,
    }
}

fn same_reference(
    expected_version: u64,
    expected_length: u32,
    expected_digest: Sha256Digest,
    actual_version: u64,
    actual_length: u32,
    actual_digest: Sha256Digest,
) -> bool {
    expected_version == actual_version
        && expected_length == actual_length
        && expected_digest == actual_digest
}

fn validate_target_delegation<E>(
    target: dali_metadata::TargetPackage,
    delegation: dali_metadata::DelegationMetadata,
) -> Result<(), BinaryRepositoryError<E>> {
    let namespace_allowed = delegation.allowed_namespaces
        [..usize::from(delegation.namespace_count)]
        .contains(&target.namespace);
    let target_allowed = delegation.allowed_targets[..usize::from(delegation.target_count)]
        .contains(&target.target_profile);
    let abi_allowed =
        delegation.allowed_abis[..usize::from(delegation.abi_count)].contains(&target.abi_version);
    if target.developer_id != delegation.developer_id
        || target.developer_key_id != delegation.key_id
        || !namespace_allowed
        || !target_allowed
        || !abi_allowed
    {
        return Err(BinaryRepositoryError::DelegationMismatch);
    }
    Ok(())
}

fn is_revoked(
    revocations: &dali_metadata::RevocationMetadata,
    delegation: dali_metadata::DelegationMetadata,
    current_version: u64,
) -> bool {
    revocations
        .records
        .iter()
        .take(usize::from(revocations.record_count))
        .any(|record| {
            record.developer_id == delegation.developer_id
                && record.key_id == delegation.key_id
                && record.effective_version <= current_version
        })
}

/// Parses one role document and verifies it in a second bounded pass.
pub(crate) fn stream_verified_role<S, P>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    role: RoleDefinition,
    keys: &[RoleKey],
    chunk: &mut [u8],
    parser: P,
) -> Result<StreamedRole<P::Output>, StreamedRoleError<S::Error>>
where
    S: RepositoryStreamStorage,
    P: BinaryRoleBodyParser,
{
    let captured = capture_role(storage, document, expected_role, chunk, parser)?;
    verify_captured_role(
        storage,
        document,
        expected_role,
        role,
        keys,
        chunk,
        captured,
    )
}

/// Parses and verifies Root metadata against a target-provisioned anchor.
pub(crate) fn stream_verified_root<S>(
    storage: &mut S,
    anchors: &[dali_targets::TrustAnchorProfile],
    chunk: &mut [u8],
) -> Result<StreamedRole<RootMetadata>, StreamedRoleError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let captured = capture_role(
        storage,
        RepositoryDocument::Root,
        MetadataRole::Root,
        chunk,
        dali_metadata::BinaryRootBodyStreamParser::new(),
    )?;
    let root = captured.metadata;
    if !anchors
        .iter()
        .copied()
        .any(|anchor| super::trust::contains_root_anchor(&root, anchor))
    {
        return Err(StreamedRoleError::UnknownTrustAnchor);
    }
    let role = root
        .roles
        .iter()
        .take(usize::from(root.role_count))
        .find(|definition| definition.role == MetadataRole::Root)
        .copied()
        .ok_or(StreamedRoleError::Decode)?;
    let keys = root.keys;
    verify_captured_role(
        storage,
        RepositoryDocument::Root,
        MetadataRole::Root,
        role,
        &keys[..usize::from(root.key_count)],
        chunk,
        captured,
    )
}

/// Captures typed metadata and its complete serialized digest without verifying it.
pub(crate) fn capture_role<S, P>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    chunk: &mut [u8],
    mut parser: P,
) -> Result<StreamedRole<P::Output>, StreamedRoleError<S::Error>>
where
    S: RepositoryStreamStorage,
    P: BinaryRoleBodyParser,
{
    let mut envelope = BinaryEnvelopeStreamParser::new(expected_role);
    let mut digest = dali_crypto::Sha256Accumulator::new();
    let mut decode_error = false;
    let length = storage
        .stream_metadata(document, chunk, |bytes| {
            digest.update(bytes);
            if decode_error {
                return Ok(());
            }
            if envelope
                .feed(bytes, |body| {
                    parser.feed(body).map_err(|_| DecodeError::InvalidValue)
                })
                .is_err()
            {
                decode_error = true;
            }
            Ok(())
        })
        .map_err(StreamedRoleError::Storage)?;
    if decode_error {
        return Err(StreamedRoleError::Decode);
    }
    let envelope = envelope.finish().map_err(|_| StreamedRoleError::Decode)?;
    let metadata = parser.finish().map_err(|_| StreamedRoleError::Decode)?;
    let delivered = envelope_total_length(envelope);
    if delivered != u64::from(length) {
        return Err(StreamedRoleError::LengthMismatch);
    }
    Ok(StreamedRole {
        metadata,
        envelope,
        length,
        digest: Sha256Digest(digest.finalize()),
    })
}

fn verify_captured_role<S, M>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    role: RoleDefinition,
    keys: &[RoleKey],
    chunk: &mut [u8],
    captured: StreamedRole<M>,
) -> Result<StreamedRole<M>, StreamedRoleError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let mut verifier = StreamingRoleVerifier::new(role, keys, captured.envelope.signatures)
        .map_err(|_| StreamedRoleError::Signature)?;
    let mut replay = BinaryEnvelopeStreamParser::new(expected_role);
    let mut digest = dali_crypto::Sha256Accumulator::new();
    let mut decode_error = false;
    storage
        .stream_metadata(document, chunk, |bytes| {
            if decode_error {
                return Ok(());
            }
            digest.update(bytes);
            if replay
                .feed(bytes, |body| {
                    verifier.update(body);
                    Ok::<(), DecodeError>(())
                })
                .is_err()
            {
                decode_error = true;
            }
            Ok(())
        })
        .map_err(StreamedRoleError::Storage)?;
    let replayed = replay.finish().map_err(|_| StreamedRoleError::Decode)?;
    if decode_error
        || replayed.body_sha256 != captured.envelope.body_sha256
        || Sha256Digest(digest.finalize()) != captured.digest
    {
        return Err(StreamedRoleError::Decode);
    }
    verifier
        .finish()
        .map_err(|_| StreamedRoleError::Signature)?;
    Ok(captured)
}

fn envelope_total_length(envelope: StreamedEnvelope) -> u64 {
    u64::from(envelope.body_length)
        .saturating_add(dali_metadata::BINARY_ENVELOPE_HEADER_BYTES as u64)
        .saturating_add(
            u64::from(envelope.signatures.count)
                .saturating_mul(dali_metadata::BINARY_SIGNATURE_RECORD_BYTES as u64),
        )
}
