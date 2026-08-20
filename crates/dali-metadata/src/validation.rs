//! Validation for bounded metadata contract values.

use crate::{
    KEY_ID_LENGTH, MAX_DELEGATION_SCOPES, MAX_DEVELOPER_ID_BYTES, MAX_NAMESPACE_BYTES,
    MAX_ROLE_KEYS, MAX_TARGET_RECORDS, MetadataHeader, MetadataRole, RoleDefinition, RoleKey,
    TargetsMetadata,
};

mod delegation;

pub use delegation::validate_delegation;
pub(crate) use delegation::validate_target_profile;

/// Errors returned when contract values violate the initial metadata profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataError {
    /// The metadata role is not supported by this profile.
    UnsupportedRole,
    /// The role version is zero or cannot be accepted as monotonic state.
    InvalidVersion,
    /// The role expiration is invalid for the supplied clock policy.
    Expired,
    /// A key identifier contains no meaningful identity.
    InvalidKeyId,
    /// A role references a key that is absent from the root key set.
    UnknownRoleKey,
    /// A role threshold or key count is outside its declared bounds.
    InvalidRoleThreshold,
    /// A namespace is empty, too long, or not in canonical form.
    InvalidNamespace,
    /// A developer identifier is empty or too long.
    InvalidDeveloperId,
    /// A package record violates its bounded identity or compatibility fields.
    InvalidPackageRecord,
    /// A package references a delegation absent from targets metadata.
    UnknownDelegation,
    /// Two target records claim the same package identity.
    DuplicatePackage,
    /// A metadata reference is empty or has an invalid version, length, or hash.
    InvalidMetadataReference,
    /// Two snapshot references claim the same delegation identifier.
    DuplicateDelegationReference,
    /// A signature list contains an invalid, duplicate, or unsorted signer.
    InvalidSignatureSet,
    /// A developer delegation violates its bounded authorization contract.
    InvalidDelegation,
    /// An offline bundle manifest violates its bounded file contract.
    InvalidBundle,
}

/// Validates common signed metadata fields.
pub fn validate_header(header: MetadataHeader, now: Option<u64>) -> Result<(), MetadataError> {
    if header.version == 0 {
        return Err(MetadataError::InvalidVersion);
    }
    if let (Some(now), expiration) = (now, header.expires)
        && expiration != 0
        && now > expiration
    {
        return Err(MetadataError::Expired);
    }
    Ok(())
}

/// Validates one role's bounded key list and signature threshold.
pub fn validate_role(role: RoleDefinition) -> Result<(), MetadataError> {
    let key_count = usize::from(role.key_count);
    let threshold = usize::from(role.threshold);
    if key_count == 0 || key_count > MAX_ROLE_KEYS || threshold == 0 || threshold > key_count {
        return Err(MetadataError::InvalidRoleThreshold);
    }
    for key in role.keys.iter().take(key_count) {
        if key.0 == [0; KEY_ID_LENGTH] {
            return Err(MetadataError::InvalidKeyId);
        }
    }
    Ok(())
}

/// Validates that every role reference resolves to a declared public key.
pub fn validate_role_references(
    keys: &[RoleKey],
    roles: &[RoleDefinition],
) -> Result<(), MetadataError> {
    for role in roles {
        for key_id in role.keys.iter().take(usize::from(role.key_count)) {
            if !keys.iter().any(|key| key.key_id == *key_id) {
                return Err(MetadataError::UnknownRoleKey);
            }
        }
    }
    Ok(())
}

/// Validates bounded package authorization records in targets metadata.
pub fn validate_targets_metadata(metadata: &TargetsMetadata) -> Result<(), MetadataError> {
    if metadata.header.role != MetadataRole::Targets
        || metadata.header.version == 0
        || usize::from(metadata.delegation_count) > MAX_DELEGATION_SCOPES
        || usize::from(metadata.package_count) > MAX_TARGET_RECORDS
    {
        return Err(MetadataError::InvalidPackageRecord);
    }
    let delegations = &metadata.delegations[..usize::from(metadata.delegation_count)];
    let packages = &metadata.packages[..usize::from(metadata.package_count)];
    validate_target_records(delegations, packages)
}

