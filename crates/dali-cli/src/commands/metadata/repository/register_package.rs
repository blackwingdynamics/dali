use std::{fs, path::PathBuf};

use dali_metadata::{
    BoundedText, DelegationMetadata, MetadataHeader, MetadataRole, PackageId, Sha256Digest,
    SnapshotMetadata, TargetPackage, TargetsMetadata, TargetsReference, TimestampMetadata,
    parse_binary_delegation_body, parse_binary_revocation_body, parse_binary_root_body,
    parse_binary_snapshot_body, parse_binary_targets_body,
};
use sha2::{Digest, Sha256};

use super::common;
use super::manifest::{self, PackageManifest};

pub fn run(arguments: &[String]) -> Result<(), String> {
    let repository = PathBuf::from(common::required(arguments, common::INPUT_FLAG)?);
    let package_path = PathBuf::from(common::required(arguments, common::PACKAGE_FLAG)?);
    let manifest_path = PathBuf::from(common::required(arguments, common::MANIFEST_FLAG)?);
    let delegation_id = common::bounded::<{ dali_metadata::MAX_DELEGATION_ID_BYTES }>(
        &common::required(arguments, common::DELEGATION_ID_FLAG)?,
        common::DELEGATION_ID_FLAG,
    )?;
    let namespace = common::bounded::<{ dali_metadata::MAX_NAMESPACE_BYTES }>(
        &common::required(arguments, common::NAMESPACE_FLAG)?,
        common::NAMESPACE_FLAG,
    )?;
    let signing_key = common::read_seed(&PathBuf::from(common::required(
        arguments,
        common::SIGNING_KEY_FLAG,
    )?))?;
    let manifest = manifest::parse(&manifest_path)?;
    let package_bytes = fs::read(&package_path)
        .map_err(|error| format!("cannot read package {}: {error}", package_path.display()))?;
    let digest = Sha256Digest(Sha256::digest(&package_bytes).into());
    verify_stored_package(&repository, &digest, &package_bytes)?;
    let package = parse_package(&package_bytes, &manifest)?;
    validate_manifest_package(&manifest, &package)?;

    let (_, root_envelope) =
        common::read_binary(&common::root_path(&repository), MetadataRole::Root)?;
    let root = parse_binary_root_body(root_envelope.body)
        .map_err(|error| format!("invalid root body: {error:?}"))?;
    let (_, targets_envelope) =
        common::read_binary(&common::targets_path(&repository), MetadataRole::Targets)?;
    let targets = parse_binary_targets_body(targets_envelope.body)
        .map_err(|error| format!("invalid targets body: {error:?}"))?;
    let (revocations_bytes, revocations_envelope) = common::read_binary(
        &common::revocations_path(&repository),
        MetadataRole::Revocation,
    )?;
    let _revocations = parse_binary_revocation_body(revocations_envelope.body)
        .map_err(|error| format!("invalid revocations body: {error:?}"))?;
    let (_, snapshot_envelope) =
        common::read_binary(&common::snapshot_path(&repository), MetadataRole::Snapshot)?;
    let snapshot = parse_binary_snapshot_body(snapshot_envelope.body)
        .map_err(|error| format!("invalid snapshot body: {error:?}"))?;
    let delegation_path = common::delegation_path(
        &repository,
        delegation_id
            .as_str()
            .ok_or_else(|| "delegation ID is invalid".to_owned())?,
    );
    let (_, delegation_envelope) = common::read_binary(&delegation_path, MetadataRole::Delegation)?;
    let delegation = parse_binary_delegation_body(delegation_envelope.body)
        .map_err(|error| format!("invalid delegation body: {error:?}"))?;
    authorize_package(&targets, &delegation, delegation_id, namespace, &manifest)?;

    let target_package = TargetPackage {
        package_id: PackageId(manifest.package_id),
        namespace,
        developer_id: delegation.developer_id,
        delegation_id,
        developer_key_id: manifest.developer_key_id,
        target_profile: manifest.target_profile,
        amrn_format: u16::from(dali_amrn::v5::FORMAT_VERSION),
        abi_version: u16::from(dali_amrn::v3::ABI_VERSION),
        package_version: manifest.package_version,
        minimum_kernel_version: manifest.minimum_kernel_version,
        length: u32::try_from(package_bytes.len())
            .map_err(|_| "package is larger than the targets length field".to_owned())?,
        sha256: digest,
        required_services: manifest.required_services,
        slot_id: manifest.slot_id,
    };
    let updated_targets = append_package(targets, target_package)?;
    let targets_body = encode_targets(updated_targets)?;
    let targets_signer = common::role_key(&root, MetadataRole::Targets)?;
    let targets_envelope = common::sign_binary(
        MetadataRole::Targets,
        &targets_body,
        &signing_key,
        targets_signer,
    )?;
    let updated_snapshot = update_snapshot(snapshot, &targets_envelope, &revocations_bytes)?;
    let snapshot_body = encode_snapshot(updated_snapshot)?;
    let snapshot_signer = common::role_key(&root, MetadataRole::Snapshot)?;
    let snapshot_envelope = common::sign_binary(
        MetadataRole::Snapshot,
        &snapshot_body,
        &signing_key,
        snapshot_signer,
    )?;
    let timestamp = TimestampMetadata {
        header: MetadataHeader {
            role: MetadataRole::Timestamp,
            version: updated_snapshot.header.version,
            expires: 0,
        },
        snapshot_version: updated_snapshot.header.version,
        snapshot_length: snapshot_envelope.len() as u32,
        snapshot_sha256: digest_bytes(&snapshot_envelope),
    };
    let timestamp_body = encode_timestamp(timestamp)?;
    let timestamp_signer = common::role_key(&root, MetadataRole::Timestamp)?;
    let timestamp_envelope = common::sign_binary(
        MetadataRole::Timestamp,
        &timestamp_body,
        &signing_key,
        timestamp_signer,
    )?;

    common::write_replace(&common::targets_path(&repository), &targets_envelope)?;
    common::write_replace(&common::snapshot_path(&repository), &snapshot_envelope)?;
    common::write_replace(&common::timestamp_path(&repository), &timestamp_envelope)?;
    println!("Registered package: {}", package_path.display());
    println!("package id: {}", common::hex_encode(&manifest.package_id));
    println!("sha256: {}", common::hex_encode(&digest.0));
    Ok(())
}

