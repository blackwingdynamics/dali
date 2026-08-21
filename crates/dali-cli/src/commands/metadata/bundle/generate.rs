use std::{
    fs,
    path::{Path, PathBuf},
};

use dali_metadata::{
    BoundedText, BundleMetadata, KeyId, MetadataHeader, MetadataRole, SignatureRecord,
    SignatureSet, encode_binary_bundle_body, encode_binary_envelope, encode_bundle_signed,
    encode_signed_envelope,
};

use super::common;

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let root = PathBuf::from(common::required(arguments, common::INPUT_FLAG)?);
    let output = PathBuf::from(common::required(arguments, common::OUTPUT_FLAG)?);
    let format = common::MetadataFormat::parse(arguments)?;
    let target_profile =
        BoundedText::new(&common::required(arguments, common::TARGET_PROFILE_FLAG)?)
            .map_err(|error| format!("{} is invalid: {error:?}", common::TARGET_PROFILE_FLAG))?;
    let version = common::parse_u64(
        &common::required(arguments, common::VERSION_FLAG)?,
        common::VERSION_FLAG,
    )?;
    let signer_key_id = KeyId(common::parse_hex::<16>(
        &common::required(arguments, common::SIGNER_KEY_ID_FLAG)?,
        common::SIGNER_KEY_ID_FLAG,
    )?);
    let seed = common::read_seed(&PathBuf::from(common::required(
        arguments,
        common::SIGNING_KEY_FLAG,
    )?))?;
    generate_repository(
        &root,
        &output,
        format,
        target_profile,
        version,
        seed,
        signer_key_id,
    )
}

pub(crate) fn generate_binary_repository(
    root: &Path,
    output: &Path,
    target_profile: BoundedText<{ dali_metadata::MAX_TARGET_PROFILE_BYTES }>,
    version: u64,
    seed: [u8; dali_crypto::PRIVATE_KEY_LENGTH],
    signer_key_id: KeyId,
) -> Result<(), String> {
    generate_repository(
        root,
        output,
        common::MetadataFormat::BinaryV2,
        target_profile,
        version,
        seed,
        signer_key_id,
    )
}

fn generate_repository(
    root: &Path,
    output: &Path,
    format: common::MetadataFormat,
    target_profile: BoundedText<{ dali_metadata::MAX_TARGET_PROFILE_BYTES }>,
    version: u64,
    seed: [u8; dali_crypto::PRIVATE_KEY_LENGTH],
    signer_key_id: KeyId,
) -> Result<(), String> {
    let files = collect_files(root, format)?;
    let count = common::file_count(&files);
    let metadata = BundleMetadata {
        header: MetadataHeader {
            role: MetadataRole::Bundle,
            version,
            expires: 0,
        },
        target_profile,
        files,
        file_count: count,
    };
    let mut records = [SignatureRecord::default(); dali_metadata::MAX_SIGNATURES];
    records[0] = SignatureRecord {
        key_id: signer_key_id,
        signature: dali_metadata::Signature([0; dali_crypto::SIGNATURE_LENGTH]),
    };
    let (envelope, envelope_length) = match format {
        common::MetadataFormat::JsonV1 => {
            let mut signed = vec![0; dali_metadata::MAX_BUNDLE_BYTES];
            let signed_length = encode_bundle_signed(&mut signed, metadata)
                .map_err(|error| format!("cannot encode bundle manifest: {error:?}"))?;
            let signature = dali_crypto::sign(&seed, &signed[..signed_length]);
            records[0].signature = dali_metadata::Signature(signature);
            let mut envelope = vec![0; dali_metadata::MAX_ENVELOPE_BYTES];
            let length = encode_signed_envelope(
                &mut envelope,
                &signed[..signed_length],
                SignatureSet { records, count: 1 },
            )
            .map_err(|error| format!("cannot encode bundle envelope: {error:?}"))?;
            (envelope, length)
        }
        common::MetadataFormat::BinaryV2 => {
            let mut body = vec![0; dali_metadata::MAX_BUNDLE_BYTES];
            let body_length = encode_binary_bundle_body(metadata, &mut body)
                .map_err(|error| format!("cannot encode binary-v2 bundle body: {error:?}"))?;
            let signature = dali_crypto::sign(&seed, &body[..body_length]);
            records[0].signature = dali_metadata::Signature(signature);
            let mut envelope = vec![0; dali_metadata::MAX_ENVELOPE_BYTES];
            let length = encode_binary_envelope(
                MetadataRole::Bundle,
                &body[..body_length],
                SignatureSet { records, count: 1 },
                &mut envelope,
            )
            .map_err(|error| format!("cannot encode binary-v2 bundle envelope: {error:?}"))?;
            (envelope, length)
        }
    };
    common::write_new(output, &envelope[..envelope_length])?;
    println!(
        "Created signed repository bundle manifest: {}",
        output.display()
    );
    println!("files: {count}");
    Ok(())
}

