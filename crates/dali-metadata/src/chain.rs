//! End-to-end repository metadata and AMRN verification.

use dali_amrn::{v3, v4, v5};
use sha2::{Digest, Sha256};

use crate::{
    BoundedText, CartridgeAuthorizationError, CartridgeId, DelegationMetadata, Ed25519Verifier,
    MetadataRole, RevocationMetadata, RootMetadata, Sha256Digest, SignatureVerifier,
    SignedEnvelope, SnapshotMetadata, TargetCartridge, TargetsMetadata, TimestampMetadata,
    verify_role_signatures,
};

/// All signed documents needed to verify one cartridge artifact.
pub struct RepositoryCartridgeDocuments<'a> {
    /// Self-signed repository root.
    pub root: SignedEnvelope<'a>,
    /// Root-authorized timestamp document.
    pub timestamp: SignedEnvelope<'a>,
    /// Root-authorized snapshot document.
    pub snapshot: SignedEnvelope<'a>,
    /// Root-authorized targets document.
    pub targets: SignedEnvelope<'a>,
    /// Root-authorized revocation document.
    pub revocations: SignedEnvelope<'a>,
    /// Root-authorized developer delegation selected by the target record.
    pub delegation: SignedEnvelope<'a>,
    /// Complete AMRN cartridge bytes, including its DSIG trailer.
    pub cartridge: &'a [u8],
}

/// Result of a successful repository-to-AMRN verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedRepositoryCartridge<'a> {
    /// Parsed targets record that authorized the cartridge.
    pub target: TargetCartridge,
    /// Parsed developer delegation that authorized the cartridge.
    pub delegation: DelegationMetadata,
    /// Parsed AMRN cartridge borrowing the caller's bytes.
    pub amrn: v5::Cartridge<'a>,
}

/// Errors returned by the complete verification chain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainVerificationError {
    /// A signed body could not be decoded into its role model.
    Decode,
    /// One metadata role signature set failed threshold verification.
    Signature,
    /// A metadata reference did not match the supplied signed bytes.
    ReferenceMismatch,
    /// A requested cartridge or delegation was absent.
    MissingRecord,
    /// A cartridge was outside its developer delegation or was revoked.
    Authorization,
    /// The AMRN cartridge failed structural validation.
    InvalidAmrn,
    /// The AMRN cartridge signature failed verification.
    InvalidAmrnSignature,
    /// An AMRN header field disagreed with the target record.
    CartridgeRecordMismatch,
    /// A signed document was expired at the supplied clock value.
    Expired,
}

/// Verifies the complete chain with the real Ed25519 facade.
pub fn verify_repository_cartridge(
    documents: RepositoryCartridgeDocuments<'_>,
    cartridge_id: CartridgeId,
    contract: v3::Contract,
    now: Option<u64>,
) -> Result<VerifiedRepositoryCartridge<'_>, ChainVerificationError> {
    verify_repository_cartridge_with(&Ed25519Verifier, documents, cartridge_id, contract, now)
}

