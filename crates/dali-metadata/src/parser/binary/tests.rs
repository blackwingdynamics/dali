use super::*;
use crate::{
    BoundedText, DelegationMetadata, DelegationReference, KeyId, MetadataHeader, MetadataRole,
    PublicKey, RevocationReference, RoleDefinition, RoleKey, Sha256Digest, SnapshotMetadata,
    TargetPackage, TargetsMetadata, TargetsReference,
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

fn digest() -> Sha256Digest {
    Sha256Digest([7; crate::SHA256_LENGTH])
}

#[test]
fn round_trips_binary_role_bodies() {
    let key = KeyId([1; crate::KEY_ID_LENGTH]);
    let empty_key = RoleKey {
        role: MetadataRole::Root,
        key_id: KeyId([0; crate::KEY_ID_LENGTH]),
        public_key: PublicKey([0; crate::PUBLIC_KEY_LENGTH]),
    };
    let mut root_keys = [empty_key; crate::MAX_ROOT_KEYS];
    root_keys[0] = RoleKey {
        role: MetadataRole::Root,
        key_id: key,
        public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
    };
    let empty_role = RoleDefinition {
        role: MetadataRole::Root,
        keys: [KeyId([0; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS],
        key_count: 0,
        threshold: 0,
    };
    let mut roles = [empty_role; crate::MAX_ROOT_ROLES];
    let mut role_keys = [KeyId([0; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS];
    role_keys[0] = key;
    roles[0] = RoleDefinition {
        role: MetadataRole::Targets,
        keys: role_keys,
        key_count: 1,
        threshold: 1,
    };
    let root = crate::RootMetadata {
        header: header(MetadataRole::Root),
        keys: root_keys,
        key_count: 1,
        roles,
        role_count: 1,
    };
    let mut buffer = [0; crate::MAX_ROOT_BYTES];
    let length = encode_binary_root_body(root, &mut buffer).expect("root encodes");
    let parsed = parse_binary_root_body(&buffer[..length]).expect("root parses");
    assert_eq!(parsed.header, root.header);
    assert_eq!(parsed.keys[..1], root.keys[..1]);
    assert_eq!(parsed.roles[..1], root.roles[..1]);

    let reference = TargetsReference {
        version: 1,
        length: 64,
        sha256: digest(),
    };
    let snapshot = SnapshotMetadata {
        header: header(MetadataRole::Snapshot),
        targets: reference,
        revocations: RevocationReference {
            version: 1,
            length: 64,
            sha256: digest(),
        },
        delegations: [DelegationReference {
            id: text("developer"),
            version: 1,
            length: 64,
            sha256: digest(),
        }; crate::MAX_SNAPSHOT_REFERENCES],
        delegation_count: 1,
    };
    let mut buffer = [0; crate::MAX_SNAPSHOT_BYTES];
    let length = encode_binary_snapshot_body(snapshot, &mut buffer).expect("snapshot encodes");
    let parsed = parse_binary_snapshot_body(&buffer[..length]).expect("snapshot parses");
    assert_eq!(parsed.header, snapshot.header);
    assert_eq!(parsed.targets, snapshot.targets);
    assert_eq!(parsed.revocations, snapshot.revocations);
    assert_eq!(parsed.delegations[..1], snapshot.delegations[..1]);

    let delegation = DelegationMetadata {
        header: header(MetadataRole::Delegation),
        developer_id: text("developer"),
        key_id: KeyId([2; crate::KEY_ID_LENGTH]),
        public_key: PublicKey([3; crate::PUBLIC_KEY_LENGTH]),
        allowed_namespaces: [text("developer/app"); crate::MAX_DELEGATION_SCOPES],
        namespace_count: 1,
        allowed_targets: [text("f405"); crate::MAX_DELEGATION_TARGETS],
        target_count: 1,
        allowed_abis: {
            let mut values = [0; crate::MAX_DELEGATION_ABIS];
            values[0] = 3;
            values
        },
        abi_count: 1,
        not_before: 0,
        not_after: 0,
    };
    let mut buffer = [0; crate::MAX_DELEGATION_BYTES];
    let length =
        encode_binary_delegation_body(delegation, &mut buffer).expect("delegation encodes");
    let parsed = parse_binary_delegation_body(&buffer[..length]).expect("delegation parses");
    assert_eq!(parsed.header, delegation.header);
    assert_eq!(parsed.developer_id, delegation.developer_id);
    assert_eq!(parsed.key_id, delegation.key_id);
    assert_eq!(parsed.public_key, delegation.public_key);
    assert_eq!(
        parsed.allowed_namespaces[..1],
        delegation.allowed_namespaces[..1]
    );
    assert_eq!(parsed.allowed_targets[..1], delegation.allowed_targets[..1]);
    assert_eq!(parsed.allowed_abis[..1], delegation.allowed_abis[..1]);
}

#[test]
fn target_record_length_rejects_truncation() {
    let mut buffer = [0; crate::MAX_TARGETS_BYTES];
    let metadata = TargetsMetadata {
        header: header(MetadataRole::Targets),
        delegations: [text("developer"); crate::MAX_DELEGATION_SCOPES],
        delegation_count: 1,
        packages: [TargetPackage {
            package_id: crate::PackageId([1; crate::KEY_ID_LENGTH]),
            namespace: text("developer/app"),
            developer_id: text("developer"),
            delegation_id: text("developer"),
            developer_key_id: KeyId([2; crate::KEY_ID_LENGTH]),
            target_profile: text("f405"),
            amrn_format: 5,
            abi_version: 3,
            package_version: text("0.1.0"),
            minimum_kernel_version: text("0.1.0"),
            length: 64,
            sha256: digest(),
            required_services: 1,
            slot_id: 0,
        }; crate::MAX_TARGET_RECORDS],
        package_count: 1,
    };
    let length = encode_binary_targets_body(metadata, &mut buffer).expect("targets encode");
    assert!(parse_binary_targets_body(&buffer[..length - 1]).is_err());
}
