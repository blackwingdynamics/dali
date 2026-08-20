//! Fixed-capacity metadata contract records.

use crate::{
    KEY_ID_LENGTH, MAX_DELEGATION_ABIS, MAX_DELEGATION_ID_BYTES, MAX_DELEGATION_SCOPES,
    MAX_DELEGATION_TARGETS, MAX_DEVELOPER_ID_BYTES, MAX_NAMESPACE_BYTES, MAX_PACKAGE_VERSION_BYTES,
    MAX_TARGET_PROFILE_BYTES, PUBLIC_KEY_LENGTH, SHA256_LENGTH, SIGNATURE_LENGTH,
};

/// The role of one signed metadata document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataRole {
    /// Root keys and role thresholds.
    Root,
    /// Short-lived reference to one snapshot.
    Timestamp,
    /// Consistent references to targets and delegations.
    Snapshot,
    /// Package records and delegation identifiers.
    Targets,
    /// One developer public key and bounded permissions.
    Delegation,
    /// One signed offline trust-store bundle manifest.
    Bundle,
}

impl MetadataRole {
    /// Returns the canonical wire name for this role.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Timestamp => "timestamp",
            Self::Snapshot => "snapshot",
            Self::Targets => "targets",
            Self::Delegation => "delegation",
            Self::Bundle => "bundle",
        }
    }
}

/// Fixed-size opaque key identifier.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyId(pub [u8; KEY_ID_LENGTH]);

/// Fixed-capacity package identity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PackageId(pub [u8; KEY_ID_LENGTH]);

/// Fixed-capacity SHA-256 digest.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sha256Digest(pub [u8; SHA256_LENGTH]);

/// Errors returned when constructing bounded text values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextError {
    /// The value is empty where the contract requires an identifier.
    Empty,
    /// The value does not fit in its fixed-capacity field.
    TooLong,
}

/// UTF-8 text stored without heap allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundedText<const CAPACITY: usize> {
    bytes: [u8; CAPACITY],
    length: u16,
}

impl<const CAPACITY: usize> Default for BoundedText<CAPACITY> {
    fn default() -> Self {
        Self {
            bytes: [0; CAPACITY],
            length: 0,
        }
    }
}

impl<const CAPACITY: usize> BoundedText<CAPACITY> {
    /// Creates a non-empty bounded text value.
    pub fn new(value: &str) -> Result<Self, TextError> {
        if value.is_empty() {
            return Err(TextError::Empty);
        }
        if value.len() > CAPACITY || value.len() > usize::from(u16::MAX) {
            return Err(TextError::TooLong);
        }
        let mut bytes = [0; CAPACITY];
        bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(Self {
            bytes,
            length: value.len() as u16,
        })
    }

    /// Returns the bounded value as UTF-8 text when its invariant is intact.
    pub fn as_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.bytes[..usize::from(self.length)]).ok()
    }
}

/// Fixed-size Ed25519 public key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicKey(pub [u8; PUBLIC_KEY_LENGTH]);

/// Fixed-size Ed25519 signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Signature(pub [u8; SIGNATURE_LENGTH]);

impl Default for Signature {
    fn default() -> Self {
        Self([0; SIGNATURE_LENGTH])
    }
}

/// One signature and the key identifier that produced it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SignatureRecord {
    /// Authorized signer key identifier.
    pub key_id: KeyId,
    /// Ed25519 signature bytes over the canonical signed body.
    pub signature: Signature,
}

/// Fixed-capacity signature list carried by one metadata envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignatureSet {
    /// Signature records in canonical key ID order.
    pub records: [SignatureRecord; crate::MAX_SIGNATURES],
    /// Number of active records.
    pub count: u8,
}

/// Borrowed signed metadata body paired with its bounded signatures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedEnvelope<'a> {
    /// Canonical signed body bytes; these exact bytes are authenticated.
    pub signed: &'a [u8],
    /// Signatures over `signed`.
    pub signatures: SignatureSet,
}

/// Common fields shared by every signed metadata role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataHeader {
    /// The role-specific metadata role.
    pub role: MetadataRole,
    /// Monotonic version for this role.
    pub version: u64,
    /// Unix expiration time; zero means no trusted wall clock is available.
    pub expires: u64,
}

