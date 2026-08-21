//! Bounded Binary Metadata v2 role-chain primitives.

use dali_metadata::{
    BinaryEnvelopeStreamParser, BinaryRoleBodyParser, DecodeError, MetadataRole, RoleDefinition,
    RoleKey, RootMetadata, Sha256Digest, StreamedEnvelope, StreamingRoleVerifier,
};

use crate::storage::repository::{RepositoryDocument, RepositoryStreamStorage};

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
        .any(|anchor| crate::loader::repository::trust::contains_root_anchor(&root, anchor))
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
