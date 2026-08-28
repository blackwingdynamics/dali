use std::{
    fs,
    path::{Path, PathBuf},
};

use dali_amrn::v3;
use dali_metadata::{
    BundleMetadata, CartridgeId, parse_bundle_signed, parse_root_signed, parse_signed_envelope,
    parse_targets_signed, verify_repository_cartridge,
};

use super::common;

pub(super) fn inspect(arguments: &[String]) -> Result<(), String> {
    let root = PathBuf::from(common::required(arguments, common::INPUT_FLAG)?);
    let format = common::MetadataFormat::parse(arguments)?;
    println!("{}", inspect_bundle(&root, format)?);
    Ok(())
}

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let root = PathBuf::from(common::required(arguments, common::INPUT_FLAG)?);
    let format = common::MetadataFormat::parse(arguments)?;
    if format == common::MetadataFormat::BinaryV2 {
        return verify_binary_manifest(&root);
    }
    let cartridge_id = CartridgeId(common::parse_hex::<16>(
        &common::required(arguments, common::CARTRIDGE_ID_FLAG)?,
        common::CARTRIDGE_ID_FLAG,
    )?);
    let (manifest, bundle) = load_bundle(&root, format)?;
    common::verify_bundle_files(&root, &bundle, format)?;
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
        .cartridges
        .iter()
        .take(usize::from(targets_metadata.cartridge_count))
        .find(|candidate| candidate.cartridge_id == cartridge_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "cartridge {} is not declared by targets metadata",
                common::hex_encode(&cartridge_id.0)
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
    let cartridge_path = root
        .join(common::CARTRIDGES_DIRECTORY)
        .join(format!("{}.amrn", common::hex_encode(&target.sha256.0)));
    let cartridge = Box::leak(
        fs::read(&cartridge_path)
            .map_err(|error| {
                format!(
                    "cannot read cartridge {}: {error}",
                    cartridge_path.display()
                )
            })?
            .into_boxed_slice(),
    );
    let documents = dali_metadata::RepositoryCartridgeDocuments {
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
        cartridge,
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
    verify_repository_cartridge(documents, cartridge_id, contract, None)
        .map_err(|error| format!("repository verification failed: {error:?}"))?;
    println!("Repository bundle verified");
    println!("target_profile: {profile_name}");
    println!("cartridge_id: {}", common::hex_encode(&cartridge_id.0));
    println!(
        "chain: root -> timestamp -> snapshot -> targets -> delegation -> revocation -> cartridge -> AMRN"
    );
    Ok(())
}

fn inspect_bundle(root: &Path, format: common::MetadataFormat) -> Result<String, String> {
    let (_, bundle) = load_bundle(root, format)?;
    common::verify_bundle_files(root, &bundle, format)?;
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
    format: common::MetadataFormat,
) -> Result<(dali_metadata::SignedEnvelope<'static>, BundleMetadata), String> {
    let bytes = Box::leak(
        fs::read(root.join(common::MANIFEST_NAME))
            .map_err(|error| format!("cannot read bundle manifest: {error}"))?
            .into_boxed_slice(),
    );
    match format {
        common::MetadataFormat::JsonV1 => {
            let envelope = parse_signed_envelope(bytes)
                .map_err(|error| format!("invalid bundle manifest envelope: {error:?}"))?;
            let bundle = parse_bundle_signed(envelope.signed)
                .map_err(|error| format!("invalid bundle manifest body: {error:?}"))?;
            Ok((envelope, bundle))
        }
        common::MetadataFormat::BinaryV2 => {
            let envelope = dali_metadata::parse_binary_envelope(bytes)
                .map_err(|error| format!("invalid binary-v2 bundle envelope: {error:?}"))?;
            if envelope.role != dali_metadata::MetadataRole::Bundle {
                return Err("binary-v2 manifest has a non-bundle role".to_owned());
            }
            let bundle = dali_metadata::parse_binary_bundle_body(envelope.body)
                .map_err(|error| format!("invalid binary-v2 bundle body: {error:?}"))?;
            let signed = dali_metadata::SignedEnvelope {
                signed: envelope.body,
                signatures: envelope.signatures,
            };
            Ok((signed, bundle))
        }
    }
}

fn verify_binary_manifest(root: &Path) -> Result<(), String> {
    let (manifest, bundle) = load_bundle(root, common::MetadataFormat::BinaryV2)?;
    common::verify_bundle_files(root, &bundle, common::MetadataFormat::BinaryV2)?;
    let root_path = common::bundle_file_path(
        root,
        dali_metadata::BundleFile {
            kind: dali_metadata::BundleFileKind::Root,
            id: dali_metadata::BoundedText::new("root")
                .map_err(|_| "invalid root file ID".to_owned())?,
            length: 0,
            sha256: dali_metadata::Sha256Digest([0; 32]),
        },
        common::MetadataFormat::BinaryV2,
    )?;
    let root_bytes = fs::read(&root_path)
        .map_err(|error| format!("cannot read binary-v2 root metadata: {error}"))?;
    let root_envelope = dali_metadata::parse_binary_envelope(&root_bytes)
        .map_err(|error| format!("invalid binary-v2 root envelope: {error:?}"))?;
    let root_metadata = dali_metadata::parse_binary_root_body(root_envelope.body)
        .map_err(|error| format!("invalid binary-v2 root body: {error:?}"))?;
    common::verify_bundle_signature_binary(&root_metadata, manifest)?;
    println!("Binary-v2 repository manifest and references verified");
    println!(
        "target_profile: {}",
        bundle.target_profile.as_str().unwrap_or("<invalid>")
    );
    println!("chain: binary root -> bundle manifest -> referenced artifacts");
    println!("note: cartridge-chain boot verification remains pending kernel streaming wiring");
    Ok(())
}
