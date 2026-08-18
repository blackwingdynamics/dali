use std::{env, fs};

use super::super::package as package_command;
use super::{artifacts, build};

const AMRN_EXTENSION: &str = "amrn";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    if arguments.len() != 2 {
        return Err(usage());
    }
    let project_directory = env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?;
    let manifest = build::read_manifest(&project_directory)?;
    let target_profile = build::target_profile(&manifest.target_profile)?;
    let target = target_profile.rust_target;
    let abi_version = manifest.abi_version.unwrap_or(target_profile.abi_version);
    let abi_contract = dali_amrn::compatibility::for_abi(abi_version)
        .ok_or_else(|| format!("unsupported application ABI version {abi_version}"))?;
    let release = build::cargo_profile_is_release(&manifest.profile)?;
    let output = build::payload_path(&project_directory, target, release, &manifest.name)
        .with_extension(AMRN_EXTENSION);
    let format_version = manifest
        .format_version
        .unwrap_or(abi_contract.default_format_version);
    if manifest.slot_name.is_some()
        && format_version != dali_amrn::v3::FORMAT_VERSION
        && format_version != dali_amrn::v4::FORMAT_VERSION
    {
        return Err("manifest slot selection requires AMRN format version 3 or 4".to_owned());
    }
    build::validate_target_capabilities(target_profile, abi_version, Some(format_version))?;
    if !abi_contract.supports_format(format_version) {
        return Err(format!(
            "AMRN format version {format_version} is incompatible with ABI version {abi_version}"
        ));
    }
    let package = if abi_contract.family == dali_amrn::compatibility::AbiFamily::Legacy {
        if format_version != dali_amrn::FORMAT_VERSION {
            return Err(format!(
                "AMRN format version {format_version} is incompatible with ABI version {abi_version}"
            ));
        }
        let payload = build::payload_path(&project_directory, target, release, &manifest.name);
        let payload_bytes = fs::read(&payload).map_err(|error| {
            format!("cannot read native payload {}: {error}", payload.display())
        })?;
        package_command::build_package(&payload_bytes, manifest.entry_offset)?
    } else {
        match format_version {
            dali_amrn::v2::FORMAT_VERSION => build_isolation_package(
                &project_directory,
                &manifest,
                target_profile,
                target,
                release,
            )?,
            dali_amrn::v3::FORMAT_VERSION => build_relocatable_package(
                &project_directory,
                &manifest,
                target_profile,
                target,
                release,
            )?,
            dali_amrn::v4::FORMAT_VERSION => build_relocatable_identity_package(
                &project_directory,
                &manifest,
                target_profile,
                target,
                release,
            )?,
            _ => return Err(format!("unsupported AMRN format version {format_version}")),
        }
    };
    fs::write(&output, package)
        .map_err(|error| format!("cannot write AMRN package {}: {error}", output.display()))?;
    println!("Created AMRN package: {}", output.display());
    Ok(())
}

