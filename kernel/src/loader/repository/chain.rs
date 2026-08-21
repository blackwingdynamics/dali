//! Bounded Binary Metadata v2 role-chain primitives.

use core::mem::MaybeUninit;
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

/// Maximum number of repository packages handed to the bounded execution pipeline.
pub const MAX_BINARY_REPOSITORY_PACKAGES: usize = 4;

/// Authentication result retained for one streamed metadata document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StreamedRoleInfo {
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
    /// Root policy retained after target-provisioned anchor validation.
    pub(crate) root: MaybeUninit<RootMetadata>,
    /// Snapshot metadata retained after its timestamp reference is checked.
    pub(crate) snapshot: MaybeUninit<dali_metadata::SnapshotMetadata>,
    /// Revocation metadata retained for package authorization checks.
    pub(crate) revocations: MaybeUninit<dali_metadata::RevocationMetadata>,
    /// Delegation metadata reused for each selected package.
    pub(crate) delegation: MaybeUninit<dali_metadata::DelegationMetadata>,
}

impl BinaryRepositoryBuffers {
    /// Creates zeroed storage for the streaming chain.
    pub const fn new() -> Self {
        Self {
            chunk: [0; streaming::STREAMING_METADATA_CHUNK_BYTES],
            amrn: amrn::AmrnStreamBuffers::new(),
            root: MaybeUninit::uninit(),
            snapshot: MaybeUninit::uninit(),
            revocations: MaybeUninit::uninit(),
            delegation: MaybeUninit::uninit(),
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
}

/// Bounded authorization set for one repository verification pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryRepositoryAuthorizations {
    entries: [Option<BinaryRepositoryAuthorization>; MAX_BINARY_REPOSITORY_PACKAGES],
    length: usize,
}

impl BinaryRepositoryAuthorizations {
    /// Creates an empty authorization set.
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_BINARY_REPOSITORY_PACKAGES],
            length: 0,
        }
    }

    /// Returns the number of authorized packages.
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Returns whether no package was authorized.
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Iterates over authorized packages in Targets document order.
    pub fn iter(&self) -> impl Iterator<Item = BinaryRepositoryAuthorization> + '_ {
        self.entries[..self.length]
            .iter()
            .filter_map(Option::as_ref)
            .copied()
    }

    fn push(&mut self, authorization: BinaryRepositoryAuthorization) -> Result<(), ()> {
        let Some(entry) = self.entries.get_mut(self.length) else {
            return Err(());
        };
        *entry = Some(authorization);
        self.length += 1;
        Ok(())
    }
}

impl Default for BinaryRepositoryAuthorizations {
    fn default() -> Self {
        Self::new()
    }
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
/// Revocation -> AMRN for all bounded matching packages.
pub fn load_binary_repository<S>(
    storage: &mut S,
    request: RepositoryLoadRequest,
    anchors: &[dali_targets::TrustAnchorProfile],
    buffers: &mut BinaryRepositoryBuffers,
) -> Result<BinaryRepositoryAuthorizations, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let Some(contract) = request.contract else {
        return Err(BinaryRepositoryError::Package);
    };
    load_binary_repository_with_contract(
        storage,
        request,
        anchors,
        buffers,
        |_| Some(contract),
        no_repository_progress,
    )
}

fn no_repository_progress() -> bool {
    true
}