/// Validates target records supplied by a bounded encoder or parser.
pub fn validate_target_records(
    delegations: &[crate::BoundedText<{ crate::MAX_DELEGATION_ID_BYTES }>],
    packages: &[crate::TargetPackage],
) -> Result<(), MetadataError> {
    if delegations.len() > MAX_DELEGATION_SCOPES || packages.len() > MAX_TARGET_RECORDS {
        return Err(MetadataError::InvalidPackageRecord);
    }
    for (index, delegation) in delegations.iter().enumerate() {
        if delegation.as_str().is_none()
            || delegations[..index]
                .iter()
                .any(|candidate| candidate == delegation)
        {
            return Err(MetadataError::InvalidPackageRecord);
        }
    }
    for (index, package) in packages.iter().enumerate() {
        if package.package_id.0 == [0; KEY_ID_LENGTH]
            || package.developer_key_id.0 == [0; KEY_ID_LENGTH]
            || package.sha256.0 == [0; crate::SHA256_LENGTH]
            || package.length == 0
            || package.amrn_format == 0
            || package.abi_version == 0
            || package.namespace.as_str().is_none()
            || package.developer_id.as_str().is_none()
            || package.delegation_id.as_str().is_none()
            || package.target_profile.as_str().is_none()
            || package.package_version.as_str().is_none()
            || package.minimum_kernel_version.as_str().is_none()
        {
            return Err(MetadataError::InvalidPackageRecord);
        }
        validate_namespace(
            package
                .namespace
                .as_str()
                .ok_or(MetadataError::InvalidPackageRecord)?,
        )?;
        validate_developer_id(
            package
                .developer_id
                .as_str()
                .ok_or(MetadataError::InvalidPackageRecord)?,
        )?;
        if !delegations
            .iter()
            .any(|delegation| delegation == &package.delegation_id)
        {
            return Err(MetadataError::UnknownDelegation);
        }
        if packages[..index]
            .iter()
            .any(|candidate| candidate.package_id == package.package_id)
        {
            return Err(MetadataError::DuplicatePackage);
        }
    }
    Ok(())
}

/// Validates snapshot references before encoding or accepting them.
pub fn validate_snapshot_metadata(metadata: &crate::SnapshotMetadata) -> Result<(), MetadataError> {
    if metadata.header.role != MetadataRole::Snapshot
        || metadata.header.version == 0
        || usize::from(metadata.delegation_count) > crate::MAX_SNAPSHOT_REFERENCES
    {
        return Err(MetadataError::InvalidMetadataReference);
    }
    validate_reference(
        metadata.targets.version,
        metadata.targets.length,
        metadata.targets.sha256,
    )?;
    for (index, reference) in metadata
        .delegations
        .iter()
        .take(usize::from(metadata.delegation_count))
        .enumerate()
    {
        if reference.id.as_str().is_none()
            || metadata.delegations[..index]
                .iter()
                .any(|candidate| candidate.id == reference.id)
        {
            return Err(if reference.id.as_str().is_none() {
                MetadataError::InvalidMetadataReference
            } else {
                MetadataError::DuplicateDelegationReference
            });
        }
        validate_reference(reference.version, reference.length, reference.sha256)?;
    }
    Ok(())
}

/// Validates the single snapshot reference in timestamp metadata.
pub fn validate_timestamp_metadata(
    metadata: &crate::TimestampMetadata,
) -> Result<(), MetadataError> {
    if metadata.header.role != MetadataRole::Timestamp || metadata.header.version == 0 {
        return Err(MetadataError::InvalidMetadataReference);
    }
    validate_reference(
        metadata.snapshot_version,
        metadata.snapshot_length,
        metadata.snapshot_sha256,
    )
}

/// Validates one bounded offline bundle manifest.
pub fn validate_bundle_metadata(metadata: &crate::BundleMetadata) -> Result<(), MetadataError> {
    if metadata.header.role != MetadataRole::Bundle
        || metadata.header.version == 0
        || usize::from(metadata.file_count) > crate::MAX_BUNDLE_FILES
    {
        return Err(MetadataError::InvalidBundle);
    }
    let target_profile = metadata
        .target_profile
        .as_str()
        .ok_or(MetadataError::InvalidBundle)?;
    validate_target_profile(target_profile).map_err(|_| MetadataError::InvalidBundle)?;
    let files = &metadata.files[..usize::from(metadata.file_count)];
    if files.is_empty() {
        return Err(MetadataError::InvalidBundle);
    }
    for (index, file) in files.iter().enumerate() {
        let id = file.id.as_str().ok_or(MetadataError::InvalidBundle)?;
        if file.length == 0
            || file.sha256.0 == [0; crate::SHA256_LENGTH]
            || files[..index]
                .iter()
                .any(|candidate| (candidate.kind, candidate.id) == (file.kind, file.id))
            || (index > 0
                && bundle_file_order(files[index - 1], *file) != core::cmp::Ordering::Less)
        {
            return Err(MetadataError::InvalidBundle);
        }
        if matches!(
            file.kind,
            crate::BundleFileKind::Root
                | crate::BundleFileKind::Timestamp
                | crate::BundleFileKind::Snapshot
                | crate::BundleFileKind::Targets
        ) && id != file.kind.as_str()
        {
            return Err(MetadataError::InvalidBundle);
        }
    }
    Ok(())
}

