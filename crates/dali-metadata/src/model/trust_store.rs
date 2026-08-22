//! Board-agnostic durable trust-store payload model.

use super::{BoundedText, BundleFile, MetadataHeader};

/// Signed, package-free trust-store state persisted in one durable slot.
///
/// The references identify the complete Root -> Timestamp -> Snapshot ->
/// Targets -> Revocation -> Delegation state. Application packages are not
/// part of this payload and cannot be activated through this record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrustStorePayload {
    /// Recovery-role metadata fields; version is the durable generation.
    pub header: MetadataHeader,
    /// Target profile to which every referenced document applies.
    pub target_profile: BoundedText<{ crate::MAX_TARGET_PROFILE_BYTES }>,
    /// Canonical references to signed trust metadata files.
    pub files: [BundleFile; crate::MAX_TRUST_STORE_FILES],
    /// Number of active references in `files`.
    pub file_count: u16,
}
