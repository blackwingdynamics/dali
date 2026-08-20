use std::{
    fs,
    path::{Path, PathBuf},
};

use dali_amrn::v3;
use dali_metadata::{
    BundleMetadata, PackageId, parse_bundle_signed, parse_root_signed, parse_signed_envelope,
    parse_targets_signed, verify_repository_package,
};

use super::common;

pub(super) fn inspect(arguments: &[String]) -> Result<(), String> {
    let root = PathBuf::from(common::required(arguments, common::INPUT_FLAG)?);
    println!("{}", inspect_bundle(&root)?);
    Ok(())
}

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let root = PathBuf::from(common::required(arguments, common::INPUT_FLAG)?);
    let package_id = PackageId(common::parse_hex::<16>(
        &common::required(arguments, common::PACKAGE_ID_FLAG)?,
        common::PACKAGE_ID_FLAG,
    )?);
    let (manifest, bundle) = load_bundle(&root)?;
    common::verify_bundle_files(&root, &bundle)?;
    let root_envelope = common::read_envelope(
        &root.join(common::METADATA_DIRECTORY).join("root.json"),
        "root",
    )?;
    let root_metadata = parse_root_signed(root_envelope.signed)
        .map_err(|error| format!("invalid root metadata: {error:?}"))?;
    common::verify_bundle_signature(&root_metadata, manifest)?;
    let targets = common::read_envelope(
        &root.join(common::METADATA_DIRECTORY).join("targets.json"),
        "targets",
    )?;
    let targets_metadata = parse_targets_signed(targets.signed)
        .map_err(|error| format!("invalid targets metadata: {error:?}"))?;
    let target = targets_metadata
        .packages
        .iter()
        .take(usize::from(targets_metadata.package_count))
        .find(|candidate| candidate.package_id == package_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "package {} is not declared by targets metadata",
                common::hex_encode(&package_id.0)
            )
        })?;
    let delegation_id = target
        .delegation_id
        .as_str()
        .ok_or_else(|| "target delegation ID is invalid".to_owned())?;
    let delegation_path = root
        .join(common::METADATA_DIRECTORY)
        .join(common::DELEGATIONS_DIRECTORY)
        .join(format!("{delegation_id}.json"));
    let package_path = root
        .join(common::PACKAGES_DIRECTORY)
        .join(format!("{}.amrn", common::hex_encode(&target.sha256.0)));
    let package = Box::leak(
        fs::read(&package_path)
            .map_err(|error| format!("cannot read package {}: {error}", package_path.display()))?
            .into_boxed_slice(),
    );
    let documents = dali_metadata::RepositoryPackageDocuments {
        root: root_envelope,
        timestamp: common::read_envelope(
            &root.join(common::METADATA_DIRECTORY).join("timestamp.json"),
            "timestamp",
        )?,
        snapshot: common::read_envelope(
            &root.join(common::METADATA_DIRECTORY).join("snapshot.json"),
            "snapshot",
        )?,
        targets,
        revocations: common::read_envelope(
            &root
                .join(common::METADATA_DIRECTORY)
                .join("revocations.json"),
            "revocation",
        )?,
        delegation: common::read_envelope(&delegation_path, "delegation")?,
        package,
    };
    let profile_name = bundle
        .target_profile
        .as_str()
        .ok_or_else(|| "bundle target profile is invalid".to_owned())?;
    let profile = dali_targets::find_target(profile_name)
        .ok_or_else(|| format!("unsupported target profile in bundle: {profile_name}"))?;
    let slot = profile
        .memory
        .isolation
        .and_then(|memory| memory.active_slot())
        .ok_or_else(|| "target profile has no application slot contract".to_owned())?;
    let contract = v3::Contract {
        target_id: profile.amrn_target_id,
        code_load_address: slot.code_origin,
        code_capacity: slot.code_length,
        data_load_address: slot.data_origin,
        data_capacity: slot.data_length,
    };
    verify_repository_package(documents, package_id, contract, None)
        .map_err(|error| format!("repository verification failed: {error:?}"))?;
    println!("Repository bundle verified");
    println!("target_profile: {profile_name}");
    println!("package_id: {}", common::hex_encode(&package_id.0));
    println!(
        "chain: root -> timestamp -> snapshot -> targets -> delegation -> revocation -> package -> AMRN"
    );
    Ok(())
}

fn inspect_bundle(root: &Path) -> Result<String, String> {
    let (_, bundle) = load_bundle(root)?;
    common::verify_bundle_files(root, &bundle)?;
    let mut report = format!(
        "Repository bundle valid\ntarget_profile: {}\nversion: {}\nfiles: {}",
        bundle.target_profile.as_str().unwrap_or("<invalid>"),
        bundle.header.version,
        bundle.file_count
    );
    for file in bundle.files.iter().take(usize::from(bundle.file_count)) {
        report.push_str(&format!(
            "\n- {}:{} length={} sha256={}",
            file.kind.as_str(),
            file.id.as_str().unwrap_or("<invalid>"),
            file.length,
            common::hex_encode(&file.sha256.0)
        ));
    }
    Ok(report)
}

fn load_bundle(
    root: &Path,
) -> Result<(dali_metadata::SignedEnvelope<'static>, BundleMetadata), String> {
    let bytes = Box::leak(
        fs::read(root.join(common::MANIFEST_NAME))
            .map_err(|error| format!("cannot read bundle manifest: {error}"))?
            .into_boxed_slice(),
    );
    let envelope = parse_signed_envelope(bytes)
        .map_err(|error| format!("invalid bundle manifest envelope: {error:?}"))?;
    let bundle = parse_bundle_signed(envelope.signed)
        .map_err(|error| format!("invalid bundle manifest body: {error:?}"))?;
    Ok((envelope, bundle))
}