fn bundle_file_order(left: crate::BundleFile, right: crate::BundleFile) -> core::cmp::Ordering {
    left.kind.as_str().cmp(right.kind.as_str()).then_with(|| {
        left.id
            .as_str()
            .unwrap_or("")
            .cmp(right.id.as_str().unwrap_or(""))
    })
}

fn validate_reference(
    version: u64,
    length: u32,
    sha256: crate::Sha256Digest,
) -> Result<(), MetadataError> {
    if version == 0 || length == 0 || sha256.0 == [0; crate::SHA256_LENGTH] {
        Err(MetadataError::InvalidMetadataReference)
    } else {
        Ok(())
    }
}

/// Validates a bounded signature set before encoding or verification.
pub fn validate_signature_set(set: &crate::SignatureSet) -> Result<(), MetadataError> {
    let count = usize::from(set.count);
    if count > crate::MAX_SIGNATURES {
        return Err(MetadataError::InvalidSignatureSet);
    }
    let records = &set.records[..count];
    if records.iter().any(|record| {
        record.key_id.0 == [0; crate::KEY_ID_LENGTH]
            || record.signature.0 == [0; crate::SIGNATURE_LENGTH]
    }) || !records
        .windows(2)
        .all(|pair| pair[0].key_id.0 < pair[1].key_id.0)
    {
        return Err(MetadataError::InvalidSignatureSet);
    }
    Ok(())
}

/// Validates a developer identifier under the bounded ASCII-compatible rule.
pub fn validate_developer_id(value: &str) -> Result<(), MetadataError> {
    if value.is_empty()
        || value.len() > MAX_DEVELOPER_ID_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
    {
        return Err(MetadataError::InvalidDeveloperId);
    }
    Ok(())
}

/// Validates an exact or bounded namespace according to the initial profile.
pub fn validate_namespace(value: &str) -> Result<(), MetadataError> {
    if value.is_empty()
        || value.len() > MAX_NAMESPACE_BYTES
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b'-'
                || byte == b'_'
                || byte == b'/'
        })
    {
        return Err(MetadataError::InvalidNamespace);
    }
    Ok(())
}

