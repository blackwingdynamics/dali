use std::path::PathBuf;

use dali_metadata::{
    BoundedText, DelegationMetadata, DelegationReference, MetadataHeader, MetadataRole, PublicKey,
    RevocationMetadata, RevocationReference, Sha256Digest, SnapshotMetadata, TargetsMetadata,
    TargetsReference, parse_binary_revocation_body, parse_binary_root_body,
    parse_binary_snapshot_body, parse_binary_targets_body,
};
use sha2::{Digest, Sha256};

use super::common;

pub fn run(arguments: &[String]) -> Result<(), String> {
    let root = PathBuf::from(common::required(arguments, common::INPUT_FLAG)?);
    let signing_key = common::read_seed(&PathBuf::from(common::required(
        arguments,
        common::SIGNING_KEY_FLAG,
    )?))?;
    let developer_id = common::bounded::<{ dali_metadata::MAX_DEVELOPER_ID_BYTES }>(
        &common::required(arguments, common::DEVELOPER_ID_FLAG)?,
        common::DEVELOPER_ID_FLAG,
    )?;
    let developer_key_id = dali_metadata::KeyId(common::parse_hex::<16>(
        &common::required(arguments, common::DEVELOPER_KEY_ID_FLAG)?,
        common::DEVELOPER_KEY_ID_FLAG,
    )?);
    let public_key = PublicKey(common::parse_hex::<32>(
        &common::required(arguments, common::DEVELOPER_PUBLIC_KEY_FLAG)?,
        common::DEVELOPER_PUBLIC_KEY_FLAG,
    )?);
    let delegation_id = common::bounded::<{ dali_metadata::MAX_DELEGATION_ID_BYTES }>(
        &common::required(arguments, common::DELEGATION_ID_FLAG)?,
        common::DELEGATION_ID_FLAG,
    )?;
    let namespace = common::bounded::<{ dali_metadata::MAX_NAMESPACE_BYTES }>(
        &common::required(arguments, common::NAMESPACE_FLAG)?,
        common::NAMESPACE_FLAG,
    )?;
    let target = common::bounded::<{ dali_metadata::MAX_TARGET_PROFILE_BYTES }>(
        &common::required(arguments, common::TARGET_FLAG)?,
        common::TARGET_FLAG,
    )?;
    let abi = common::parse_u16(
        &common::required(arguments, common::ABI_FLAG)?,
        common::ABI_FLAG,
    )?;
    let (root_bytes, root_envelope) =
        common::read_binary(&common::root_path(&root), MetadataRole::Root)?;
    let root_metadata = parse_binary_root_body(root_envelope.body)
        .map_err(|error| format!("invalid root body: {error:?}"))?;
    let delegation_path = common::delegation_path(
        &root,
        delegation_id
            .as_str()
            .ok_or_else(|| "delegation ID is invalid".to_owned())?,
    );
    if delegation_path.exists() {
        return Err(format!(
            "delegation already exists: {}",
            delegation_path.display()
        ));
    }
    let (_, targets_envelope) =
        common::read_binary(&common::targets_path(&root), MetadataRole::Targets)?;
    let targets = parse_binary_targets_body(targets_envelope.body)
        .map_err(|error| format!("invalid targets body: {error:?}"))?;
    if targets.delegations[..usize::from(targets.delegation_count)].contains(&delegation_id) {
        return Err("targets already references this delegation".to_owned());
    }
    let (revocations_bytes, revocations_envelope) =
        common::read_binary(&common::revocations_path(&root), MetadataRole::Revocation)?;
    let revocations = parse_binary_revocation_body(revocations_envelope.body)
        .map_err(|error| format!("invalid revocations body: {error:?}"))?;
    let (_, snapshot_envelope) =
        common::read_binary(&common::snapshot_path(&root), MetadataRole::Snapshot)?;
    let snapshot = parse_binary_snapshot_body(snapshot_envelope.body)
        .map_err(|error| format!("invalid snapshot body: {error:?}"))?;

    let delegation = delegation_metadata(
        DeveloperAuthorization {
            developer_id,
            key_id: developer_key_id,
            public_key,
            namespace,
            target,
            abi,
        },
        1,
    );
    let delegation_body = encode_delegation(delegation)?;
    let delegation_signer = common::role_key(&root_metadata, MetadataRole::Delegation)?;
    let delegation_envelope = common::sign_binary(
        MetadataRole::Delegation,
        &delegation_body,
        &signing_key,
        delegation_signer,
    )?;

    let updated_targets = append_delegation(targets, delegation_id)?;
    let targets_body = encode_targets(updated_targets)?;
    let targets_signer = common::role_key(&root_metadata, MetadataRole::Targets)?;
    let targets_envelope = common::sign_binary(
        MetadataRole::Targets,
        &targets_body,
        &signing_key,
        targets_signer,
    )?;
    let updated_snapshot = update_snapshot(
        snapshot,
        updated_targets.header.version,
        &targets_envelope,
        &revocations,
        &revocations_bytes,
        delegation_id,
        &delegation_envelope,
    )?;
    let snapshot_body = encode_snapshot(updated_snapshot)?;
    let snapshot_signer = common::role_key(&root_metadata, MetadataRole::Snapshot)?;
    let snapshot_envelope = common::sign_binary(
        MetadataRole::Snapshot,
        &snapshot_body,
        &signing_key,
        snapshot_signer,
    )?;
    let timestamp = dali_metadata::TimestampMetadata {
        header: MetadataHeader {
            role: MetadataRole::Timestamp,
            version: updated_snapshot.header.version,
            expires: 0,
        },
        snapshot_version: updated_snapshot.header.version,
        snapshot_length: snapshot_envelope.len() as u32,
        snapshot_sha256: digest(&snapshot_envelope),
    };
    let timestamp_body = encode_timestamp(timestamp)?;
    let timestamp_signer = common::role_key(&root_metadata, MetadataRole::Timestamp)?;
    let timestamp_envelope = common::sign_binary(
        MetadataRole::Timestamp,
        &timestamp_body,
        &signing_key,
        timestamp_signer,
    )?;

    common::write_new(&delegation_path, &delegation_envelope)?;
    common::write_replace(&common::targets_path(&root), &targets_envelope)?;
    common::write_replace(&common::snapshot_path(&root), &snapshot_envelope)?;
    common::write_replace(&common::timestamp_path(&root), &timestamp_envelope)?;
    let _ = root_bytes;
    println!(
        "Added developer delegation: {}",
        delegation_id.as_str().unwrap_or("<invalid>")
    );
    println!("developer key: {}", common::hex_encode(&developer_key_id.0));
    Ok(())
}

