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
    pub(super) target_profile: String,
    pub(super) profile: String,
    pub(super) entry_offset: u32,
    pub(super) abi_version: Option<u8>,
    pub(super) format_version: Option<u8>,
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
    let release = cargo_profile_is_release(&manifest.profile)?;
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
        let bss_size = super::artifacts::extract_v3_sections(
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
    Ok(ApplicationManifest {
        name,
        target_profile,
        profile,
        entry_offset,
        abi_version,
        format_version,
    })
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
    match abi_version {
        dali_amrn::ABI_VERSION => Ok(EMBEDDED_PAYLOAD_FEATURE.to_owned()),
        dali_amrn::v2::ABI_VERSION => Ok(format!("{EMBEDDED_PAYLOAD_FEATURE},abi-v3")),
        _ => Err(format!("unsupported application ABI version {abi_version}")),
    }
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
    use super::{DEVELOPMENT_PROFILE, RELEASE_PROFILE, cargo_profile_is_release, parse_manifest};

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
    fn parses_explicit_relocation_format() {
        let manifest = parse_manifest(
            "name = \"telemetry\"\ntarget_profile = \"f405\"\nentry_offset = 0\nformat_version = 3",
        )
        .expect("manifest should parse");
        assert_eq!(manifest.format_version, Some(3));
    }
}