fn parse_package<'a>(
    bytes: &'a [u8],
    manifest: &PackageManifest,
) -> Result<dali_amrn::v5::Package<'a>, String> {
    let target = dali_targets::find_target(manifest.target_profile.as_str().unwrap_or_default())
        .ok_or_else(|| "manifest target_profile is not a supported target".to_owned())?;
    let slot = target
        .memory
        .isolation
        .and_then(|memory| memory.slots.iter().find(|slot| slot.id == manifest.slot_id))
        .ok_or_else(|| "manifest slot is not declared by the target".to_owned())?;
    let contract = dali_amrn::v3::Contract {
        target_id: target.amrn_target_id,
        code_load_address: slot.code_origin,
        code_capacity: slot.code_length,
        data_load_address: slot.data_origin,
        data_capacity: slot.data_length,
    };
    dali_amrn::v5::parse(bytes, contract)
        .map_err(|error| format!("invalid AMRN v5 package: {error:?}"))
}

fn verify_stored_package(
    repository: &std::path::Path,
    digest: &Sha256Digest,
    package: &[u8],
) -> Result<(), String> {
    let path = repository
        .join(common::PACKAGES_DIRECTORY)
        .join(format!("{}.amrn", common::hex_encode(&digest.0)));
    let stored = fs::read(&path).map_err(|error| {
        format!(
            "cannot read content-addressed package {}: {error}",
            path.display()
        )
    })?;
    if stored != package {
        return Err(format!(
            "content-addressed package differs from input: {}",
            path.display()
        ));
    }
    Ok(())
}

fn validate_manifest_package(
    manifest: &PackageManifest,
    package: &dali_amrn::v5::Package<'_>,
) -> Result<(), String> {
    if package.header.metadata.package_id != manifest.package_id
        || package.header.metadata.slot_id != manifest.slot_id
        || package.header.metadata.required_services != manifest.required_services
        || package.header.metadata.package_version != manifest.package_version_parts
        || package.header.metadata.minimum_kernel_version != manifest.minimum_kernel_version_parts
    {
        return Err("dali.toml identity fields do not match the AMRN header".to_owned());
    }
    Ok(())
}

fn authorize_package(
    targets: &TargetsMetadata,
    delegation: &DelegationMetadata,
    delegation_id: BoundedText<{ dali_metadata::MAX_DELEGATION_ID_BYTES }>,
    namespace: BoundedText<{ dali_metadata::MAX_NAMESPACE_BYTES }>,
    manifest: &PackageManifest,
) -> Result<(), String> {
    if !targets.delegations[..usize::from(targets.delegation_count)].contains(&delegation_id) {
        return Err("targets does not reference the selected delegation".to_owned());
    }
    if delegation.key_id != manifest.developer_key_id
        || !delegation.allowed_namespaces[..usize::from(delegation.namespace_count)]
            .contains(&namespace)
        || !delegation.allowed_targets[..usize::from(delegation.target_count)]
            .contains(&manifest.target_profile)
        || !delegation.allowed_abis[..usize::from(delegation.abi_count)]
            .contains(&u16::from(dali_amrn::v3::ABI_VERSION))
    {
        return Err("package fields are not authorized by the selected delegation".to_owned());
    }
    Ok(())
}

fn append_package(
    mut targets: TargetsMetadata,
    package: TargetPackage,
) -> Result<TargetsMetadata, String> {
    let count = usize::from(targets.package_count);
    if count >= dali_metadata::MAX_TARGET_RECORDS {
        return Err("targets package capacity is exhausted".to_owned());
    }
    if targets.packages[..count]
        .iter()
        .any(|existing| existing.package_id == package.package_id)
    {
        return Err("targets already contains this package ID".to_owned());
    }
    targets.packages[count] = package;
    targets.package_count += 1;
    let active = &mut targets.packages[..usize::from(targets.package_count)];
    active.sort_by_key(|left| left.package_id.0);
    targets.header.version += 1;
    Ok(targets)
}

fn update_snapshot(
    mut snapshot: SnapshotMetadata,
    targets_envelope: &[u8],
    revocations_envelope: &[u8],
) -> Result<SnapshotMetadata, String> {
    snapshot.header.version += 1;
    snapshot.targets = TargetsReference {
        version: snapshot.targets.version + 1,
        length: targets_envelope.len() as u32,
        sha256: digest_bytes(targets_envelope),
    };
    if revocations_envelope.is_empty() {
        return Err("revocation metadata envelope is empty".to_owned());
    }
    snapshot.revocations.length = revocations_envelope.len() as u32;
    snapshot.revocations.sha256 = digest_bytes(revocations_envelope);
    Ok(snapshot)
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

fn encode_timestamp(metadata: TimestampMetadata) -> Result<Vec<u8>, String> {
    let mut output = vec![0; dali_metadata::MAX_TIMESTAMP_BYTES];
    let length = dali_metadata::encode_binary_timestamp_body(metadata, &mut output)
        .map_err(|error| format!("cannot encode timestamp: {error:?}"))?;
    output.truncate(length);
    Ok(output)
}

fn digest_bytes(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest(Sha256::digest(bytes).into())
}