struct DeveloperAuthorization {
    developer_id: BoundedText<{ dali_metadata::MAX_DEVELOPER_ID_BYTES }>,
    key_id: dali_metadata::KeyId,
    public_key: PublicKey,
    namespace: BoundedText<{ dali_metadata::MAX_NAMESPACE_BYTES }>,
    target: BoundedText<{ dali_metadata::MAX_TARGET_PROFILE_BYTES }>,
    abi: u16,
}

fn delegation_metadata(request: DeveloperAuthorization, version: u64) -> DelegationMetadata {
    let mut namespaces = [BoundedText::default(); dali_metadata::MAX_DELEGATION_SCOPES];
    namespaces[0] = request.namespace;
    let mut targets = [BoundedText::default(); dali_metadata::MAX_DELEGATION_TARGETS];
    targets[0] = request.target;
    let mut abis = [0; dali_metadata::MAX_DELEGATION_ABIS];
    abis[0] = request.abi;
    DelegationMetadata {
        header: MetadataHeader {
            role: MetadataRole::Delegation,
            version,
            expires: 0,
        },
        developer_id: request.developer_id,
        key_id: request.key_id,
        public_key: request.public_key,
        allowed_namespaces: namespaces,
        namespace_count: 1,
        allowed_targets: targets,
        target_count: 1,
        allowed_abis: abis,
        abi_count: 1,
        not_before: 0,
        not_after: 0,
    }
}

