use super::descriptor::TARGET_BYTES_OFFSET;
use super::*;
use crate::{
    Ed25519Verifier, KeyId, MetadataHeader, PublicKey, RoleDefinition, RoleKey, Signature,
    SignatureRecord,
};

const SEEDS: [[u8; 32]; 3] = [[1; 32], [2; 32], [3; 32]];

fn target() -> BoundedText<{ crate::MAX_TARGET_PROFILE_BYTES }> {
    BoundedText::new("f405").expect("test target fits")
}

fn root() -> RootMetadata {
    let ids = [
        KeyId([1; crate::KEY_ID_LENGTH]),
        KeyId([2; crate::KEY_ID_LENGTH]),
        KeyId([3; crate::KEY_ID_LENGTH]),
    ];
    let mut keys = [RoleKey {
        role: MetadataRole::Root,
        key_id: ids[0],
        public_key: PublicKey(dali_crypto::public_key_from_seed(&SEEDS[0])),
    }; crate::MAX_ROOT_KEYS];
    for (index, key) in keys.iter_mut().take(ids.len()).enumerate() {
        key.key_id = ids[index];
        key.public_key = PublicKey(dali_crypto::public_key_from_seed(&SEEDS[index]));
    }
    let mut role_keys = [KeyId::default(); crate::MAX_ROLE_KEYS];
    role_keys[..ids.len()].copy_from_slice(&ids);
    let role = RoleDefinition {
        role: MetadataRole::Root,
        keys: role_keys,
        key_count: ids.len() as u8,
        threshold: PRODUCTION_ROOT_THRESHOLD,
    };
    let mut roles = [role; crate::MAX_ROOT_ROLES];
    roles[1].key_count = 0;
    roles[1].threshold = 0;
    RootMetadata {
        header: MetadataHeader {
            role: MetadataRole::Root,
            version: 1,
            expires: 0,
        },
        keys,
        key_count: ids.len() as u8,
        roles,
        role_count: 1,
    }
}

fn signed_image<'a>(image: &'a [u8], version: u64) -> SecureBootImage<'a> {
    let descriptor = KernelImageDescriptor::new(target(), version, image).expect("descriptor");
    let signed = descriptor.encode();
    let mut records = [SignatureRecord::default(); crate::MAX_SIGNATURES];
    for (index, seed) in SEEDS
        .iter()
        .take(usize::from(PRODUCTION_ROOT_THRESHOLD))
        .enumerate()
    {
        records[index] = SignatureRecord {
            key_id: KeyId([(index + 1) as u8; crate::KEY_ID_LENGTH]),
            signature: Signature(dali_crypto::sign(seed, &signed)),
        };
    }
    SecureBootImage {
        descriptor,
        image,
        signatures: SignatureSet {
            records,
            count: PRODUCTION_ROOT_THRESHOLD,
        },
    }
}

#[test]
fn production_custody_requires_three_distinct_root_keys() {
    let mut root = root();
    root.key_count = 1;
    root.roles[0].key_count = 1;
    root.roles[0].threshold = 1;
    assert_eq!(
        RootKeyCustodyPolicy::production().validate(&root),
        Err(RootKeyCustodyError::InsufficientRootKeys)
    );
}

#[test]
fn production_custody_rejects_duplicate_root_role_membership() {
    let mut root = root();
    root.roles[0].keys[1] = root.roles[0].keys[0];
    assert_eq!(
        RootKeyCustodyPolicy::production().validate(&root),
        Err(RootKeyCustodyError::InvalidRootThreshold)
    );
}

#[test]
fn secure_boot_accepts_root_signed_image_with_newer_version() {
    let image = b"kernel image";
    assert_eq!(
        verify_secure_boot_image(
            SecureBootContract::production(target()),
            &Ed25519Verifier,
            &root(),
            signed_image(image, 2),
            Some(1),
        ),
        Ok(())
    );
}

#[test]
fn secure_boot_rejects_target_digest_and_rollback_failures() {
    let image = b"kernel image";
    assert_eq!(
        verify_secure_boot_image(
            SecureBootContract::production(target()),
            &Ed25519Verifier,
            &root(),
            signed_image(image, 1),
            Some(1),
        ),
        Err(SecureBootError::Rollback)
    );
    let mut tampered = signed_image(image, 2);
    tampered.image = b"tampered img";
    assert_eq!(
        verify_secure_boot_image(
            SecureBootContract::production(target()),
            &Ed25519Verifier,
            &root(),
            tampered,
            Some(1),
        ),
        Err(SecureBootError::DigestMismatch)
    );
}

#[test]
fn secure_boot_rejects_an_oversized_signature_set_without_panicking() {
    let mut image = signed_image(b"kernel image", 2);
    image.signatures.count = (crate::MAX_SIGNATURES + 1) as u8;
    assert_eq!(
        verify_secure_boot_image(
            SecureBootContract::production(target()),
            &Ed25519Verifier,
            &root(),
            image,
            Some(1),
        ),
        Err(SecureBootError::Signature(
            VerificationError::InvalidSignatureSet
        ))
    );
}

#[test]
fn descriptor_round_trips_and_rejects_nonzero_padding() {
    let descriptor =
        KernelImageDescriptor::new(target(), 2, b"kernel image").expect("descriptor is valid");
    assert_eq!(
        KernelImageDescriptor::decode(&descriptor.encode()),
        Ok(descriptor)
    );
    let mut encoded = descriptor.encode();
    encoded[TARGET_BYTES_OFFSET + 8] = 1;
    assert_eq!(
        KernelImageDescriptor::decode(&encoded),
        Err(SecureBootDescriptorError::InvalidTarget)
    );
}
