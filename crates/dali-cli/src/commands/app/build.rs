use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const MANIFEST_FILE: &str = "dali.toml";
const CARGO_MANIFEST_FILE: &str = "Cargo.toml";
const EMBEDDED_PAYLOAD_FEATURE: &str = "embedded-payload";
const DEVELOPMENT_PROFILE: &str = "dev";
pub(super) const DEBUG_OUTPUT_DIRECTORY: &str = "debug";
pub(super) const RELEASE_PROFILE: &str = "release";

pub(super) struct ApplicationManifest {
    pub(super) name: String,
    pub(super) package_version: Option<String>,
    pub(super) package_id: Option<String>,
    pub(super) signing_key_id: Option<String>,
    pub(super) minimum_kernel_version: Option<String>,
    pub(super) required_services: Option<u32>,
    pub(super) target_profile: String,
    pub(super) profile: String,
    pub(super) entry_offset: u32,
    pub(super) abi_version: Option<u8>,
    pub(super) format_version: Option<u8>,
    pub(super) slot_name: Option<String>,
}

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    if arguments.len() != 2 {
        return Err(usage());
    }
    let project_directory = env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?;
    let manifest = read_manifest(&project_directory)?;
    let target_profile = target_profile(&manifest.target_profile)?;
    let target = target_profile.rust_target;
    let abi_version = manifest.abi_version.unwrap_or(target_profile.abi_version);
    validate_target_capabilities(target_profile, abi_version, manifest.format_version)?;
    let release = cargo_profile_is_release(&manifest.profile)?;
    validate_package_authentication(target_profile, release, manifest.format_version)?;
    let cargo_manifest = project_directory.join(CARGO_MANIFEST_FILE);
    run_cargo_build(
        &project_directory,
        &cargo_manifest,
        target,
        release,
        abi_version,
    )?;
    let output = payload_path(&project_directory, target, release, &manifest.name);
    if abi_version == dali_amrn::ABI_VERSION {
        run_cargo_objcopy(
            &project_directory,
            &cargo_manifest,
            target,
            release,
            &manifest.name,
            &output,
            abi_version,
        )?;
        println!("Built native payload: {}", output.display());
    } else if abi_version == dali_amrn::v2::ABI_VERSION {
        let code = super::artifacts::code_path(&project_directory, target, release, &manifest.name);
        let data = super::artifacts::data_path(&project_directory, target, release, &manifest.name);
        let bss_size = super::artifacts::extract_isolation_sections(
            &project_directory,
            &cargo_manifest,
            target,
            release,
            &manifest,
            &code,
            &data,
        )?;
        let elf = super::artifacts::elf_path(&project_directory, target, release, &manifest.name);
        let relocations = super::relocations::extract(&elf)?;
        println!("Built ABI v3 code: {}", code.display());
        println!("Built ABI v3 data: {}", data.display());
        println!("ABI v3 zero-init size: {bss_size} bytes");
        println!(
            "Retained ABI v3 relocation records: {}",
            relocations.relocations.len()
        );
    } else {
        return Err(format!("unsupported application ABI version {abi_version}"));
    }
    super::package::run(&["app".to_owned(), "package".to_owned()])?;
    Ok(())
}

pub(super) fn read_manifest(project_directory: &Path) -> Result<ApplicationManifest, String> {
    let path = project_directory.join(MANIFEST_FILE);
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse_manifest(&contents)
}