fn build_relocatable_identity_package(
    project_directory: &std::path::Path,
    manifest: &build::ApplicationManifest,
    target_profile: &dali_targets::TargetProfile,
    target: &str,
    release: bool,
) -> Result<Vec<u8>, String> {
    let slot_name = manifest
        .slot_name
        .as_deref()
        .ok_or_else(|| "AMRN format version 4 requires an explicit `slot`".to_owned())?;
    let package_id = manifest
        .package_id
        .as_deref()
        .ok_or_else(|| "AMRN format version 4 requires `package_id`".to_owned())?;
    let package_version = manifest
        .package_version
        .as_deref()
        .ok_or_else(|| "AMRN format version 4 requires `version`".to_owned())?;
    let minimum_kernel_version = manifest
        .minimum_kernel_version
        .as_deref()
        .ok_or_else(|| "AMRN format version 4 requires `minimum_kernel_version`".to_owned())?;
    let slot = build::application_slot(target_profile, Some(slot_name))?;
    let code = artifacts::code_path(project_directory, target, release, &manifest.name);
    let data = artifacts::data_path(project_directory, target, release, &manifest.name);
    let elf = artifacts::elf_path(project_directory, target, release, &manifest.name);
    let code_bytes = fs::read(&code)
        .map_err(|error| format!("cannot read ABI v3 code {}: {error}", code.display()))?;
    let data_bytes = fs::read(&data)
        .map_err(|error| format!("cannot read ABI v3 data {}: {error}", data.display()))?;
    let relocation_artifact = super::relocations::extract(&elf)?;
    let features = build::features_for_abi(dali_amrn::v2::ABI_VERSION)?;
    let context = artifacts::ArtifactContext {
        project_directory,
        manifest: &project_directory.join("Cargo.toml"),
        target,
        release,
        binary: &manifest.name,
        features: &features,
    };
    let image = dali_amrn::v4::Image {
        image: dali_amrn::v3::Image {
            code: &code_bytes,
            initialized_data: &data_bytes,
            data_zero_size: artifacts::read_zero_init_size(&context)?,
            stack_size: slot.stack_length,
            linked_code_base: relocation_artifact.linked_code_base,
            linked_data_base: relocation_artifact.linked_data_base,
            execution_offset: manifest.entry_offset,
            relocations: &relocation_artifact.relocations,
        },
        metadata: dali_amrn::v4::Metadata {
            package_id: build::parse_package_id(package_id)?,
            package_version: build::parse_version(package_version, "version")?,
            minimum_kernel_version: build::parse_version(
                minimum_kernel_version,
                "minimum_kernel_version",
            )?,
            required_services: manifest.required_services.unwrap_or(0),
            slot_id: slot.id,
        },
    };
    let contract = dali_amrn::v3::Contract {
        target_id: target_profile.amrn_target_id,
        code_load_address: slot.code_origin,
        code_capacity: slot.code_length,
        data_load_address: slot.data_origin,
        data_capacity: slot.data_length,
    };
    let relocation_bytes = relocation_artifact
        .relocations
        .len()
        .checked_mul(dali_amrn::v4::RELOCATION_ENTRY_SIZE)
        .ok_or_else(|| "AMRN v4 relocation table size overflow".to_owned())?;
    let capacity = dali_amrn::v4::HEADER_SIZE
        .checked_add(code_bytes.len())
        .and_then(|size| size.checked_add(data_bytes.len()))
        .and_then(|size| size.checked_add(relocation_bytes))
        .ok_or_else(|| "AMRN v4 package size overflow".to_owned())?;
    let mut package = vec![0; capacity];
    let size = dali_amrn::v4::encode(image, contract, &mut package)
        .map_err(|error| format!("cannot encode AMRN v4 package: {error:?}"))?;
    package.truncate(size);
    Ok(package)
}

