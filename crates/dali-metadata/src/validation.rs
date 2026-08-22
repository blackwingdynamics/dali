//! Validation for bounded metadata contract values.

mod bundle;
mod common;
mod core;
mod delegation;
mod records;
mod trust_store;

pub use bundle::validate_bundle_metadata;
pub(crate) use common::validate_reference;
pub use common::{
    is_repository_role, validate_developer_id, validate_namespace, validate_signature_set,
};
pub use core::{
    TrustedTime, validate_header, validate_header_at, validate_role, validate_role_references,
};
pub use delegation::validate_delegation;
pub(crate) use delegation::validate_target_profile;
pub use records::{
    validate_revocation_metadata, validate_snapshot_metadata, validate_target_records,
    validate_targets_metadata, validate_timestamp_metadata,
};
pub use trust_store::validate_trust_store_payload;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataError {
    UnsupportedRole,
    InvalidVersion,
    Expired,
    InvalidKeyId,
    UnknownRoleKey,
    InvalidRoleThreshold,
    InvalidNamespace,
    InvalidDeveloperId,
    InvalidPackageRecord,
    UnknownDelegation,
    DuplicatePackage,
    InvalidMetadataReference,
    DuplicateDelegationReference,
    InvalidSignatureSet,
    InvalidDelegation,
    InvalidBundle,
    InvalidRevocation,
}

#[cfg(test)]
mod tests;
