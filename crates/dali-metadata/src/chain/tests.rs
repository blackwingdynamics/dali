extern crate std;

use super::*;
use crate::{
    BoundedText, DelegationReference, KeyId, MAX_DELEGATION_ABIS, MAX_DELEGATION_SCOPES,
    MAX_DELEGATION_TARGETS, MAX_ROLE_KEYS, MAX_SIGNATURES, MAX_SNAPSHOT_REFERENCES,
    MAX_TARGETS_BYTES, MetadataHeader, RevocationReference, RoleDefinition, RoleKey, Signature,
    SignatureRecord, SignatureSet, TargetsReference, encode_delegation_signed,
    encode_revocation_signed, encode_root_signed, encode_snapshot_signed, encode_targets_signed,
    encode_timestamp_signed,
};
use dali_amrn::{v3, v4, v5};
use std::vec::Vec;

const CONTRACT: v3::Contract = v3::Contract {
    target_id: 2,
    code_load_address: 0x2001_0000,
    code_capacity: 0x4000,
    data_load_address: 0x2001_4000,
    data_capacity: 0x4000,
};

const ROOT_SEED: [u8; 32] = [1; 32];
const TIMESTAMP_SEED: [u8; 32] = [2; 32];
const SNAPSHOT_SEED: [u8; 32] = [3; 32];
const TARGETS_SEED: [u8; 32] = [4; 32];
const DELEGATION_SEED: [u8; 32] = [5; 32];
const REVOCATION_SEED: [u8; 32] = [6; 32];
const DEVELOPER_SEED: [u8; 32] = [9; 32];

struct Fixture {
    root: Vec<u8>,
    timestamp: Vec<u8>,
    snapshot: Vec<u8>,
    targets: Vec<u8>,
    revocations: Vec<u8>,
    delegation: Vec<u8>,
    cartridge: Vec<u8>,
    cartridge_id: CartridgeId,
}

impl Fixture {
    fn documents(&self) -> RepositoryCartridgeDocuments<'_> {
        RepositoryCartridgeDocuments {
            root: envelope(&self.root, key_id(1), ROOT_SEED),
            timestamp: envelope(&self.timestamp, key_id(2), TIMESTAMP_SEED),
            snapshot: envelope(&self.snapshot, key_id(3), SNAPSHOT_SEED),
            targets: envelope(&self.targets, key_id(4), TARGETS_SEED),
            revocations: envelope(&self.revocations, key_id(6), REVOCATION_SEED),
            delegation: envelope(&self.delegation, key_id(5), DELEGATION_SEED),
            cartridge: &self.cartridge,
        }
    }
}

fn key_id(value: u8) -> KeyId {
    KeyId([value; crate::KEY_ID_LENGTH])
}
fn text<const N: usize>(value: &str) -> BoundedText<N> {
    BoundedText::new(value).expect("fixture text fits")
}
fn digest(bytes: &[u8]) -> crate::Sha256Digest {
    use sha2::{Digest, Sha256};
    crate::Sha256Digest(Sha256::digest(bytes).into())
}

fn envelope<'a>(body: &'a [u8], key: KeyId, seed: [u8; 32]) -> SignedEnvelope<'a> {
    let mut records = [SignatureRecord::default(); MAX_SIGNATURES];
    records[0] = SignatureRecord {
        key_id: key,
        signature: Signature(dali_crypto::sign(&seed, body)),
    };
    SignedEnvelope {
        signed: body,
        signatures: SignatureSet { records, count: 1 },
    }
}

fn role(role: MetadataRole, key: KeyId) -> RoleDefinition {
    let mut keys = [KeyId([0; crate::KEY_ID_LENGTH]); MAX_ROLE_KEYS];
    keys[0] = key;
    RoleDefinition {
        role,
        keys,
        key_count: 1,
        threshold: 1,
    }
}