/// Verifies the complete chain with an injected signature backend.
pub fn verify_repository_cartridge_with<'a, V: SignatureVerifier>(
    verifier: &V,
    documents: RepositoryCartridgeDocuments<'a>,
    cartridge_id: CartridgeId,
    contract: v3::Contract,
    now: Option<u64>,
) -> Result<VerifiedRepositoryCartridge<'a>, ChainVerificationError> {
    let root = decode_root(documents.root.signed)?;
    let timestamp = decode_timestamp(documents.timestamp.signed)?;
    let snapshot = decode_snapshot(documents.snapshot.signed)?;
    let targets = decode_targets(documents.targets.signed)?;
    let revocations = decode_revocations(documents.revocations.signed)?;
    let delegation = decode_delegation(documents.delegation.signed)?;

    verify_role_document(
        verifier,
        &root,
        MetadataRole::Root,
        documents.root,
        root.header.expires,
        now,
    )?;
    verify_role_document(
        verifier,
        &root,
        MetadataRole::Timestamp,
        documents.timestamp,
        timestamp.header.expires,
        now,
    )?;
    verify_role_document(
        verifier,
        &root,
        MetadataRole::Snapshot,
        documents.snapshot,
        snapshot.header.expires,
        now,
    )?;
    verify_role_document(
        verifier,
        &root,
        MetadataRole::Targets,
        documents.targets,
        targets.header.expires,
        now,
    )?;
    verify_role_document(
        verifier,
        &root,
        MetadataRole::Revocation,
        documents.revocations,
        revocations.header.expires,
        now,
    )?;
    verify_role_document(
        verifier,
        &root,
        MetadataRole::Delegation,
        documents.delegation,
        delegation.header.expires,
        now,
    )?;

    check_reference(
        timestamp.snapshot_version,
        timestamp.snapshot_length,
        timestamp.snapshot_sha256,
        documents.snapshot.signed,
        snapshot.header.version,
    )?;
    check_reference(
        snapshot.targets.version,
        snapshot.targets.length,
        snapshot.targets.sha256,
        documents.targets.signed,
        targets.header.version,
    )?;
    check_reference(
        snapshot.revocations.version,
        snapshot.revocations.length,
        snapshot.revocations.sha256,
        documents.revocations.signed,
        revocations.header.version,
    )?;

    let target = find_target(&targets, cartridge_id)?;
    let delegation_id = target.delegation_id;
    let reference = snapshot
        .delegations
        .iter()
        .take(usize::from(snapshot.delegation_count))
        .find(|candidate| candidate.id == delegation_id)
        .ok_or(ChainVerificationError::MissingRecord)?;
    check_reference(
        reference.version,
        reference.length,
        reference.sha256,
        documents.delegation.signed,
        delegation.header.version,
    )?;

    crate::authorize_target_cartridge_with_revocations(
        &target,
        &delegation_id,
        &delegation,
        &revocations,
        snapshot.header.version,
        now,
    )
    .map_err(map_authorization_error)?;

    let amrn = v5::parse(documents.cartridge, contract)
        .map_err(|_| ChainVerificationError::InvalidAmrn)?;
    verify_cartridge_record(&target, &amrn, documents.cartridge)?;
    verify_amrn_signature(&delegation, &amrn)?;
    Ok(VerifiedRepositoryCartridge {
        target,
        delegation,
        amrn,
    })
}

fn decode_root(bytes: &[u8]) -> Result<RootMetadata, ChainVerificationError> {
    crate::parse_root_signed(bytes).map_err(|_| ChainVerificationError::Decode)
}
fn decode_timestamp(bytes: &[u8]) -> Result<TimestampMetadata, ChainVerificationError> {
    crate::parse_timestamp_signed(bytes).map_err(|_| ChainVerificationError::Decode)
}
fn decode_snapshot(bytes: &[u8]) -> Result<SnapshotMetadata, ChainVerificationError> {
    crate::parse_snapshot_signed(bytes).map_err(|_| ChainVerificationError::Decode)
}
fn decode_targets(bytes: &[u8]) -> Result<TargetsMetadata, ChainVerificationError> {
    crate::parse_targets_signed(bytes).map_err(|_| ChainVerificationError::Decode)
}
fn decode_revocations(bytes: &[u8]) -> Result<RevocationMetadata, ChainVerificationError> {
    crate::parse_revocation_signed(bytes).map_err(|_| ChainVerificationError::Decode)
}
fn decode_delegation(bytes: &[u8]) -> Result<DelegationMetadata, ChainVerificationError> {
    crate::parse_delegation_signed(bytes).map_err(|_| ChainVerificationError::Decode)
}

fn verify_role_document<V: SignatureVerifier>(
    verifier: &V,
    root: &RootMetadata,
    role: MetadataRole,
    envelope: SignedEnvelope<'_>,
    expires: u64,
    now: Option<u64>,
) -> Result<(), ChainVerificationError> {
    if now.is_some_and(|clock| expires != 0 && clock > expires) {
        return Err(ChainVerificationError::Expired);
    }
    let definition = root
        .roles
        .iter()
        .take(usize::from(root.role_count))
        .find(|item| item.role == role)
        .ok_or(ChainVerificationError::Signature)?;
    verify_role_signatures(
        verifier,
        envelope.signed,
        *definition,
        &root.keys[..usize::from(root.key_count)],
        envelope.signatures,
    )
    .map_err(|_| ChainVerificationError::Signature)
}

