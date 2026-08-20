use super::*;
use crate::MAX_ROLE_KEYS;
use crate::{
    BoundedText, DelegationReference, KeyId, MAX_DELEGATION_ID_BYTES, MAX_NAMESPACE_BYTES,
    MAX_PACKAGE_VERSION_BYTES, MAX_SIGNATURES, MAX_SNAPSHOT_REFERENCES, MAX_TARGET_PROFILE_BYTES,
    MAX_TARGETS_BYTES, MetadataHeader, MetadataRole, PackageId, PublicKey, RoleDefinition, RoleKey,
    Sha256Digest, Signature, SignatureRecord, SignatureSet, SnapshotMetadata, TargetPackage,
    TargetsReference, TimestampMetadata,
};

const KEY: RoleKey = RoleKey {
    role: MetadataRole::Root,
    key_id: KeyId([1; crate::KEY_ID_LENGTH]),
    public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
};

fn root_role(key_id: KeyId) -> RoleDefinition {
    let mut keys = [KeyId([0; crate::KEY_ID_LENGTH]); MAX_ROLE_KEYS];
    keys[0] = key_id;
    RoleDefinition {
        role: MetadataRole::Targets,
        keys,
        key_count: 1,
        threshold: 1,
    }
}

fn target_package() -> TargetPackage {
    TargetPackage {
        package_id: PackageId([1; crate::KEY_ID_LENGTH]),
        namespace: BoundedText::<MAX_NAMESPACE_BYTES>::new("developer/app")
            .expect("test namespace fits"),
        developer_id: BoundedText::<{ crate::MAX_DEVELOPER_ID_BYTES }>::new("developer")
            .expect("test developer fits"),
        delegation_id: BoundedText::<MAX_DELEGATION_ID_BYTES>::new("developer")
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
fn encodes_the_canonical_root_signed_body() {
    let mut output = [0; crate::MAX_ROOT_BYTES];
    let length = encode_root_signed(
        &mut output,
        MetadataHeader {
            role: MetadataRole::Root,
            version: 1,
            expires: 0,
        },
        &[KEY],
        &[root_role(KEY.key_id)],
    )
    .expect("root body should encode");
    assert_eq!(
        &output[..length],
        br#"{"expires":0,"keys":[{"key_id":"01010101010101010101010101010101","public_key":"0202020202020202020202020202020202020202020202020202020202020202","role":"root"}],"role":"root","roles":[{"key_ids":["01010101010101010101010101010101"],"name":"targets","threshold":1}],"schema":"dali.metadata.v1","version":1}"#
    );
}

#[test]
fn rejects_non_canonical_key_order() {
    let mut output = [0; crate::MAX_ROOT_BYTES];
    let second = RoleKey {
        key_id: KeyId([0; crate::KEY_ID_LENGTH]),
        ..KEY
    };
    assert_eq!(
        encode_root_signed(
            &mut output,
            MetadataHeader {
                role: MetadataRole::Root,
                version: 1,
                expires: 0,
            },
            &[KEY, second],
            &[root_role(KEY.key_id)],
        ),
        Err(EncodeError::NonCanonicalOrder)
    );
}

#[test]
fn rejects_an_output_buffer_that_is_too_small() {
    let mut output = [0; 8];
    assert_eq!(
        encode_root_signed(
            &mut output,
            MetadataHeader {
                role: MetadataRole::Root,
                version: 1,
                expires: 0,
            },
            &[KEY],
            &[root_role(KEY.key_id)],
        ),
        Err(EncodeError::BufferTooSmall)
    );
}

#[test]
fn encodes_targets_fields_in_canonical_order() {
    let delegation =
        BoundedText::<MAX_DELEGATION_ID_BYTES>::new("developer").expect("test delegation fits");
    let mut output = [0; MAX_TARGETS_BYTES];
    let length = encode_targets_signed(
        &mut output,
        MetadataHeader {
            role: MetadataRole::Targets,
            version: 1,
            expires: 0,
        },
        &[delegation],
        &[target_package()],
    )
    .expect("targets body should encode");
    let body = &output[..length];
    let fields: &[&[u8]] = &[
        b"\"delegations\"",
        b"\"expires\"",
        b"\"packages\"",
        b"\"role\"",
        b"\"schema\"",
        b"\"version\"",
    ];
    let positions: [usize; 6] = core::array::from_fn(|index| {
        let field = fields[index];
        body.windows(field.len())
            .position(|window| window == field)
            .expect("field should be present")
    });
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(body.windows(13).any(|window| window == br#""package_id":"#));
}

fn snapshot_metadata() -> SnapshotMetadata {
    let id =
        BoundedText::<MAX_DELEGATION_ID_BYTES>::new("developer").expect("test delegation fits");
    let reference = DelegationReference {
        id,
        version: 1,
        length: 64,
        sha256: Sha256Digest([4; crate::SHA256_LENGTH]),
    };
    SnapshotMetadata {
        header: MetadataHeader {
            role: MetadataRole::Snapshot,
            version: 1,
            expires: 0,
        },
        targets: TargetsReference {
            version: 1,
            length: 128,
            sha256: Sha256Digest([3; crate::SHA256_LENGTH]),
        },
        delegations: [reference; MAX_SNAPSHOT_REFERENCES],
        delegation_count: 1,
    }
}

#[test]
fn encodes_snapshot_and_timestamp_bodies() {
    let mut snapshot = [0; crate::MAX_SNAPSHOT_BYTES];
    let snapshot_length = encode_snapshot_signed(&mut snapshot, snapshot_metadata())
        .expect("snapshot body should encode");
    assert!(
        snapshot[..snapshot_length]
            .windows(12)
            .any(|window| window == b"\"metadata\":[")
    );

    let mut timestamp = [0; crate::MAX_TIMESTAMP_BYTES];
    let timestamp_metadata = TimestampMetadata {
        header: MetadataHeader {
            role: MetadataRole::Timestamp,
            version: 1,
            expires: 0,
        },
        snapshot_version: 1,
        snapshot_length: 128,
        snapshot_sha256: Sha256Digest([3; crate::SHA256_LENGTH]),
    };
    let timestamp_length = encode_timestamp_signed(&mut timestamp, timestamp_metadata)
        .expect("timestamp body should encode");
    assert!(
        timestamp[..timestamp_length]
            .windows(12)
            .any(|window| window == b"\"snapshot\":{")
    );
}

#[test]
fn encodes_signature_records_in_key_order() {
    let mut records = [SignatureRecord::default(); MAX_SIGNATURES];
    records[0] = SignatureRecord {
        key_id: KeyId([1; crate::KEY_ID_LENGTH]),
        signature: Signature([2; crate::SIGNATURE_LENGTH]),
    };
    let mut output = [0; crate::MAX_ROOT_BYTES];
    let length = encode_signature_list(&mut output, SignatureSet { records, count: 1 })
        .expect("signature list should encode");
    assert!(
        output[..length]
            .starts_with(br#"[{"key_id":"01010101010101010101010101010101","signature":""#)
    );
    assert!(output[..length].ends_with(br#""}]"#));
    assert_eq!(length, 190);
}