/// One public key authorized for a repository role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleKey {
    /// Repository role authorized to use this key.
    pub role: MetadataRole,
    /// Key identifier selected by a signature.
    pub key_id: KeyId,
    /// Ed25519 public key bytes.
    pub public_key: PublicKey,
}

/// One repository role and its signature threshold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleDefinition {
    /// Canonical role name.
    pub role: MetadataRole,
    /// Authorized keys; only the first `key_count` entries are active.
    pub keys: [KeyId; crate::MAX_ROLE_KEYS],
    /// Number of active entries in `keys`.
    pub key_count: u8,
    /// Number of distinct valid signatures required.
    pub threshold: u8,
}

/// Decoded root metadata held in fixed-capacity storage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootMetadata {
    /// Common signed metadata fields.
    pub header: MetadataHeader,
    /// Root and delegated public keys.
    pub keys: [RoleKey; crate::MAX_ROOT_KEYS],
    /// Number of active entries in `keys`.
    pub key_count: u8,
    /// Repository role definitions.
    pub roles: [RoleDefinition; crate::MAX_ROOT_ROLES],
    /// Number of active entries in `roles`.
    pub role_count: u8,
}

/// One package authorization record from targets metadata.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TargetPackage {
    /// Exact package identity carried by the AMRN package.
    pub package_id: PackageId,
    /// Developer-owned package namespace.
    pub namespace: BoundedText<MAX_NAMESPACE_BYTES>,
    /// Delegated developer identifier.
    pub developer_id: BoundedText<MAX_DEVELOPER_ID_BYTES>,
    /// Delegation record authorizing the developer key and namespace.
    pub delegation_id: BoundedText<MAX_DELEGATION_ID_BYTES>,
    /// Developer key authorized by the delegation.
    pub developer_key_id: KeyId,
    /// Target profile required by the package.
    pub target_profile: BoundedText<MAX_TARGET_PROFILE_BYTES>,
    /// AMRN format required by the package.
    pub amrn_format: u16,
    /// Application ABI required by the package.
    pub abi_version: u16,
    /// Package semantic version.
    pub package_version: BoundedText<MAX_PACKAGE_VERSION_BYTES>,
    /// Minimum compatible kernel version.
    pub minimum_kernel_version: BoundedText<MAX_PACKAGE_VERSION_BYTES>,
    /// Exact stored AMRN file length.
    pub length: u32,
    /// Exact SHA-256 digest of the stored AMRN file.
    pub sha256: Sha256Digest,
    /// Service capability bitset requested by the package.
    pub required_services: u32,
    /// Declared application slot.
    pub slot_id: u8,
}

/// Bounded targets metadata records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetsMetadata {
    /// Common signed metadata fields.
    pub header: MetadataHeader,
    /// Delegation identifiers referenced by package records.
    pub delegations: [BoundedText<MAX_DELEGATION_ID_BYTES>; MAX_DELEGATION_SCOPES],
    /// Number of active delegation identifiers.
    pub delegation_count: u8,
    /// Package authorization records.
    pub packages: [TargetPackage; crate::MAX_TARGET_RECORDS],
    /// Number of active package records.
    pub package_count: u16,
}

/// One bounded developer delegation signed by a repository authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegationMetadata {
    /// Common signed metadata fields.
    pub header: MetadataHeader,
    /// Stable developer registry identifier.
    pub developer_id: BoundedText<MAX_DEVELOPER_ID_BYTES>,
    /// Developer signing key identifier.
    pub key_id: KeyId,
    /// Developer Ed25519 public key.
    pub public_key: PublicKey,
    /// Exact namespaces authorized for this developer.
    pub allowed_namespaces: [BoundedText<MAX_NAMESPACE_BYTES>; MAX_DELEGATION_SCOPES],
    /// Number of active namespace entries.
    pub namespace_count: u8,
    /// Target profiles authorized for this developer.
    pub allowed_targets: [BoundedText<MAX_TARGET_PROFILE_BYTES>; MAX_DELEGATION_TARGETS],
    /// Number of active target entries.
    pub target_count: u8,
    /// ABI versions authorized for this developer.
    pub allowed_abis: [u16; MAX_DELEGATION_ABIS],
    /// Number of active ABI entries.
    pub abi_count: u8,
    /// Inclusive validity start; zero means no lower bound.
    pub not_before: u64,
    /// Inclusive validity end; zero means no upper bound.
    pub not_after: u64,
}

