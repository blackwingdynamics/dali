use super::*;
use crate::{
    BoundedText, DelegationMetadata, KeyId, MetadataHeader, MetadataRole, PackageId, PublicKey,
    RoleDefinition, RoleKey, Sha256Digest, TargetPackage, TargetsMetadata,
};

fn header(role: MetadataRole) -> MetadataHeader {
    MetadataHeader {
        role,
        version: 1,
        expires: 0,
    }
}
fn text<const N: usize>(value: &str) -> BoundedText<N> {
    BoundedText::new(value).expect("test text fits")
}
fn target_package(delegation_id: &str) -> TargetPackage {
    TargetPackage {
        package_id: PackageId([1; crate::KEY_ID_LENGTH]),
        namespace: text("developer/app"),
        developer_id: text("developer"),
        delegation_id: text(delegation_id),
        developer_key_id: KeyId([2; crate::KEY_ID_LENGTH]),
        target_profile: text("f405"),
        amrn_format: 5,
        abi_version: 3,
        package_version: text("0.1.0"),
        minimum_kernel_version: text("0.1.0"),
        length: 128,
        sha256: Sha256Digest([3; crate::SHA256_LENGTH]),
        required_services: 1,
        slot_id: 0,
    }
}

#[test]
fn accepts_a_valid_header_without_a_clock() {
    assert_eq!(validate_header(header(MetadataRole::Targets), None), Ok(()));
}

#[test]
fn rejects_expired_metadata_when_a_clock_is_available() {
    assert_eq!(
        validate_header(
            MetadataHeader {
                expires: 10,
                ..header(MetadataRole::Targets)
            },
            Some(11)
        ),
        Err(MetadataError::Expired)
    );
}

#[test]
fn accepts_an_explicit_revocation_record() {
    let mut records = [crate::RevocationRecord::default(); crate::MAX_REVOCATIONS];
    records[0] = crate::RevocationRecord {
        developer_id: text("developer"),
        effective_version: 2,
        issuer_key_id: KeyId([1; crate::KEY_ID_LENGTH]),
        key_id: KeyId([2; crate::KEY_ID_LENGTH]),
        reason: text("compromised"),
    };
    assert_eq!(
        validate_revocation_metadata(&crate::RevocationMetadata {
            header: header(MetadataRole::Revocation),
            records,
            record_count: 1
        }),
        Ok(())
    );
}

#[test]
fn validates_threshold_against_active_keys() {
    let mut keys = [KeyId([0; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS];
    keys[0] = KeyId([1; crate::KEY_ID_LENGTH]);
    assert_eq!(
        validate_role(RoleDefinition {
            role: MetadataRole::Targets,
            keys,
            key_count: 1,
            threshold: 1
        }),
        Ok(())
    );
    assert_eq!(
        validate_role(RoleDefinition {
            role: MetadataRole::Targets,
            keys,
            key_count: 1,
            threshold: 2
        }),
        Err(MetadataError::InvalidRoleThreshold)
    );
}

#[test]
fn rejects_a_role_reference_to_an_unknown_key() {
    let role = RoleDefinition {
        role: MetadataRole::Targets,
        keys: [KeyId([9; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS],
        key_count: 1,
        threshold: 1,
    };
    let keys = [RoleKey {
        role: MetadataRole::Root,
        key_id: KeyId([1; crate::KEY_ID_LENGTH]),
        public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
    }];
    assert_eq!(
        validate_role_references(&keys, &[role]),
        Err(MetadataError::UnknownRoleKey)
    );
}

#[test]
fn validates_a_target_package_against_its_delegation() {
    let metadata = TargetsMetadata {
        header: header(MetadataRole::Targets),
        delegations: [text("developer"); crate::MAX_DELEGATION_SCOPES],
        delegation_count: 1,
        packages: [target_package("developer"); crate::MAX_TARGET_RECORDS],
        package_count: 1,
    };
    assert_eq!(validate_targets_metadata(&metadata), Ok(()));
}

#[test]
fn rejects_a_target_package_with_an_unknown_delegation() {
    let metadata = TargetsMetadata {
        header: header(MetadataRole::Targets),
        delegations: [text("other"); crate::MAX_DELEGATION_SCOPES],
        delegation_count: 1,
        packages: [target_package("developer"); crate::MAX_TARGET_RECORDS],
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
    let mut namespaces = [BoundedText::default(); crate::MAX_DELEGATION_SCOPES];
    namespaces[0] = text("developer/z");
    namespaces[1] = text("developer/a");
    let mut abis = [0; crate::MAX_DELEGATION_ABIS];
    abis[0] = 3;
    let delegation = DelegationMetadata {
        header: header(MetadataRole::Delegation),
        developer_id: text("developer"),
        key_id: KeyId([1; crate::KEY_ID_LENGTH]),
        public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
        allowed_namespaces: namespaces,
        namespace_count: 2,
        allowed_targets: [text("f405"); crate::MAX_DELEGATION_TARGETS],
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