fn append_delegation(
    mut metadata: TargetsMetadata,
    id: BoundedText<{ dali_metadata::MAX_DELEGATION_ID_BYTES }>,
) -> Result<TargetsMetadata, String> {
    let count = usize::from(metadata.delegation_count);
    if count >= dali_metadata::MAX_DELEGATION_SCOPES {
        return Err("targets delegation capacity is exhausted".to_owned());
    }
    metadata.delegations[count] = id;
    metadata.delegation_count += 1;
    let active = &mut metadata.delegations[..usize::from(metadata.delegation_count)];
    active.sort_by(|left, right| {
        left.as_str()
            .unwrap_or("")
            .cmp(right.as_str().unwrap_or(""))
    });
    metadata.header.version += 1;
    Ok(metadata)
}

fn update_snapshot(
    mut snapshot: SnapshotMetadata,
    targets_version: u64,
    targets_envelope: &[u8],
    revocations: &RevocationMetadata,
    revocations_envelope: &[u8],
    delegation_id: BoundedText<{ dali_metadata::MAX_DELEGATION_ID_BYTES }>,
    delegation_envelope: &[u8],
) -> Result<SnapshotMetadata, String> {
    let count = usize::from(snapshot.delegation_count);
    if count >= dali_metadata::MAX_SNAPSHOT_REFERENCES {
        return Err("snapshot delegation capacity is exhausted".to_owned());
    }
    snapshot.header.version += 1;
    snapshot.targets = TargetsReference {
        version: targets_version,
        length: targets_envelope.len() as u32,
        sha256: digest(targets_envelope),
    };
    snapshot.revocations = RevocationReference {
        version: revocations.header.version,
        length: revocations_envelope.len() as u32,
        sha256: digest(revocations_envelope),
    };
    snapshot.delegations[count] = DelegationReference {
        id: delegation_id,
        version: 1,
        length: delegation_envelope.len() as u32,
        sha256: digest(delegation_envelope),
    };
    snapshot.delegation_count += 1;
    let active = &mut snapshot.delegations[..usize::from(snapshot.delegation_count)];
    active.sort_by(|left, right| {
        left.id
            .as_str()
            .unwrap_or("")
            .cmp(right.id.as_str().unwrap_or(""))
    });
    Ok(snapshot)
}

fn encode_delegation(metadata: DelegationMetadata) -> Result<Vec<u8>, String> {
    let mut output = vec![0; dali_metadata::MAX_DELEGATION_BYTES];
    let length = dali_metadata::encode_binary_delegation_body(metadata, &mut output)
        .map_err(|error| format!("cannot encode delegation: {error:?}"))?;
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

fn encode_snapshot(metadata: SnapshotMetadata) -> Result<Vec<u8>, String> {
    let mut output = vec![0; dali_metadata::MAX_SNAPSHOT_BYTES];
    let length = dali_metadata::encode_binary_snapshot_body(metadata, &mut output)
        .map_err(|error| format!("cannot encode snapshot: {error:?}"))?;
    output.truncate(length);
    Ok(output)
}

fn encode_timestamp(metadata: dali_metadata::TimestampMetadata) -> Result<Vec<u8>, String> {
    let mut output = vec![0; dali_metadata::MAX_TIMESTAMP_BYTES];
    let length = dali_metadata::encode_binary_timestamp_body(metadata, &mut output)
        .map_err(|error| format!("cannot encode timestamp: {error:?}"))?;
    output.truncate(length);
    Ok(output)
}

fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest(Sha256::digest(bytes).into())
}