/// Verifies a repository and resolves the AMRN memory contract after target selection.
pub fn load_binary_repository_with_contract<S, F>(
    storage: &mut S,
    request: RepositoryLoadRequest,
    anchors: &[dali_targets::TrustAnchorProfile],
    buffers: &mut BinaryRepositoryBuffers,
    mut contract_for: F,
    progress: fn() -> bool,
) -> Result<BinaryRepositoryAuthorizations, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
    F: FnMut(dali_metadata::TargetPackage) -> Option<dali_amrn::v3::Contract>,
{
    stream_verified_root(
        storage,
        anchors,
        &mut buffers.chunk,
        &mut buffers.root,
        progress,
    )
    .map_err(map_root_error)?;
    // SAFETY: stream_verified_root writes the root before returning and this
    // workspace is exclusively borrowed for the duration of this load.
    let root = unsafe { buffers.root.assume_init_ref() };
    verify_timestamp_and_snapshot(
        storage,
        root,
        &mut buffers.chunk,
        &mut buffers.snapshot,
        progress,
    )?;
    // SAFETY: the preceding helper writes the snapshot before this reference
    // is used, and the workspace remains exclusively borrowed by this load.
    let snapshot = unsafe { buffers.snapshot.assume_init_ref() };
    let targets_role = role(root, MetadataRole::Targets)?;
    let targets = if let Some(package_id) = request.package_id {
        streaming::select_verified_binary_targets_for_package::<S, MAX_BINARY_REPOSITORY_PACKAGES>(
            storage,
            package_id,
            targets_role,
            &root.keys[..usize::from(root.key_count)],
            &mut buffers.chunk,
            progress,
        )
    } else {
        streaming::select_verified_binary_targets::<S, MAX_BINARY_REPOSITORY_PACKAGES>(
            storage,
            request.target_profile,
            targets_role,
            &root.keys[..usize::from(root.key_count)],
            &mut buffers.chunk,
            progress,
        )
    }
    .map_err(|_| BinaryRepositoryError::Package)?;
    let first_target = targets
        .iter()
        .next()
        .ok_or(BinaryRepositoryError::MissingRecord)?;
    if !same_reference(
        snapshot.targets.version,
        snapshot.targets.length,
        snapshot.targets.sha256,
        first_target.version,
        first_target.length,
        first_target.digest,
    ) {
        return Err(BinaryRepositoryError::ReferenceMismatch);
    }
    verify_revocations(
        storage,
        root,
        snapshot,
        &mut buffers.chunk,
        &mut buffers.revocations,
        progress,
    )?;
    // SAFETY: the preceding helper writes the revocation metadata before this
    // reference is used, and the workspace remains exclusively borrowed here.
    let revocations = unsafe { buffers.revocations.assume_init_ref() };
    let mut authorizations = BinaryRepositoryAuthorizations::new();
    for selected in targets.iter() {
        let contract = contract_for(selected.target).ok_or(BinaryRepositoryError::Package)?;
        verify_delegation_and_package(
            &mut PackageVerificationContext {
                storage,
                root,
                snapshot,
                revocations,
                chunk: &mut buffers.chunk,
                amrn_buffers: &mut buffers.amrn,
                delegation_output: &mut buffers.delegation,
                progress,
            },
            selected.target,
            selected.version,
            contract,
        )?;
        authorizations
            .push(BinaryRepositoryAuthorization {
                target: selected.target,
            })
            .map_err(|_| BinaryRepositoryError::Package)?;
    }
    Ok(authorizations)
}

#[inline(never)]
fn verify_timestamp_and_snapshot<S>(
    storage: &mut S,
    root: &RootMetadata,
    chunk: &mut [u8],
    output: &mut MaybeUninit<dali_metadata::SnapshotMetadata>,
    progress: fn() -> bool,
) -> Result<(), BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let mut timestamp_output = MaybeUninit::<dali_metadata::TimestampMetadata>::uninit();
    let _timestamp_info = verify_role_from_root(
        storage,
        RepositoryDocument::Timestamp,
        MetadataRole::Timestamp,
        BinaryTimestampBodyStreamParser::new(),
        root,
        &mut timestamp_output,
        RoleVerificationInput { chunk, progress },
    )?;
    // SAFETY: verify_role_from_root writes the timestamp before returning.
    let timestamp_metadata = unsafe { timestamp_output.assume_init_ref() };
    let snapshot = verify_role_from_root(
        storage,
        RepositoryDocument::Snapshot,
        MetadataRole::Snapshot,
        BinarySnapshotBodyStreamParser::new(),
        root,
        output,
        RoleVerificationInput { chunk, progress },
    )?;
    // SAFETY: verify_role_from_root writes the snapshot before returning.
    let snapshot_metadata = unsafe { output.assume_init_ref() };
    if !same_reference(
        timestamp_metadata.snapshot_version,
        timestamp_metadata.snapshot_length,
        timestamp_metadata.snapshot_sha256,
        snapshot_metadata.header.version,
        snapshot.length,
        snapshot.digest,
    ) {
        return Err(BinaryRepositoryError::ReferenceMismatch);
    }
    Ok(())
}

#[inline(never)]
fn verify_revocations<S>(
    storage: &mut S,
    root: &RootMetadata,
    snapshot: &dali_metadata::SnapshotMetadata,
    chunk: &mut [u8],
    output: &mut MaybeUninit<dali_metadata::RevocationMetadata>,
    progress: fn() -> bool,
) -> Result<(), BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let revocations = verify_role_from_root(
        storage,
        RepositoryDocument::Revocations,
        MetadataRole::Revocation,
        BinaryRevocationBodyStreamParser::new(),
        root,
        output,
        RoleVerificationInput { chunk, progress },
    )?;
    // SAFETY: verify_role_from_root writes the revocations before returning.
    let revocation_metadata = unsafe { output.assume_init_ref() };
    if !same_reference(
        snapshot.revocations.version,
        snapshot.revocations.length,
        snapshot.revocations.sha256,
        revocation_metadata.header.version,
        revocations.length,
        revocations.digest,
    ) {
        return Err(BinaryRepositoryError::ReferenceMismatch);
    }
    Ok(())
}