/// Returns whether a role is one of the profile's repository roles.
pub const fn is_repository_role(role: MetadataRole) -> bool {
    matches!(
        role,
        MetadataRole::Root
            | MetadataRole::Timestamp
            | MetadataRole::Snapshot
            | MetadataRole::Targets
            | MetadataRole::Delegation
            | MetadataRole::Bundle
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BoundedText, DelegationMetadata, KeyId, MAX_DELEGATION_ABIS, MAX_DELEGATION_SCOPES,
        MAX_DELEGATION_TARGETS, MAX_NAMESPACE_BYTES, MAX_PACKAGE_VERSION_BYTES,
        MAX_TARGET_PROFILE_BYTES, MetadataHeader, MetadataRole, PackageId, PublicKey,
        RoleDefinition, RoleKey, Sha256Digest, TargetPackage, TargetsMetadata,
    };

    #[test]
    fn accepts_a_valid_header_without_a_clock() {
        assert_eq!(
            validate_header(
                MetadataHeader {
                    role: MetadataRole::Targets,
                    version: 1,
                    expires: 0,
                },
                None,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_expired_metadata_when_a_clock_is_available() {
        assert_eq!(
            validate_header(
                MetadataHeader {
                    role: MetadataRole::Targets,
                    version: 1,
                    expires: 10,
                },
                Some(11),
            ),
            Err(MetadataError::Expired)
        );
    }

    #[test]
    fn validates_threshold_against_active_keys() {
        let mut keys = [KeyId([0; crate::KEY_ID_LENGTH]); MAX_ROLE_KEYS];
        keys[0] = KeyId([1; crate::KEY_ID_LENGTH]);
        assert_eq!(
            validate_role(RoleDefinition {
                role: MetadataRole::Targets,
                keys,
                key_count: 1,
                threshold: 1,
            }),
            Ok(())
        );
        assert_eq!(
            validate_role(RoleDefinition {
                role: MetadataRole::Targets,
                keys,
                key_count: 1,
                threshold: 2,
            }),
            Err(MetadataError::InvalidRoleThreshold)
        );
    }

    #[test]
    fn rejects_a_role_reference_to_an_unknown_key() {
        let role = RoleDefinition {
            role: MetadataRole::Targets,
            keys: [KeyId([9; crate::KEY_ID_LENGTH]); MAX_ROLE_KEYS],
            key_count: 1,
            threshold: 1,
        };
        let keys = [RoleKey {
            role: MetadataRole::Root,
            key_id: KeyId([1; crate::KEY_ID_LENGTH]),
            public_key: crate::PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
        }];
        assert_eq!(
            validate_role_references(&keys, &[role]),
            Err(MetadataError::UnknownRoleKey)
        );
    }

    fn target_package(delegation_id: &str) -> TargetPackage {
        TargetPackage {
            package_id: PackageId([1; crate::KEY_ID_LENGTH]),
            namespace: BoundedText::<MAX_NAMESPACE_BYTES>::new("developer/app")
                .expect("test namespace fits"),
            developer_id: BoundedText::<{ crate::MAX_DEVELOPER_ID_BYTES }>::new("developer")
                .expect("test developer fits"),
            delegation_id: BoundedText::<{ crate::MAX_DELEGATION_ID_BYTES }>::new(delegation_id)
                .expect("test delegation fits"),
            developer_key_id: KeyId([2; crate::KEY_ID_LENGTH]),
            target_profile: BoundedText::<MAX_TARGET_PROFILE_BYTES>::new("f405")
                .expect("test target fits"),
            amrn_format: 5,
            abi_version: 3,
            package_version: BoundedText::<MAX_PACKAGE_VERSION_BYTES>::new("0.1.0")
                .expect("test version fits"),
            minimum_kernel_version: BoundedText::<MAX_PACKAGE_VERSION_BYTES>::new("0.1.0")
                .expect("test minimum version fits"),
            length: 128,
            sha256: Sha256Digest([3; crate::SHA256_LENGTH]),
            required_services: 1,
            slot_id: 0,
        }
    }

    #[test]
    fn validates_a_target_package_against_its_delegation() {
        let delegation = BoundedText::<{ crate::MAX_DELEGATION_ID_BYTES }>::new("developer")
            .expect("test delegation fits");
        let package = target_package("developer");
        let metadata = TargetsMetadata {
            header: MetadataHeader {
                role: MetadataRole::Targets,
                version: 1,
                expires: 0,
            },
            delegations: [delegation; MAX_DELEGATION_SCOPES],
            delegation_count: 1,
            packages: [package; crate::MAX_TARGET_RECORDS],
            package_count: 1,
        };
        assert_eq!(validate_targets_metadata(&metadata), Ok(()));
    }

    #[test]
    fn rejects_a_target_package_with_an_unknown_delegation() {
        let delegation = BoundedText::<{ crate::MAX_DELEGATION_ID_BYTES }>::new("other")
            .expect("test delegation fits");
        let package = target_package("developer");
        let metadata = TargetsMetadata {
            header: MetadataHeader {
                role: MetadataRole::Targets,
                version: 1,
                expires: 0,
            },
            delegations: [delegation; MAX_DELEGATION_SCOPES],
            delegation_count: 1,
            packages: [package; crate::MAX_TARGET_RECORDS],
            package_count: 1,
        };
        assert_eq!(
            validate_targets_metadata(&metadata),
            Err(MetadataError::UnknownDelegation)
        );
    }

    #[test]
    fn rejects_non_canonical_namespace_values() {
        assert_eq!(validate_namespace("developer/app"), Ok(()));
        assert_eq!(
            validate_namespace("Developer/app"),
            Err(MetadataError::InvalidNamespace)
        );
        assert_eq!(
            validate_namespace("developer//app"),
            Err(MetadataError::InvalidNamespace)
        );
    }

    #[test]
    fn validates_bounded_developer_identifiers() {
        assert_eq!(validate_developer_id("developer-1"), Ok(()));
        assert_eq!(
            validate_developer_id("Developer"),
            Err(MetadataError::InvalidDeveloperId)
        );
    }

    #[test]
    fn rejects_an_unordered_developer_delegation_scope() {
        let mut namespaces = [BoundedText::default(); MAX_DELEGATION_SCOPES];
        namespaces[0] = BoundedText::new("developer/z").expect("test namespace fits");
        namespaces[1] = BoundedText::new("developer/a").expect("test namespace fits");
        let mut targets = [BoundedText::default(); MAX_DELEGATION_TARGETS];
        targets[0] = BoundedText::new("f405").expect("test target fits");
        let mut abis = [0; MAX_DELEGATION_ABIS];
        abis[0] = 3;
        let delegation = DelegationMetadata {
            header: MetadataHeader {
                role: MetadataRole::Delegation,
                version: 1,
                expires: 0,
            },
            developer_id: BoundedText::new("developer").expect("test developer fits"),
            key_id: KeyId([1; crate::KEY_ID_LENGTH]),
            public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
            allowed_namespaces: namespaces,
            namespace_count: 2,
            allowed_targets: targets,
            target_count: 1,
            allowed_abis: abis,
            abi_count: 1,
            not_before: 0,
            not_after: 0,
        };
        assert_eq!(
            validate_delegation(&delegation),
            Err(MetadataError::InvalidDelegation)
        );
    }
}
