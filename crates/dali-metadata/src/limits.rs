//! Target-owned bounds for the initial Dali metadata profile.

/// Canonical metadata schema identifier.
pub const SCHEMA_ID: &str = "dali.metadata.v1";
/// Fixed key identifier length in bytes.
pub const KEY_ID_LENGTH: usize = 16;
/// Ed25519 public-key length in bytes.
pub const PUBLIC_KEY_LENGTH: usize = 32;
/// SHA-256 digest length in bytes.
pub const SHA256_LENGTH: usize = 32;
/// Ed25519 signature length in bytes.
pub const SIGNATURE_LENGTH: usize = 64;
/// Maximum root metadata size in bytes.
pub const MAX_ROOT_BYTES: usize = 16 * 1024;
/// Maximum timestamp metadata size in bytes.
pub const MAX_TIMESTAMP_BYTES: usize = 4 * 1024;
/// Maximum snapshot metadata size in bytes.
pub const MAX_SNAPSHOT_BYTES: usize = 16 * 1024;
/// Maximum targets metadata size in bytes.
pub const MAX_TARGETS_BYTES: usize = 64 * 1024;
/// Maximum developer delegation size in bytes.
pub const MAX_DELEGATION_BYTES: usize = 4 * 1024;
/// Maximum complete offline trust-store bundle size in bytes.
pub const MAX_BUNDLE_BYTES: usize = 128 * 1024;
/// Maximum keys declared by one root document.
pub const MAX_ROOT_KEYS: usize = 16;
/// Maximum roles declared by one root document.
pub const MAX_ROOT_ROLES: usize = 16;
/// Maximum key identifiers authorized by one role.
pub const MAX_ROLE_KEYS: usize = 16;
/// Maximum package records in one targets document.
pub const MAX_TARGET_RECORDS: usize = 256;
/// Maximum metadata references in one snapshot document.
pub const MAX_SNAPSHOT_REFERENCES: usize = 64;
/// Maximum package scopes in one developer delegation.
pub const MAX_DELEGATION_SCOPES: usize = 32;
/// Maximum namespace bytes, excluding its separator.
pub const MAX_NAMESPACE_BYTES: usize = 128;
/// Maximum opaque developer identifier bytes.
pub const MAX_DEVELOPER_ID_BYTES: usize = 64;
/// Maximum bytes in a target profile identifier.
pub const MAX_TARGET_PROFILE_BYTES: usize = 32;
/// Maximum bytes in a package version string.
pub const MAX_PACKAGE_VERSION_BYTES: usize = 32;
/// Maximum bytes in a delegation identifier.
pub const MAX_DELEGATION_ID_BYTES: usize = 64;
