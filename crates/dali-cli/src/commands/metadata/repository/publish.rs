use std::{fs, path::PathBuf};

use dali_metadata::{BoundedText, KeyId, MetadataRole, parse_binary_root_body};

use super::common;

pub fn run(arguments: &[String]) -> Result<(), String> {
    let root = PathBuf::from(common::required(arguments, common::INPUT_FLAG)?);
    let signing_key = common::read_seed(&PathBuf::from(common::required(
        arguments,
        common::ROOT_SIGNING_KEY_FLAG,
    )?))?;
    let bundle_signing_key = common::read_seed(&PathBuf::from(common::required(
        arguments,
        common::BUNDLE_SIGNING_KEY_FLAG,
    )?))?;
    let bundle_key_id = KeyId(common::parse_hex::<16>(
        &common::required(arguments, common::BUNDLE_KEY_ID_FLAG)?,
        common::BUNDLE_KEY_ID_FLAG,
    )?);
    let target_profile =
        BoundedText::new(&common::required(arguments, common::TARGET_PROFILE_FLAG)?)
            .map_err(|error| format!("{} is invalid: {error:?}", common::TARGET_PROFILE_FLAG))?;
    let version = common::parse_u64(
        &common::required(arguments, common::VERSION_FLAG)?,
        common::VERSION_FLAG,
    )?;
    let (_, root_envelope) = common::read_binary(&common::root_path(&root), MetadataRole::Root)?;
    let root_metadata = parse_binary_root_body(root_envelope.body)
        .map_err(|error| format!("invalid root body: {error:?}"))?;
    resign_file(&root, MetadataRole::Root, &signing_key, &root_metadata)?;
    resign_file(&root, MetadataRole::Timestamp, &signing_key, &root_metadata)?;
    resign_file(&root, MetadataRole::Snapshot, &signing_key, &root_metadata)?;
    resign_file(&root, MetadataRole::Targets, &signing_key, &root_metadata)?;
    resign_file(
        &root,
        MetadataRole::Revocation,
        &signing_key,
        &root_metadata,
    )?;
    let delegation_dir = root
        .join(common::METADATA_DIRECTORY)
        .join(common::DELEGATIONS_DIRECTORY);
    for entry in fs::read_dir(&delegation_dir)
        .map_err(|error| format!("cannot scan {}: {error}", delegation_dir.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot read delegation entry: {error}"))?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("dmb") {
            resign_path(
                &path,
                MetadataRole::Delegation,
                &signing_key,
                &root_metadata,
            )?;
        }
    }
    super::super::bundle::generate::generate_binary_repository(
        &root,
        &root.join("bundle.manifest"),
        target_profile,
        version,
        bundle_signing_key,
        bundle_key_id,
    )?;
    println!(
        "Published signed Binary v2 metadata chain: {}",
        root.display()
    );
    println!(
        "Created signed bundle manifest: {}",
        root.join("bundle.manifest").display()
    );
    Ok(())
}

fn resign_file(
    root: &std::path::Path,
    role: MetadataRole,
    seed: &[u8; 32],
    root_metadata: &dali_metadata::RootMetadata,
) -> Result<(), String> {
    let path = match role {
        MetadataRole::Root => common::root_path(root),
        MetadataRole::Timestamp => common::timestamp_path(root),
        MetadataRole::Snapshot => common::snapshot_path(root),
        MetadataRole::Targets => common::targets_path(root),
        MetadataRole::Revocation => common::revocations_path(root),
        _ => return Err(format!("unsupported publication role: {}", role.as_str())),
    };
    resign_path(&path, role, seed, root_metadata)
}

fn resign_path(
    path: &std::path::Path,
    role: MetadataRole,
    seed: &[u8; 32],
    root_metadata: &dali_metadata::RootMetadata,
) -> Result<(), String> {
    let (_, envelope) = common::read_binary(path, role)?;
    let signer = common::role_key(root_metadata, role)?;
    let replacement = common::sign_binary(role, envelope.body, seed, signer)?;
    common::write_replace(path, &replacement)
}
