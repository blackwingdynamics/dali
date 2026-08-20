use super::*;
use crate::{
    KEY_ID_LENGTH, KeyId, MAX_ROLE_KEYS, MAX_ROOT_BYTES, MetadataHeader, MetadataRole,
    PUBLIC_KEY_LENGTH, PublicKey, RoleDefinition, RoleKey, encode_root_signed,
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