fn parse_manifest(contents: &str) -> Result<ApplicationManifest, String> {
    let name = required_value(contents, "name")?;
    let package_version = optional_value(contents, "version");
    let package_id = optional_value(contents, "package_id");
    let signing_key_id = optional_value(contents, "signing_key_id");
    let minimum_kernel_version = optional_value(contents, "minimum_kernel_version");
    let required_services = optional_value(contents, "required_services")
        .map(|value| parse_u32(&value, "required_services"))
        .transpose()?;
    let target_profile = required_value(contents, "target_profile")?;
    let profile =
        optional_value(contents, "profile").unwrap_or_else(|| DEVELOPMENT_PROFILE.to_owned());
    let entry_offset = required_value(contents, "entry_offset")?
        .parse::<u32>()
        .map_err(|_| "dali.toml contains an invalid `entry_offset`".to_owned())?;
    if name.is_empty() || target_profile.is_empty() || profile.is_empty() {
        return Err("dali.toml contains an empty required build value".to_owned());
    }
    let abi_version = optional_value(contents, "abi_version")
        .map(|value| {
            value
                .parse::<u8>()
                .map_err(|_| "dali.toml contains an invalid `abi_version`".to_owned())
        })
        .transpose()?;
    let format_version = optional_value(contents, "format_version")
        .map(|value| {
            value
                .parse::<u8>()
                .map_err(|_| "dali.toml contains an invalid format_version".to_owned())
        })
        .transpose()?;
    let slot_name = optional_value(contents, "slot");
    Ok(ApplicationManifest {
        name,
        package_version,
        package_id,
        signing_key_id,
        minimum_kernel_version,
        required_services,
        target_profile,
        profile,
        entry_offset,
        abi_version,
        format_version,
        slot_name,
    })
}

pub(super) fn parse_package_id(value: &str) -> Result<[u8; 16], String> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 32 {
        return Err("dali.toml `package_id` must contain exactly 32 hexadecimal digits".to_owned());
    }
    let mut result = [0; 16];
    for (index, byte) in result.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| "dali.toml contains an invalid `package_id`".to_owned())?;
    }
    if result.iter().all(|byte| *byte == 0) {
        return Err("dali.toml `package_id` must not be all zero".to_owned());
    }
    Ok(result)
}

pub(super) fn parse_version(value: &str, field: &str) -> Result<dali_amrn::v4::Version, String> {
    let mut components = value.split('.');
    let version = [components.next(), components.next(), components.next()];
    if components.next().is_some() || version.iter().any(Option::is_none) {
        return Err(format!("dali.toml `{field}` must use major.minor.patch"));
    }
    Ok(dali_amrn::v4::Version {
        major: version[0]
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("dali.toml contains an invalid `{field}`"))?,
        minor: version[1]
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("dali.toml contains an invalid `{field}`"))?,
        patch: version[2]
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("dali.toml contains an invalid `{field}`"))?,
    })
}

fn parse_u32(value: &str, field: &str) -> Result<u32, String> {
    let (radix, digits) = match value.strip_prefix("0x") {
        Some(value) => (16, value),
        None => (10, value),
    };
    u32::from_str_radix(digits, radix)
        .map_err(|_| format!("dali.toml contains an invalid `{field}`"))
}

pub(super) fn application_slot(
    target: &dali_targets::TargetProfile,
    slot_name: Option<&str>,
) -> Result<dali_targets::IsolationSlot, String> {
    let isolation = target
        .memory
        .isolation
        .ok_or_else(|| "target does not declare isolation memory".to_owned())?;
    match slot_name {
        Some(name) => isolation
            .slots
            .iter()
            .copied()
            .find(|slot| slot.name == name)
            .ok_or_else(|| format!("target does not declare application slot `{name}`")),
        None => isolation
            .active_slot()
            .ok_or_else(|| "target does not declare an application slot".to_owned()),
    }
}

pub(super) fn target_profile(name: &str) -> Result<&'static dali_targets::TargetProfile, String> {
    dali_targets::find_target(name)
        .ok_or_else(|| format!("unsupported target profile `{name}`; run `dali target list`"))
}

fn required_value(contents: &str, key: &str) -> Result<String, String> {
    optional_value(contents, key).ok_or_else(|| format!("dali.toml is missing `{key}`"))
}

fn optional_value(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let (candidate, value) = line.split_once('=')?;
        if candidate.trim() != key {
            return None;
        }
        Some(value.trim().trim_matches('"').to_owned())
    })
}

pub(super) fn cargo_profile_is_release(profile: &str) -> Result<bool, String> {
    match profile {
        DEVELOPMENT_PROFILE => Ok(false),
        RELEASE_PROFILE => Ok(true),
        _ => Err(format!(
            "unsupported Cargo profile `{profile}`; use `{DEVELOPMENT_PROFILE}` or `{RELEASE_PROFILE}`"
        )),
    }
}

