use std::{fs, path::PathBuf};

use dali_metadata::{
    BoundedText, MetadataHeader, MetadataRole, PublicKey, RevocationMetadata, RevocationReference,
    RoleDefinition, RoleKey, RootMetadata, Sha256Digest, SnapshotMetadata, TargetsMetadata,
    TargetsReference, TimestampMetadata,
};
use sha2::{Digest, Sha256};

use super::common;

const INITIAL_VERSION: u64 = 1;
const ROLE_COUNT: usize = 7;

pub fn run(arguments: &[String]) -> Result<(), String> {
    let output = PathBuf::from(common::required(arguments, common::OUTPUT_FLAG)?);
    let root_seed_path = PathBuf::from(common::required(arguments, common::ROOT_SIGNING_KEY_FLAG)?);
    let root_key_id = dali_metadata::KeyId(common::parse_hex::<16>(
        &common::required(arguments, common::ROOT_KEY_ID_FLAG)?,
        common::ROOT_KEY_ID_FLAG,
    )?);
    let bundle_seed_path = PathBuf::from(common::required(
        arguments,
        common::BUNDLE_SIGNING_KEY_FLAG,
    )?);
    let bundle_key_id = dali_metadata::KeyId(common::parse_hex::<16>(
        &common::required(arguments, common::BUNDLE_KEY_ID_FLAG)?,
        common::BUNDLE_KEY_ID_FLAG,
    )?);
    if output.exists() {
        return Err(format!("repository already exists: {}", output.display()));
    }
    let root_seed = common::read_seed(&root_seed_path)?;
    let bundle_seed = common::read_seed(&bundle_seed_path)?;
    let root_public = PublicKey(dali_crypto::public_key_from_seed(&root_seed));
    let bundle_public = PublicKey(dali_crypto::public_key_from_seed(&bundle_seed));
    let role_ids = role_ids(root_key_id, bundle_key_id)?;
    let root = root_metadata(
        root_key_id,
        bundle_key_id,
        root_public,
        bundle_public,
        role_ids,
    );

    let targets = TargetsMetadata {
        header: header(MetadataRole::Targets),
        delegations: [BoundedText::default(); dali_metadata::MAX_DELEGATION_SCOPES],
        delegation_count: 0,
        cartridges: [dali_metadata::TargetCartridge::default(); dali_metadata::MAX_TARGET_RECORDS],
        cartridge_count: 0,
    };
    let revocations = RevocationMetadata {
        header: header(MetadataRole::Revocation),
        records: [dali_metadata::RevocationRecord::default(); dali_metadata::MAX_REVOCATIONS],
        record_count: 0,
    };
    let targets_body = encode_targets(targets)?;
    let revocations_body = encode_revocations(revocations)?;
    let targets_envelope = common::sign_binary(
        MetadataRole::Targets,
        &targets_body,
        &root_seed,
        role_ids[3],
    )?;
    let revocations_envelope = common::sign_binary(
        MetadataRole::Revocation,
        &revocations_body,
        &root_seed,
        role_ids[5],
    )?;
    let snapshot = SnapshotMetadata {
        header: header(MetadataRole::Snapshot),
        targets: TargetsReference {
            version: targets.header.version,
            length: targets_envelope.len() as u32,
            sha256: digest(&targets_envelope),
        },
        revocations: RevocationReference {
            version: revocations.header.version,
            length: revocations_envelope.len() as u32,
            sha256: digest(&revocations_envelope),
        },
        delegations: [dali_metadata::DelegationReference::default();
            dali_metadata::MAX_SNAPSHOT_REFERENCES],
        delegation_count: 0,
    };
    let snapshot_body = encode_snapshot(snapshot)?;
    let snapshot_envelope = common::sign_binary(
        MetadataRole::Snapshot,
        &snapshot_body,
        &root_seed,
        role_ids[2],
    )?;
    let timestamp = TimestampMetadata {
        header: header(MetadataRole::Timestamp),
        snapshot_version: snapshot.header.version,
        snapshot_length: snapshot_envelope.len() as u32,
        snapshot_sha256: digest(&snapshot_envelope),
    };
    let timestamp_body = encode_timestamp(timestamp)?;
    let timestamp_envelope = common::sign_binary(
        MetadataRole::Timestamp,
        &timestamp_body,
        &root_seed,
        role_ids[1],
    )?;
    let root_body = encode_root(root)?;

    let metadata = output.join(common::METADATA_DIRECTORY);
    let delegations = metadata.join(common::DELEGATIONS_DIRECTORY);
    fs::create_dir_all(&delegations)
        .map_err(|error| format!("cannot create repository: {error}"))?;
    fs::create_dir_all(output.join("cartridges"))
        .map_err(|error| format!("cannot create cartridges directory: {error}"))?;
    common::write_new(
        &common::root_path(&output),
        &common::sign_binary(MetadataRole::Root, &root_body, &root_seed, role_ids[0])?,
    )?;
    common::write_new(&common::targets_path(&output), &targets_envelope)?;
    common::write_new(&common::revocations_path(&output), &revocations_envelope)?;
    common::write_new(&common::snapshot_path(&output), &snapshot_envelope)?;
    common::write_new(&common::timestamp_path(&output), &timestamp_envelope)?;
    println!(
        "Created Binary v2 repository metadata: {}",
        output.display()
    );
    println!("root key: {}", common::hex_encode(&root_key_id.0));
    println!("bundle key: {}", common::hex_encode(&bundle_key_id.0));
    Ok(())
}

