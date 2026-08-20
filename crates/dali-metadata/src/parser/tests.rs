use super::*;
use crate::{
    BoundedText, KEY_ID_LENGTH, KeyId, MAX_DELEGATION_ID_BYTES, MAX_NAMESPACE_BYTES,
    MAX_PACKAGE_VERSION_BYTES, MAX_ROLE_KEYS, MAX_ROOT_BYTES, MAX_TARGET_PROFILE_BYTES,
    MAX_TARGETS_BYTES, MetadataHeader, MetadataRole, PUBLIC_KEY_LENGTH, PackageId, PublicKey,
    RoleDefinition, RoleKey, Sha256Digest, TargetPackage, encode_root_signed,
    encode_targets_signed, parse_snapshot_signed, parse_targets_signed, parse_timestamp_signed,
};

fn encoded_root() -> ([u8; MAX_ROOT_BYTES], usize) {
    let key = RoleKey {
        role: MetadataRole::Root,
        key_id: KeyId([1; KEY_ID_LENGTH]),
        public_key: PublicKey([2; PUBLIC_KEY_LENGTH]),
    };
    let mut keys = [KeyId([0; KEY_ID_LENGTH]); MAX_ROLE_KEYS];
    keys[0] = key.key_id;
    let role = RoleDefinition {
        role: MetadataRole::Targets,
        keys,
        key_count: 1,
        threshold: 1,
    };
    let mut output = [0; MAX_ROOT_BYTES];
    let length = encode_root_signed(
        &mut output,
        MetadataHeader {
            role: MetadataRole::Root,
            version: 1,
            expires: 0,
        },
        &[key],
        &[role],
    )
    .expect("root body should encode");
    (output, length)
}

#[test]
fn parses_the_canonical_root_body() {
    let (bytes, length) = encoded_root();
    let parsed = parse_root_signed(&bytes[..length]).expect("root body should parse");
    assert_eq!(parsed.header.version, 1);
    assert_eq!(parsed.key_count, 1);
    assert_eq!(parsed.role_count, 1);
    assert_eq!(parsed.keys[0].key_id, KeyId([1; KEY_ID_LENGTH]));
}

#[test]
fn rejects_trailing_bytes_and_whitespace() {
    let (bytes, length) = encoded_root();
    let mut extended = [0; MAX_ROOT_BYTES + 1];
    extended[..length].copy_from_slice(&bytes[..length]);
    extended[length] = b' ';
    assert_eq!(
        parse_root_signed(&extended[..length + 1]),
        Err(DecodeError::TrailingBytes)
    );
}

#[test]
fn rejects_a_non_canonical_field_order() {
    let input = br#"{"role":"root","expires":0,"keys":[],"roles":[],"schema":"dali.metadata.v1","version":1}"#;
    assert_eq!(
        parse_root_signed(input),
        Err(DecodeError::InvalidFieldOrder)
    );
}

#[test]
fn rejects_uppercase_hex() {
    let input = br#"{"expires":0,"keys":[{"key_id":"01010101010101010101010101010101","public_key":"0202020202020202020202020202020202020202020202020202020202020202","role":"targets"}],"role":"root","roles":[{"key_ids":["01010101010101010101010101010101"],"name":"targets","threshold":1}],"schema":"dali.metadata.v1","version":1}"#;
    let mut modified = [0; 512];
    modified[..input.len()].copy_from_slice(input);
    let hex_offset = input
        .windows(4)
        .position(|window| window == b"0101")
        .expect("key ID hex should exist");
    modified[hex_offset] = b'A';
    assert_eq!(
        parse_root_signed(&modified[..input.len()]),
        Err(DecodeError::InvalidHex)
    );
}

fn target_package() -> TargetPackage {
    TargetPackage {
        package_id: PackageId([1; KEY_ID_LENGTH]),
        namespace: BoundedText::<MAX_NAMESPACE_BYTES>::new("developer/app")
            .expect("test namespace fits"),
        developer_id: BoundedText::<{ crate::MAX_DEVELOPER_ID_BYTES }>::new("developer")
            .expect("test developer fits"),
        delegation_id: BoundedText::<MAX_DELEGATION_ID_BYTES>::new("developer")
            .expect("test delegation fits"),
        developer_key_id: KeyId([2; KEY_ID_LENGTH]),
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
fn parses_a_canonical_targets_body() {
    let delegation =
        BoundedText::<MAX_DELEGATION_ID_BYTES>::new("developer").expect("test delegation fits");
    let mut bytes = [0; MAX_TARGETS_BYTES];
    let length = encode_targets_signed(
        &mut bytes,
        MetadataHeader {
            role: MetadataRole::Targets,
            version: 1,
            expires: 0,
        },
        &[delegation],
        &[target_package()],
    )
    .expect("targets body should encode");
    let parsed = parse_targets_signed(&bytes[..length]).expect("targets body should parse");
    assert_eq!(parsed.header.role, MetadataRole::Targets);
    assert_eq!(parsed.package_count, 1);
    assert_eq!(parsed.packages[0].slot_id, 0);
    assert_eq!(parsed.packages[0].length, 128);
}

#[test]
fn rejects_non_canonical_targets_field_order() {
    let input = br#"{"expires":0,"delegations":[],"packages":[],"role":"targets","schema":"dali.metadata.v1","version":1}"#;
    assert_eq!(
        parse_targets_signed(input),
        Err(DecodeError::InvalidFieldOrder)
    );
}

#[test]
fn parses_canonical_snapshot_and_timestamp_bodies() {
    let snapshot = br#"{"expires":0,"metadata":[{"length":128,"role":"targets","sha256":"0303030303030303030303030303030303030303030303030303030303030303","version":1}],"role":"snapshot","schema":"dali.metadata.v1","version":1}"#;
    let parsed_snapshot = parse_snapshot_signed(snapshot).expect("snapshot should parse");
    assert_eq!(parsed_snapshot.targets.version, 1);
    assert_eq!(parsed_snapshot.delegation_count, 0);

    let timestamp = br#"{"expires":0,"role":"timestamp","schema":"dali.metadata.v1","snapshot":{"length":128,"sha256":"0303030303030303030303030303030303030303030303030303030303030303","version":1},"version":1}"#;
    let parsed_timestamp = parse_timestamp_signed(timestamp).expect("timestamp should parse");
    assert_eq!(parsed_timestamp.snapshot_length, 128);
    assert_eq!(parsed_timestamp.snapshot_version, 1);
}