/// Rejects unsigned release packaging until a configured signer exists.
pub(super) fn validate_package_authentication(
    target: &dali_targets::TargetProfile,
    release: bool,
    format_version: Option<u8>,
) -> Result<(), String> {
    let policy = if release {
        target.authentication.release
    } else {
        target.authentication.development
    };
    if policy == dali_targets::PackageAuthentication::Ed25519Required
        && format_version != Some(dali_amrn::v5::FORMAT_VERSION)
    {
        return Err(format!(
            "target `{}` release packages must use AMRN format version {} with an Ed25519 signature",
            target.name,
            dali_amrn::v5::FORMAT_VERSION
        ));
    }
    Ok(())
}

fn run_cargo_build(
    project_directory: &Path,
    manifest: &Path,
    target: &str,
    release: bool,
    abi_version: u8,
) -> Result<(), String> {
    let mut command = cargo_command("build", project_directory, manifest);
    let features = features_for_abi(abi_version)?;
    command.args(["--features", &features, "--target", target]);
    if release {
        command.arg("--release");
    }
    run_command(&mut command, "cargo build")
}

pub(super) fn features_for_abi(abi_version: u8) -> Result<String, String> {
    let contract = dali_amrn::compatibility::for_abi(abi_version)
        .ok_or_else(|| format!("unsupported application ABI version {abi_version}"))?;
    match contract.family {
        dali_amrn::compatibility::AbiFamily::Legacy => Ok(EMBEDDED_PAYLOAD_FEATURE.to_owned()),
        dali_amrn::compatibility::AbiFamily::Isolation => {
            Ok(format!("{EMBEDDED_PAYLOAD_FEATURE},abi-current"))
        }
    }
}

/// Verifies that a target manifest can provide the requested application contract.
pub(super) fn validate_target_capabilities(
    target: &dali_targets::TargetProfile,
    abi_version: u8,
    format_version: Option<u8>,
) -> Result<(), String> {
    let contract = dali_amrn::compatibility::for_abi(abi_version)
        .ok_or_else(|| format!("unsupported application ABI version {abi_version}"))?;
    let format_version = format_version.unwrap_or(contract.default_format_version);
    if !contract.supports_format(format_version) {
        return Err(format!(
            "AMRN format version {format_version} is incompatible with ABI version {abi_version}"
        ));
    }
    if contract.family == dali_amrn::compatibility::AbiFamily::Isolation && !target.capabilities.mpu
    {
        return Err(format!(
            "target `{}` does not declare MPU support for ABI version {abi_version}",
            target.name
        ));
    }
    if (format_version == dali_amrn::v3::FORMAT_VERSION
        || format_version == dali_amrn::v4::FORMAT_VERSION
        || format_version == dali_amrn::v5::FORMAT_VERSION)
        && !target.capabilities.relocation
    {
        return Err(format!(
            "target `{}` does not declare relocation support for AMRN format version {}",
            target.name,
            dali_amrn::v3::FORMAT_VERSION
        ));
    }
    Ok(())
}

pub(super) fn run_cargo_objcopy(
    project_directory: &Path,
    manifest: &Path,
    target: &str,
    release: bool,
    binary: &str,
    output: &Path,
    abi_version: u8,
) -> Result<(), String> {
    let mut command = cargo_command("objcopy", project_directory, manifest);
    let features = features_for_abi(abi_version)?;
    command.args(["--features", &features, "--target", target, "--bin", binary]);
    if release {
        command.arg("--release");
    }
    command.args(["--", "-O", "binary"]);
    command.arg(output);
    run_command(&mut command, "cargo objcopy")
}

pub(super) fn cargo_command(
    subcommand: &str,
    project_directory: &Path,
    manifest: &Path,
) -> Command {
    let mut command = Command::new("cargo");
    command.current_dir(project_directory);
    command.args([subcommand, "--manifest-path"]);
    command.arg(manifest);
    command
}

pub(super) fn run_command(command: &mut Command, description: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("cannot run {description}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{description} failed with status {status}"))
    }
}

pub(super) fn payload_path(
    project_directory: &Path,
    target: &str,
    release: bool,
    name: &str,
) -> PathBuf {
    let profile_directory = if release {
        RELEASE_PROFILE
    } else {
        DEBUG_OUTPUT_DIRECTORY
    };
    project_directory
        .join("target")
        .join(target)
        .join(profile_directory)
        .join(format!("{name}.bin"))
}

