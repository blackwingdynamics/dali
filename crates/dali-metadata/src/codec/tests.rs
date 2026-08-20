use super::*;
use crate::MAX_ROLE_KEYS;
use crate::{KeyId, MetadataHeader, MetadataRole, PublicKey, RoleDefinition, RoleKey};

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