fn collect_files(
    root: &Path,
    format: common::MetadataFormat,
) -> Result<[dali_metadata::BundleFile; dali_metadata::MAX_BUNDLE_FILES], String> {
    let mut collected = Vec::new();
    for (kind, id, path) in fixed_files(root, format) {
        collected.push(common::file_record(kind, id, &path)?);
    }
    let delegation_dir = root
        .join(common::METADATA_DIRECTORY)
        .join(common::DELEGATIONS_DIRECTORY);
    for entry in fs::read_dir(&delegation_dir)
        .map_err(|error| format!("cannot scan {}: {error}", delegation_dir.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot read delegation entry: {error}"))?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) == Some(format.extension()) {
            let id = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| "invalid delegation filename".to_owned())?;
            collected.push(common::file_record(
                dali_metadata::BundleFileKind::Delegation,
                id,
                &path,
            )?);
        }
    }
    let package_dir = root.join(common::PACKAGES_DIRECTORY);
    for entry in fs::read_dir(&package_dir)
        .map_err(|error| format!("cannot scan {}: {error}", package_dir.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot read package entry: {error}"))?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("amrn") {
            let id = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| "invalid package filename".to_owned())?;
            collected.push(common::file_record(
                dali_metadata::BundleFileKind::Package,
                id,
                &path,
            )?);
        }
    }
    collected.sort_by(|left, right| common::bundle_order(*left, *right));
    let mut files = [dali_metadata::BundleFile::default(); dali_metadata::MAX_BUNDLE_FILES];
    if collected.len() > files.len() {
        return Err("repository bundle exceeds the bounded file limit".to_owned());
    }
    files[..collected.len()].copy_from_slice(&collected);
    Ok(files)
}

fn fixed_files(
    root: &Path,
    format: common::MetadataFormat,
) -> [(dali_metadata::BundleFileKind, &str, PathBuf); 5] {
    let extension = format.extension();
    [
        (
            dali_metadata::BundleFileKind::Root,
            "root",
            root.join(common::METADATA_DIRECTORY)
                .join(format!("root.{extension}")),
        ),
        (
            dali_metadata::BundleFileKind::Timestamp,
            "timestamp",
            root.join(common::METADATA_DIRECTORY)
                .join(format!("timestamp.{extension}")),
        ),
        (
            dali_metadata::BundleFileKind::Snapshot,
            "snapshot",
            root.join(common::METADATA_DIRECTORY)
                .join(format!("snapshot.{extension}")),
        ),
        (
            dali_metadata::BundleFileKind::Targets,
            "targets",
            root.join(common::METADATA_DIRECTORY)
                .join(format!("targets.{extension}")),
        ),
        (
            dali_metadata::BundleFileKind::Revocation,
            "revocation",
            root.join(common::METADATA_DIRECTORY)
                .join(format!("revocations.{extension}")),
        ),
    ]
}