fn fixture(revoked: bool) -> Fixture {
    let cartridge_id = CartridgeId([0xAA; crate::KEY_ID_LENGTH]);
    let mut unsigned = std::vec![0; v5::HEADER_SIZE + 4];
    let image = v5::Image {
        image: v3::Image {
            code: &[0, 0, 0, 0],
            initialized_data: &[],
            data_zero_size: 0,
            stack_size: 0x1000,
            linked_code_base: 0x2000_8000,
            linked_data_base: 0x2000_C000,
            execution_offset: 0,
            relocations: &[],
        },
        metadata: v4::Metadata {
            cartridge_id: cartridge_id.0,
            cartridge_version: v4::Version {
                major: 1,
                minor: 0,
                patch: 0,
            },
            minimum_kernel_version: v4::Version {
                major: 0,
                minor: 1,
                patch: 0,
            },
            required_services: 1,
            slot_id: 1,
        },
    };
    let unsigned_len = v5::encode_unsigned(image, CONTRACT, &mut unsigned).expect("AMRN encodes");
    unsigned.truncate(unsigned_len);
    let mut cartridge = std::vec![0; unsigned_len + v5::SIGNATURE_SIZE];
    v5::append_signature(
        &unsigned,
        &key_id(9).0,
        &dali_crypto::sign(&DEVELOPER_SEED, &unsigned),
        &mut cartridge,
    )
    .expect("AMRN signs");
    cartridge.truncate(unsigned_len + v5::SIGNATURE_SIZE);

    let delegation = delegation_body();
    let revocations = revocation_body(revoked);
    let target = target_record(cartridge_id, &cartridge);
    let targets = targets_body(target);
    let snapshot = snapshot_body(&targets, &revocations, &delegation);
    let timestamp = timestamp_body(&snapshot);
    let root = root_body();
    Fixture {
        root,
        timestamp,
        snapshot,
        targets,
        revocations,
        delegation,
        cartridge,
        cartridge_id,
    }
}

fn root_body() -> Vec<u8> {
    let roles = [
        role(MetadataRole::Delegation, key_id(5)),
        role(MetadataRole::Revocation, key_id(6)),
        role(MetadataRole::Root, key_id(1)),
        role(MetadataRole::Snapshot, key_id(3)),
        role(MetadataRole::Targets, key_id(4)),
        role(MetadataRole::Timestamp, key_id(2)),
    ];
    let keys = [
        RoleKey {
            role: MetadataRole::Root,
            key_id: key_id(1),
            public_key: crate::PublicKey(dali_crypto::public_key_from_seed(&ROOT_SEED)),
        },
        RoleKey {
            role: MetadataRole::Timestamp,
            key_id: key_id(2),
            public_key: crate::PublicKey(dali_crypto::public_key_from_seed(&TIMESTAMP_SEED)),
        },
        RoleKey {
            role: MetadataRole::Snapshot,
            key_id: key_id(3),
            public_key: crate::PublicKey(dali_crypto::public_key_from_seed(&SNAPSHOT_SEED)),
        },
        RoleKey {
            role: MetadataRole::Targets,
            key_id: key_id(4),
            public_key: crate::PublicKey(dali_crypto::public_key_from_seed(&TARGETS_SEED)),
        },
        RoleKey {
            role: MetadataRole::Delegation,
            key_id: key_id(5),
            public_key: crate::PublicKey(dali_crypto::public_key_from_seed(&DELEGATION_SEED)),
        },
        RoleKey {
            role: MetadataRole::Revocation,
            key_id: key_id(6),
            public_key: crate::PublicKey(dali_crypto::public_key_from_seed(&REVOCATION_SEED)),
        },
    ];
    let mut output = std::vec![0; crate::MAX_ROOT_BYTES];
    let length = encode_root_signed(
        &mut output,
        MetadataHeader {
            role: MetadataRole::Root,
            version: 1,
            expires: 0,
        },
        &keys,
        &roles,
    )
    .expect("root encodes");
    output.truncate(length);
    output
}

fn delegation_body() -> Vec<u8> {
    let mut namespaces = [BoundedText::default(); MAX_DELEGATION_SCOPES];
    namespaces[0] = text("developer/app");
    let mut targets = [BoundedText::default(); MAX_DELEGATION_TARGETS];
    targets[0] = text("f405");
    let mut abis = [0; MAX_DELEGATION_ABIS];
    abis[0] = 3;
    let metadata = DelegationMetadata {
        header: MetadataHeader {
            role: MetadataRole::Delegation,
            version: 1,
            expires: 0,
        },
        developer_id: text("developer"),
        key_id: key_id(9),
        public_key: crate::PublicKey(dali_crypto::public_key_from_seed(&DEVELOPER_SEED)),
        allowed_namespaces: namespaces,
        namespace_count: 1,
        allowed_targets: targets,
        target_count: 1,
        allowed_abis: abis,
        abi_count: 1,
        not_before: 0,
        not_after: 0,
    };
    let mut output = std::vec![0; crate::MAX_DELEGATION_BYTES];
    let length = encode_delegation_signed(&mut output, metadata).expect("delegation encodes");
    output.truncate(length);
    output
}

fn revocation_body(revoked: bool) -> Vec<u8> {
    let mut records = [crate::RevocationRecord::default(); crate::MAX_REVOCATIONS];
    if revoked {
        records[0] = crate::RevocationRecord {
            developer_id: text("developer"),
            effective_version: 1,
            issuer_key_id: key_id(6),
            key_id: key_id(9),
            reason: text("compromised"),
        };
    }
    let metadata = RevocationMetadata {
        header: MetadataHeader {
            role: MetadataRole::Revocation,
            version: 1,
            expires: 0,
        },
        records,
        record_count: u8::from(revoked),
    };
    let mut output = std::vec![0; crate::MAX_REVOCATION_BYTES];
    let length = encode_revocation_signed(&mut output, metadata).expect("revocation encodes");
    output.truncate(length);
    output
}