/// Kind of file referenced by an offline bundle manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleFileKind {
    /// Root metadata file.
    Root,
    /// Timestamp metadata file.
    Timestamp,
    /// Snapshot metadata file.
    Snapshot,
    /// Targets metadata file.
    Targets,
    /// Developer delegation metadata file.
    Delegation,
    /// AMRN package artifact.
    Package,
}

impl BundleFileKind {
    /// Returns the canonical wire name for this file kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Timestamp => "timestamp",
            Self::Snapshot => "snapshot",
            Self::Targets => "targets",
            Self::Delegation => "delegation",
            Self::Package => "package",
        }
    }
}

/// One bounded file reference from an offline bundle manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BundleFile {
    /// Logical file kind.
    pub kind: BundleFileKind,
    /// Fixed metadata identity, delegation ID, or package hash stem.
    pub id: BoundedText<{ crate::MAX_BUNDLE_ID_BYTES }>,
    /// Exact file length in bytes.
    pub length: u32,
    /// Exact SHA-256 digest of the complete file.
    pub sha256: Sha256Digest,
}

impl Default for BundleFile {
    fn default() -> Self {
        Self {
            kind: BundleFileKind::Root,
            id: BoundedText::default(),
            length: 0,
            sha256: Sha256Digest::default(),
        }
    }
}

/// Bounded signed offline trust-store bundle manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BundleMetadata {
    /// Common signed metadata fields; version is the durable bundle version.
    pub header: MetadataHeader,
    /// Target profile for every bundle artifact.
    pub target_profile: BoundedText<MAX_TARGET_PROFILE_BYTES>,
    /// Referenced files in canonical kind/id order.
    pub files: [BundleFile; crate::MAX_BUNDLE_FILES],
    /// Number of active file references.
    pub file_count: u16,
}

/// Hash-and-length reference to the targets metadata file.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TargetsReference {
    /// Monotonic targets metadata version.
    pub version: u64,
    /// Exact serialized targets metadata length.
    pub length: u32,
    /// Exact serialized targets metadata digest.
    pub sha256: Sha256Digest,
}

/// Hash-and-length reference to one delegated metadata file.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DelegationReference {
    /// Delegation identifier.
    pub id: BoundedText<MAX_DELEGATION_ID_BYTES>,
    /// Monotonic delegation metadata version.
    pub version: u64,
    /// Exact serialized delegation metadata length.
    pub length: u32,
    /// Exact serialized delegation metadata digest.
    pub sha256: Sha256Digest,
}

/// Snapshot metadata references held in fixed-capacity storage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotMetadata {
    /// Common signed metadata fields.
    pub header: MetadataHeader,
    /// Exact targets metadata reference.
    pub targets: TargetsReference,
    /// Delegation metadata references.
    pub delegations: [DelegationReference; crate::MAX_SNAPSHOT_REFERENCES],
    /// Number of active delegation references.
    pub delegation_count: u8,
}

/// Timestamp metadata containing one snapshot reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimestampMetadata {
    /// Common signed metadata fields.
    pub header: MetadataHeader,
    /// Exact snapshot metadata version.
    pub snapshot_version: u64,
    /// Exact serialized snapshot metadata length.
    pub snapshot_length: u32,
    /// Exact serialized snapshot metadata digest.
    pub snapshot_sha256: Sha256Digest,
}

#[cfg(test)]
mod tests {
    use super::{BoundedText, TextError};

    #[test]
    fn stores_bounded_text_without_heap_allocation() {
        let value = BoundedText::<8>::new("dali").expect("test value fits");
        assert_eq!(value.as_str(), Some("dali"));
    }

    #[test]
    fn rejects_empty_and_oversized_text() {
        assert_eq!(BoundedText::<8>::new(""), Err(TextError::Empty));
        assert_eq!(BoundedText::<3>::new("dali"), Err(TextError::TooLong));
    }
}