fn role_ids(
    root: dali_metadata::KeyId,
    bundle: dali_metadata::KeyId,
) -> Result<[dali_metadata::KeyId; ROLE_COUNT], String> {
    let mut ids = [root; ROLE_COUNT];
    for id in ids.iter_mut().skip(1).take(5) {
        *id = common::random_key_id()?;
    }
    ids[6] = bundle;
    if ids[..6].windows(2).any(|pair| pair[0] == pair[1]) || ids[..6].contains(&bundle) {
        return Err("generated role key identifiers are not unique".to_owned());
    }
    Ok(ids)
}

fn root_metadata(
    root_id: dali_metadata::KeyId,
    bundle_id: dali_metadata::KeyId,
    root_public: PublicKey,
    bundle_public: PublicKey,
    ids: [dali_metadata::KeyId; ROLE_COUNT],
) -> RootMetadata {
    let roles = [
        role(MetadataRole::Root, ids[0]),
        role(MetadataRole::Timestamp, ids[1]),
        role(MetadataRole::Snapshot, ids[2]),
        role(MetadataRole::Targets, ids[3]),
        role(MetadataRole::Delegation, ids[4]),
        role(MetadataRole::Revocation, ids[5]),
        role(MetadataRole::Bundle, bundle_id),
    ];
    let mut keys = [RoleKey {
        role: MetadataRole::Root,
        key_id: root_id,
        public_key: root_public,
    }; dali_metadata::MAX_ROOT_KEYS];
    for (index, key) in keys.iter_mut().take(6).enumerate() {
        key.key_id = ids[index];
        key.role = roles[index].role;
    }
    keys[6] = RoleKey {
        role: MetadataRole::Bundle,
        key_id: bundle_id,
        public_key: bundle_public,
    };
    let placeholder = RoleDefinition {
        role: MetadataRole::Root,
        keys: [root_id; dali_metadata::MAX_ROLE_KEYS],
        key_count: 0,
        threshold: 0,
    };
    let mut role_table = [placeholder; dali_metadata::MAX_ROOT_ROLES];
    role_table[..ROLE_COUNT].copy_from_slice(&roles);
    RootMetadata {
        header: header(MetadataRole::Root),
        keys,
        key_count: ROLE_COUNT as u8,
        roles: role_table,
        role_count: ROLE_COUNT as u8,
    }
}

fn role(role: MetadataRole, key_id: dali_metadata::KeyId) -> RoleDefinition {
    RoleDefinition {
        role,
        keys: [key_id; dali_metadata::MAX_ROLE_KEYS],
        key_count: 1,
        threshold: 1,
    }
}

fn header(role: MetadataRole) -> MetadataHeader {
    MetadataHeader {
        role,
        version: INITIAL_VERSION,
        expires: 0,
    }
}

fn encode_root(metadata: RootMetadata) -> Result<Vec<u8>, String> {
    let mut output = vec![0; dali_metadata::MAX_ROOT_BYTES];
    let length = dali_metadata::encode_binary_root_body(metadata, &mut output)
        .map_err(|error| format!("cannot encode root: {error:?}"))?;
    output.truncate(length);
    Ok(output)
}

fn encode_targets(metadata: TargetsMetadata) -> Result<Vec<u8>, String> {
    let mut output = vec![0; dali_metadata::MAX_TARGETS_BYTES];
    let length = dali_metadata::encode_binary_targets_body(metadata, &mut output)
        .map_err(|error| format!("cannot encode targets: {error:?}"))?;
    output.truncate(length);
    Ok(output)
}

fn encode_revocations(metadata: RevocationMetadata) -> Result<Vec<u8>, String> {
    let mut output = vec![0; dali_metadata::MAX_REVOCATION_BYTES];
    let length = dali_metadata::encode_binary_revocation_body(metadata, &mut output)
        .map_err(|error| format!("cannot encode revocations: {error:?}"))?;
    output.truncate(length);
    Ok(output)
}

fn encode_snapshot(metadata: SnapshotMetadata) -> Result<Vec<u8>, String> {
    let mut output = vec![0; dali_metadata::MAX_SNAPSHOT_BYTES];
    let length = dali_metadata::encode_binary_snapshot_body(metadata, &mut output)
        .map_err(|error| format!("cannot encode snapshot: {error:?}"))?;
    output.truncate(length);
    Ok(output)
}

fn encode_timestamp(metadata: TimestampMetadata) -> Result<Vec<u8>, String> {
    let mut output = vec![0; dali_metadata::MAX_TIMESTAMP_BYTES];
    let length = dali_metadata::encode_binary_timestamp_body(metadata, &mut output)
        .map_err(|error| format!("cannot encode timestamp: {error:?}"))?;
    output.truncate(length);
    Ok(output)
}

fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest(Sha256::digest(bytes).into())
}
