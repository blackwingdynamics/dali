use std::path::{Path, PathBuf};

use super::build::{
    ApplicationManifest, DEBUG_OUTPUT_DIRECTORY, RELEASE_PROFILE, cargo_command, features_for_abi,
    run_command,
};

pub(super) const CODE_SECTION: &str = ".dali_code";
pub(super) const DATA_SECTION: &str = ".dali_data";
pub(super) const BSS_START_SYMBOL: &str = "__dali_bss_start";
pub(super) const BSS_END_SYMBOL: &str = "__dali_bss_end";
const CODE_SUFFIX: &str = ".code.bin";
const DATA_SUFFIX: &str = ".data.bin";

pub(super) struct ArtifactContext<'a> {
    pub(super) project_directory: &'a Path,
    pub(super) manifest: &'a Path,
    pub(super) target: &'a str,
    pub(super) release: bool,
    pub(super) binary: &'a str,
    pub(super) features: &'a str,
}

pub(super) fn code_path(
    project_directory: &Path,
    target: &str,
    release: bool,
    name: &str,
) -> PathBuf {
    artifact_path(project_directory, target, release, name, CODE_SUFFIX)
}

pub(super) fn data_path(
    project_directory: &Path,
    target: &str,
    release: bool,
    name: &str,
) -> PathBuf {
    artifact_path(project_directory, target, release, name, DATA_SUFFIX)
}

pub(super) fn extract_v3_sections(
    project_directory: &Path,
    manifest_path: &Path,
    target: &str,
    release: bool,
    manifest: &ApplicationManifest,
    code_output: &Path,
    data_output: &Path,
) -> Result<u32, String> {
    let features = features_for_abi(manifest.abi_version.unwrap_or(dali_amrn::v2::ABI_VERSION))?;
    let context = ArtifactContext {
        project_directory,
        manifest: manifest_path,
        target,
        release,
        binary: &manifest.name,
        features: &features,
    };
    run_objcopy_section(&context, CODE_SECTION, code_output)?;
    run_objcopy_section(&context, DATA_SECTION, data_output)?;
    read_zero_init_size(&context)
}

pub(super) fn read_zero_init_size(context: &ArtifactContext<'_>) -> Result<u32, String> {
    let symbols = run_nm(context)?;
    bss_size(&symbols)
}

pub(super) fn bss_size(symbols: &str) -> Result<u32, String> {
    let start = symbol_address(symbols, BSS_START_SYMBOL)?;
    let end = symbol_address(symbols, BSS_END_SYMBOL)?;
    end.checked_sub(start)
        .ok_or_else(|| "ABI v3 BSS symbols are out of order".to_owned())
}

fn run_objcopy_section(
    context: &ArtifactContext<'_>,
    section: &str,
    output: &Path,
) -> Result<(), String> {
    let mut command = cargo_command("objcopy", context.project_directory, context.manifest);
    command.args([
        "--features",
        context.features,
        "--target",
        context.target,
        "--bin",
        context.binary,
    ]);
    if context.release {
        command.arg("--release");
    }
    command.args(["--", "-O", "binary", "--only-section", section]);
    command.arg(output);
    run_command(&mut command, "cargo objcopy")
}

fn run_nm(context: &ArtifactContext<'_>) -> Result<String, String> {
    let mut command = cargo_command("nm", context.project_directory, context.manifest);
    command.args([
        "--features",
        context.features,
        "--target",
        context.target,
        "--bin",
        context.binary,
    ]);
    if context.release {
        command.arg("--release");
    }
    let output = command
        .output()
        .map_err(|error| format!("cannot run cargo nm: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo nm failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo nm returned invalid UTF-8: {error}"))
}

fn symbol_address(symbols: &str, symbol: &str) -> Result<u32, String> {
    let line = symbols
        .lines()
        .find(|line| line.split_whitespace().last() == Some(symbol))
        .ok_or_else(|| format!("cargo nm did not report ABI v3 symbol `{symbol}`"))?;
    let address = line
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("cargo nm returned an invalid line for `{symbol}`"))?;
    u32::from_str_radix(address.trim_start_matches("0x"), 16)
        .map_err(|_| format!("cargo nm returned an invalid address for `{symbol}`"))
}

fn artifact_path(
    project_directory: &Path,
    target: &str,
    release: bool,
    name: &str,
    suffix: &str,
) -> PathBuf {
    let profile = if release {
        RELEASE_PROFILE
    } else {
        DEBUG_OUTPUT_DIRECTORY
    };
    project_directory
        .join("target")
        .join(target)
        .join(profile)
        .join(format!("{name}{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::bss_size;

    #[test]
    fn computes_bss_size_from_nm_symbols() {
        let symbols = "00002000 00000000 T __dali_bss_start\n00002020 00000000 B __dali_bss_end\n";
        assert_eq!(bss_size(symbols).expect("symbols should parse"), 0x20);
    }

    #[test]
    fn rejects_missing_bss_symbols() {
        assert!(bss_size("00002020 00000000 B __dali_bss_end\n").is_err());
    }
}