fn build_relocatable_package(
    project_directory: &std::path::Path,
    manifest: &build::ApplicationManifest,
    target_profile: &dali_targets::TargetProfile,
    target: &str,
    release: bool,
) -> Result<Vec<u8>, String> {
    let slot = build::application_slot(target_profile, manifest.slot_name.as_deref())?;
    let code = artifacts::code_path(project_directory, target, release, &manifest.name);
    let data = artifacts::data_path(project_directory, target, release, &manifest.name);
    let elf = artifacts::elf_path(project_directory, target, release, &manifest.name);
    let code_bytes = fs::read(&code)
        .map_err(|error| format!("cannot read ABI v3 code {}: {error}", code.display()))?;
    let data_bytes = fs::read(&data)
        .map_err(|error| format!("cannot read ABI v3 data {}: {error}", data.display()))?;
    let relocation_artifact = super::relocations::extract(&elf)?;
    let features = build::features_for_abi(dali_amrn::v2::ABI_VERSION)?;
    let context = artifacts::ArtifactContext {
        project_directory,
        manifest: &project_directory.join("Cargo.toml"),
        target,
        release,
        binary: &manifest.name,
        features: &features,
    };
    let image = dali_amrn::v3::Image {
        code: &code_bytes,
        initialized_data: &data_bytes,
        data_zero_size: artifacts::read_zero_init_size(&context)?,
        stack_size: slot.stack_length,
        linked_code_base: relocation_artifact.linked_code_base,
        linked_data_base: relocation_artifact.linked_data_base,
        execution_offset: manifest.entry_offset,
        relocations: &relocation_artifact.relocations,
    };
    let contract = dali_amrn::v3::Contract {
        target_id: target_profile.amrn_target_id,
        code_load_address: slot.code_origin,
        code_capacity: slot.code_length,
        data_load_address: slot.data_origin,
        data_capacity: slot.data_length,
    };
    let relocation_bytes = relocation_artifact
        .relocations
        .len()
        .checked_mul(dali_amrn::v3::RELOCATION_ENTRY_SIZE)
        .ok_or_else(|| "AMRN v3 relocation table size overflow".to_owned())?;
    let capacity = dali_amrn::v3::HEADER_SIZE
        .checked_add(code_bytes.len())
        .and_then(|size| size.checked_add(data_bytes.len()))
        .and_then(|size| size.checked_add(relocation_bytes))
        .ok_or_else(|| "AMRN v3 package size overflow".to_owned())?;
    let mut package = vec![0; capacity];
    let size = dali_amrn::v3::encode(image, contract, &mut package)
        .map_err(|error| format!("cannot encode AMRN v3 package: {error:?}"))?;
    package.truncate(size);
    Ok(package)
}

fn build_isolation_package(
    project_directory: &std::path::Path,
    manifest: &build::ApplicationManifest,
    target_profile: &dali_targets::TargetProfile,
    target: &str,
    release: bool,
) -> Result<Vec<u8>, String> {
    let slot = build::application_slot(target_profile, None)?;
    let code = artifacts::code_path(project_directory, target, release, &manifest.name);
    let data = artifacts::data_path(project_directory, target, release, &manifest.name);
    let code_bytes = fs::read(&code)
        .map_err(|error| format!("cannot read ABI v3 code {}: {error}", code.display()))?;
    let data_bytes = fs::read(&data)
        .map_err(|error| format!("cannot read ABI v3 data {}: {error}", data.display()))?;
    let features = build::features_for_abi(dali_amrn::v2::ABI_VERSION)?;
    let context = artifacts::ArtifactContext {
        project_directory,
        manifest: &project_directory.join("Cargo.toml"),
        target,
        release,
        binary: &manifest.name,
        features: &features,
    };
    let zero_init_size = artifacts::read_zero_init_size(&context)?;
    let image = dali_amrn::v2::Image {
        code: &code_bytes,
        initialized_data: &data_bytes,
        data_zero_size: zero_init_size,
        stack_size: slot.stack_length,
        execution_offset: manifest.entry_offset,
    };
    let contract = dali_amrn::v2::Contract {
        target_id: target_profile.amrn_target_id,
        code_load_address: slot.code_origin,
        code_capacity: slot.code_length,
        data_load_address: slot.data_origin,
        data_capacity: slot.data_length,
    };
    let capacity = dali_amrn::v2::HEADER_SIZE
        .checked_add(code_bytes.len())
        .and_then(|size| size.checked_add(data_bytes.len()))
        .ok_or_else(|| "ABI v3 package size overflow".to_owned())?;
    let mut package = vec![0; capacity];
    let size = dali_amrn::v2::encode(image, contract, &mut package)
        .map_err(|error| format!("cannot encode ABI v3 package: {error:?}"))?;
    package.truncate(size);
    Ok(package)
}

fn usage() -> String {
    "usage:\n  dali app package".to_owned()
}