fn usage() -> String {
    "usage:\n  dali app build".to_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        DEVELOPMENT_PROFILE, RELEASE_PROFILE, cargo_profile_is_release, parse_manifest,
        validate_target_capabilities,
    };

    #[test]
    fn parses_documented_manifest_values() {
        let manifest = parse_manifest(
            "[application]\nname = \"telemetry\"\n\n[build]\ntarget_profile = \"f405\"\nprofile = \"release\"\nentry_offset = 0",
        ).expect("manifest should parse");
        assert_eq!(manifest.name, "telemetry");
        assert_eq!(manifest.target_profile, "f405");
        assert_eq!(manifest.profile, RELEASE_PROFILE);
        assert_eq!(manifest.entry_offset, 0);
        assert_eq!(manifest.format_version, None);
        assert_eq!(manifest.slot_name, None);
    }

    #[test]
    fn defaults_to_development_profile() {
        let manifest =
            parse_manifest("name = \"telemetry\"\ntarget_profile = \"f405\"\nentry_offset = 0")
                .expect("manifest should parse");
        assert_eq!(manifest.profile, DEVELOPMENT_PROFILE);
    }

    #[test]
    fn rejects_unknown_profiles() {
        assert!(cargo_profile_is_release("custom").is_err());
    }

    #[test]
    fn requires_signatures_for_f405_release_packages() {
        let target = super::target_profile("f405").expect("F405 target is declared");
        assert!(super::validate_package_authentication(target, true, Some(4)).is_err());
        assert!(
            super::validate_package_authentication(
                target,
                true,
                Some(dali_amrn::v5::FORMAT_VERSION)
            )
            .is_ok()
        );
        assert!(super::validate_package_authentication(target, false, Some(4)).is_ok());
    }

    #[test]
    fn parses_explicit_relocation_format() {
        let manifest = parse_manifest(
            "name = \"telemetry\"\ntarget_profile = \"f405\"\nentry_offset = 0\nformat_version = 3",
        )
        .expect("manifest should parse");
        assert_eq!(manifest.format_version, Some(3));
    }

    #[test]
    fn parses_manifest_owned_slot_selection() {
        let manifest = parse_manifest(
            "name = \"telemetry\"\ntarget_profile = \"f405\"\nentry_offset = 0\nformat_version = 3\nslot = \"slot1\"",
        )
        .expect("manifest should parse");
        assert_eq!(manifest.slot_name.as_deref(), Some("slot1"));
    }

    #[test]
    fn parses_identity_metadata() {
        let manifest = parse_manifest(
            "name = \"telemetry\"\nversion = \"1.2.3\"\ntarget_profile = \"f405\"\nentry_offset = 0\nformat_version = 4\npackage_id = \"00112233445566778899AABBCCDDEEFF\"\nminimum_kernel_version = \"0.1.0\"\nrequired_services = \"0x1\"\nslot = \"slot1\"",
        )
        .expect("v4 metadata should parse");
        assert_eq!(manifest.package_version.as_deref(), Some("1.2.3"));
        assert_eq!(
            manifest.package_id.as_deref(),
            Some("00112233445566778899AABBCCDDEEFF")
        );
        assert_eq!(manifest.minimum_kernel_version.as_deref(), Some("0.1.0"));
        assert_eq!(manifest.required_services, Some(1));
    }

    #[test]
    fn accepts_identity_format_for_targets_with_relocation_support() {
        let target = super::target_profile("f405").expect("F405 target is declared");
        validate_target_capabilities(target, dali_amrn::v2::ABI_VERSION, Some(4))
            .expect("F405 declares relocation support for v4");
    }

    #[test]
    fn validates_package_id_and_version_values() {
        assert_eq!(
            super::parse_package_id("00112233445566778899AABBCCDDEEFF")
                .expect("package id should parse")[0],
            0
        );
        assert!(super::parse_package_id("00").is_err());
        assert_eq!(
            super::parse_version("1.2.3", "version")
                .expect("version should parse")
                .minor,
            2
        );
        assert!(super::parse_version("1.2", "version").is_err());
    }

    #[test]
    fn accepts_declared_f405_isolation_capabilities() {
        let target = super::target_profile("f405").expect("F405 target is declared");
        validate_target_capabilities(target, dali_amrn::v2::ABI_VERSION, Some(3))
            .expect("F405 declares MPU and relocation support");
    }
}