fn check_reference(
    expected_version: u64,
    expected_length: u32,
    expected_sha256: Sha256Digest,
    bytes: &[u8],
    actual_version: u64,
) -> Result<(), ChainVerificationError> {
    let digest = Sha256::digest(bytes);
    if expected_version != actual_version
        || expected_length != u32::try_from(bytes.len()).unwrap_or(u32::MAX)
        || expected_sha256.0 != digest[..]
    {
        return Err(ChainVerificationError::ReferenceMismatch);
    }
    Ok(())
}

fn find_target(
    targets: &TargetsMetadata,
    cartridge_id: CartridgeId,
) -> Result<TargetCartridge, ChainVerificationError> {
    targets
        .cartridges
        .iter()
        .take(usize::from(targets.cartridge_count))
        .find(|target| target.cartridge_id == cartridge_id)
        .copied()
        .ok_or(ChainVerificationError::MissingRecord)
}

fn verify_cartridge_record(
    target: &TargetCartridge,
    cartridge: &v5::Cartridge<'_>,
    bytes: &[u8],
) -> Result<(), ChainVerificationError> {
    let digest = Sha256::digest(bytes);
    let metadata = cartridge.header.metadata;
    if target.cartridge_id.0 != metadata.cartridge_id
        || target.length != u32::try_from(bytes.len()).unwrap_or(u32::MAX)
        || target.sha256.0 != digest[..]
        || target.amrn_format != u16::from(v5::FORMAT_VERSION)
        || target.abi_version != u16::from(v3::ABI_VERSION)
        || target.required_services != metadata.required_services
        || target.slot_id != metadata.slot_id
        || !version_matches(target.cartridge_version, metadata.cartridge_version)
        || !version_matches(
            target.minimum_kernel_version,
            metadata.minimum_kernel_version,
        )
    {
        return Err(ChainVerificationError::CartridgeRecordMismatch);
    }
    Ok(())
}

fn verify_amrn_signature(
    delegation: &DelegationMetadata,
    cartridge: &v5::Cartridge<'_>,
) -> Result<(), ChainVerificationError> {
    if cartridge.header.signature.key_id != delegation.key_id.0 {
        return Err(ChainVerificationError::InvalidAmrnSignature);
    }
    let signature = cartridge
        .header
        .signature
        .signature
        .try_into()
        .map_err(|_| ChainVerificationError::InvalidAmrnSignature)?;
    dali_crypto::verify(
        &delegation.public_key.0,
        cartridge.signed_bytes(),
        signature,
    )
    .map_err(|_| ChainVerificationError::InvalidAmrnSignature)
}

fn version_matches(
    text: BoundedText<{ crate::MAX_CARTRIDGE_VERSION_BYTES }>,
    version: v4::Version,
) -> bool {
    let Some(bytes) = text.as_str().map(str::as_bytes) else {
        return false;
    };
    let mut numbers = [0_u16; 3];
    let mut index = 0;
    let mut value = 0_u32;
    for byte in bytes.iter().copied().chain(core::iter::once(b'.')) {
        if byte.is_ascii_digit() {
            value = value
                .saturating_mul(10)
                .saturating_add(u32::from(byte - b'0'));
            if value > u32::from(u16::MAX) {
                return false;
            }
        } else if byte == b'.' && index < numbers.len() {
            numbers[index] = value as u16;
            index += 1;
            value = 0;
        } else {
            return false;
        }
    }
    index == 3 && numbers == [version.major, version.minor, version.patch]
}

fn map_authorization_error(_: CartridgeAuthorizationError) -> ChainVerificationError {
    ChainVerificationError::Authorization
}

#[cfg(test)]
mod tests;