#[inline(never)]
fn verify_delegation_and_package<S>(
    context: &mut PackageVerificationContext<'_, S>,
    target: dali_metadata::TargetPackage,
    target_version: u64,
    contract: dali_amrn::v3::Contract,
) -> Result<(), BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let delegation_id = target
        .delegation_id
        .as_str()
        .ok_or(BinaryRepositoryError::MissingRecord)?;
    let delegation_reference = context
        .snapshot
        .delegations
        .iter()
        .take(usize::from(context.snapshot.delegation_count))
        .find(|reference| reference.id.as_str() == Some(delegation_id))
        .copied()
        .ok_or(BinaryRepositoryError::MissingRecord)?;
    let delegation_info = verify_role_from_root(
        context.storage,
        RepositoryDocument::Delegation(delegation_id),
        MetadataRole::Delegation,
        BinaryDelegationBodyStreamParser::new(),
        context.root,
        context.delegation_output,
        RoleVerificationInput {
            chunk: context.chunk,
            progress: context.progress,
        },
    )?;
    // SAFETY: verify_role_from_root writes the delegation before returning.
    let delegation = unsafe { context.delegation_output.assume_init_ref() };
    if !same_reference(
        delegation_reference.version,
        delegation_reference.length,
        delegation_reference.sha256,
        delegation.header.version,
        delegation_info.length,
        delegation_info.digest,
    ) {
        return Err(BinaryRepositoryError::ReferenceMismatch);
    }
    validate_target_delegation(target, delegation)?;
    if is_revoked(context.revocations, delegation, target_version) {
        return Err(BinaryRepositoryError::Revoked);
    }
    amrn::verify_streamed_amrn(
        context.storage,
        RepositoryPackageDigest(target.sha256.0),
        delegation,
        contract,
        context.chunk,
        context.amrn_buffers,
    )
    .map(|_| ())
    .map_err(|_| BinaryRepositoryError::Package)
}

struct PackageVerificationContext<'a, S> {
    storage: &'a mut S,
    root: &'a RootMetadata,
    snapshot: &'a dali_metadata::SnapshotMetadata,
    revocations: &'a dali_metadata::RevocationMetadata,
    chunk: &'a mut [u8],
    amrn_buffers: &'a mut amrn::AmrnStreamBuffers,
    delegation_output: &'a mut MaybeUninit<dali_metadata::DelegationMetadata>,
    progress: fn() -> bool,
}

struct RoleVerificationInput<'a> {
    chunk: &'a mut [u8],
    progress: fn() -> bool,
}

#[inline(never)]
fn verify_role_from_root<S, P>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    parser: P,
    root: &RootMetadata,
    output: &mut MaybeUninit<P::Output>,
    input: RoleVerificationInput<'_>,
) -> Result<StreamedRoleInfo, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
    P: BinaryRoleBodyParser,
{
    let policy = role(root, expected_role)?;
    let RoleVerificationInput { chunk, progress } = input;
    let captured = capture_role_into(storage, document, expected_role, chunk, parser, output)
        .map_err(map_role_error)?;
    verify_captured_role(
        storage,
        document,
        expected_role,
        policy,
        &root.keys[..usize::from(root.key_count)],
        captured,
        RoleVerificationInput { chunk, progress },
    )
    .map_err(map_role_error)
}

fn role<E>(
    root: &RootMetadata,
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
    delegation: &dali_metadata::DelegationMetadata,
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
    delegation: &dali_metadata::DelegationMetadata,
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

/// Parses and verifies Root metadata against a target-provisioned anchor.
#[inline(never)]
pub(crate) fn stream_verified_root<S>(
    storage: &mut S,
    anchors: &[dali_targets::TrustAnchorProfile],
    chunk: &mut [u8],
    output: &mut MaybeUninit<RootMetadata>,
    progress: fn() -> bool,
) -> Result<StreamedRoleInfo, StreamedRoleError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let captured = capture_role_into(
        storage,
        RepositoryDocument::Root,
        MetadataRole::Root,
        chunk,
        dali_metadata::BinaryRootBodyStreamParser::new(),
        output,
    )?;
    // SAFETY: capture_role_into writes the parser output before returning.
    let root = unsafe { output.assume_init_ref() };
    if !anchors
        .iter()
        .copied()
        .any(|anchor| super::trust::contains_root_anchor(root, anchor))
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
    verify_captured_role(
        storage,
        RepositoryDocument::Root,
        MetadataRole::Root,
        role,
        &root.keys[..usize::from(root.key_count)],
        captured,
        RoleVerificationInput { chunk, progress },
    )
}

/// Captures typed metadata and its complete serialized digest without verifying it.
#[inline(never)]
pub(crate) fn capture_role_into<S, P>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    chunk: &mut [u8],
    mut parser: P,
    output: &mut MaybeUninit<P::Output>,
) -> Result<StreamedRoleInfo, StreamedRoleError<S::Error>>
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
    output.write(metadata);
    Ok(StreamedRoleInfo {
        envelope,
        length,
        digest: Sha256Digest(digest.finalize()),
    })
}

#[inline(never)]
fn verify_captured_role<S>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    role: RoleDefinition,
    keys: &[RoleKey],
    captured: StreamedRoleInfo,
    input: RoleVerificationInput<'_>,
) -> Result<StreamedRoleInfo, StreamedRoleError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let RoleVerificationInput { chunk, progress } = input;
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
                    verifier
                        .update_with_progress(body, progress)
                        .map_err(|_| DecodeError::InvalidValue)
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