fn target_record(cartridge_id: CartridgeId, cartridge: &[u8]) -> TargetCartridge {
    TargetCartridge {
        cartridge_id,
        namespace: text("developer/app"),
        developer_id: text("developer"),
        delegation_id: text("delegation-1"),
        developer_key_id: key_id(9),
        target_profile: text("f405"),
        amrn_format: 5,
        abi_version: 3,
        cartridge_version: text("1.0.0"),
        minimum_kernel_version: text("0.1.0"),
        length: cartridge.len() as u32,
        sha256: digest(cartridge),
        required_services: 1,
        slot_id: 1,
    }
}

fn targets_body(target: TargetCartridge) -> Vec<u8> {
    let mut delegations = [BoundedText::default(); MAX_DELEGATION_SCOPES];
    delegations[0] = text("delegation-1");
    let mut cartridges = [TargetCartridge::default(); crate::MAX_TARGET_RECORDS];
    cartridges[0] = target;
    let metadata = TargetsMetadata {
        header: MetadataHeader {
            role: MetadataRole::Targets,
            version: 1,
            expires: 0,
        },
        delegations,
        delegation_count: 1,
        cartridges,
        cartridge_count: 1,
    };
    let mut output = std::vec![0; MAX_TARGETS_BYTES];
    let length = encode_targets_signed(
        &mut output,
        metadata.header,
        &metadata.delegations[..usize::from(metadata.delegation_count)],
        &metadata.cartridges[..usize::from(metadata.cartridge_count)],
    )
    .expect("targets encodes");
    output.truncate(length);
    output
}

fn snapshot_body(targets: &[u8], revocations: &[u8], delegation: &[u8]) -> Vec<u8> {
    let mut delegations = [DelegationReference::default(); MAX_SNAPSHOT_REFERENCES];
    delegations[0] = DelegationReference {
        id: text("delegation-1"),
        version: 1,
        length: delegation.len() as u32,
        sha256: digest(delegation),
    };
    let metadata = SnapshotMetadata {
        header: MetadataHeader {
            role: MetadataRole::Snapshot,
            version: 1,
            expires: 0,
        },
        targets: TargetsReference {
            version: 1,
            length: targets.len() as u32,
            sha256: digest(targets),
        },
        revocations: RevocationReference {
            version: 1,
            length: revocations.len() as u32,
            sha256: digest(revocations),
        },
        delegations,
        delegation_count: 1,
    };
    let mut output = std::vec![0; crate::MAX_SNAPSHOT_BYTES];
    let length = encode_snapshot_signed(&mut output, metadata).expect("snapshot encodes");
    output.truncate(length);
    output
}

fn timestamp_body(snapshot: &[u8]) -> Vec<u8> {
    let metadata = TimestampMetadata {
        header: MetadataHeader {
            role: MetadataRole::Timestamp,
            version: 1,
            expires: 0,
        },
        snapshot_version: 1,
        snapshot_length: snapshot.len() as u32,
        snapshot_sha256: digest(snapshot),
    };
    let mut output = std::vec![0; crate::MAX_TIMESTAMP_BYTES];
    let length = encode_timestamp_signed(&mut output, metadata).expect("timestamp encodes");
    output.truncate(length);
    output
}

#[test]
fn verifies_the_complete_real_ed25519_chain() {
    let fixture = fixture(false);
    let verified =
        verify_repository_cartridge(fixture.documents(), fixture.cartridge_id, CONTRACT, None)
            .expect("chain verifies");
    assert_eq!(verified.target.cartridge_id, fixture.cartridge_id);
}

#[test]
fn rejects_a_tampered_metadata_signature() {
    let fixture = fixture(false);
    let mut documents = fixture.documents();
    documents.targets.signatures.records[0].signature.0[0] ^= 1;
    assert_eq!(
        verify_repository_cartridge(documents, fixture.cartridge_id, CONTRACT, None),
        Err(ChainVerificationError::Signature)
    );
}

#[test]
fn rejects_a_revoked_developer_key() {
    let fixture = fixture(true);
    assert_eq!(
        verify_repository_cartridge(fixture.documents(), fixture.cartridge_id, CONTRACT, None),
        Err(ChainVerificationError::Authorization)
    );
}

#[test]
fn rejects_a_tampered_amrn_artifact() {
    let mut fixture = fixture(false);
    fixture.cartridge[v5::HEADER_SIZE] ^= 1;
    assert_eq!(
        verify_repository_cartridge(fixture.documents(), fixture.cartridge_id, CONTRACT, None,),
        Err(ChainVerificationError::InvalidAmrn)
    );
}
