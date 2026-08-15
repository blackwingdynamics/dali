use std::{
    env, fs,
    path::{Path, PathBuf},
};

const LIST_COMMAND: &str = "list";
const INFO_COMMAND: &str = "info";
const FIELD_FLAG: &str = "--field";
const PROBE_CHIP_FIELD: &str = "probe-chip";
const SCAFFOLD_COMMAND: &str = "scaffold";
const OUTPUT_FLAG: &str = "--output";
const BOARD_DIRECTORY: &str = "kernel/src/board";
const DOCUMENTATION_DIRECTORY: &str = "docs/boards";
const BACKEND_SUFFIX: &str = ".rs.template";
const DOCUMENTATION_SUFFIX: &str = ".md";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(1).map(String::as_str) {
        Some(LIST_COMMAND) if arguments.len() == 2 => print_targets(),
        Some(INFO_COMMAND) => print_info(arguments),
        Some(SCAFFOLD_COMMAND) => scaffold(arguments),
        _ => Err(usage()),
    }
}

fn print_info(arguments: &[String]) -> Result<(), String> {
    if arguments.len() == 5
        && arguments.get(3).map(String::as_str) == Some(FIELD_FLAG)
        && arguments.get(4).map(String::as_str) == Some(PROBE_CHIP_FIELD)
    {
        let profile_name = &arguments[2];
        let profile = dali_targets::find_board(profile_name)
            .ok_or_else(|| format!("unknown board profile '{profile_name}'"))?;
        return profile
            .probe_chip
            .ok_or_else(|| format!("board profile '{profile_name}' has no probe chip"))
            .map(|chip| println!("{chip}"));
    }
    Err(usage())
}

fn scaffold(arguments: &[String]) -> Result<(), String> {
    let profile_name = arguments.get(2).ok_or_else(usage)?;
    let output_root = match arguments.len() {
        3 => workspace_root()?,
        5 if arguments.get(3).map(String::as_str) == Some(OUTPUT_FLAG) => {
            resolve_output_root(&PathBuf::from(&arguments[4]))?
        }
        _ => return Err(usage()),
    };
    let profile = dali_targets::find_board(profile_name)
        .ok_or_else(|| format!("unknown board profile '{profile_name}'"))?;
    let backend = output_root
        .join(BOARD_DIRECTORY)
        .join(format!("{profile_name}{BACKEND_SUFFIX}"));
    let documentation = output_root
        .join(DOCUMENTATION_DIRECTORY)
        .join(format!("{profile_name}{DOCUMENTATION_SUFFIX}"));
    reject_existing(&backend)?;
    reject_existing(&documentation)?;
    write_scaffold(&backend, &backend_template(profile))?;
    write_scaffold(&documentation, &documentation_template(profile))?;
    println!("Generated board scaffold for '{profile_name}'");
    println!("- {}", backend.display());
    println!("- {}", documentation.display());
    Ok(())
}

fn workspace_root() -> Result<PathBuf, String> {
    let current = env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?;
    current
        .ancestors()
        .find(|candidate| {
            candidate.join("Cargo.toml").is_file() && candidate.join("targets").is_dir()
        })
        .map(Path::to_path_buf)
        .ok_or_else(|| "Dali workspace root was not found; pass --output <path>".to_owned())
}

fn resolve_output_root(path: &Path) -> Result<PathBuf, String> {
    let current = env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?;
    let root = if path.is_absolute() {
        path.to_path_buf()
    } else {
        current.join(path)
    };
    if !root.exists() {
        return Err(format!("output root does not exist: {}", root.display()));
    }
    Ok(root)
}

fn reject_existing(path: &Path) -> Result<(), String> {
    if path.exists() {
        Err(format!(
            "refusing to overwrite existing file {}",
            path.display()
        ))
    } else {
        Ok(())
    }
}

fn write_scaffold(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid output path {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    fs::write(path, contents).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn backend_template(profile: &dali_targets::TargetProfile) -> String {
    format!(
        "//! Review scaffold generated from targets/{name}.toml.\n\
//! Map the profile to typed HAL resources before adding this backend to board/mod.rs.\n\
//!\n\
//! Required review areas:\n\
//! - clock and reset configuration\n\
//! - typed GPIO and alternate-function mapping\n\
//! - peripheral singleton ownership\n\
//! - DMA and interrupt ownership\n\
//! - unsafe safety invariants\n\
\n\
use dali_targets::{constant};\n\
\n\
/// Declarative profile selected for this backend.\n\
pub const TARGET_PROFILE: &dali_targets::TargetProfile = &{constant};\n",
        name = profile.name,
        constant = profile.registry_constant
    )
}

fn documentation_template(profile: &dali_targets::TargetProfile) -> String {
    format!(
        "# {board}\n\n\
Generated from targets/{name}.toml. Complete and review this document before accepting the kernel backend.\n\n\
- MCU: {mcu}\n\
- Rust target: {target}\n\
- AMRN target ID: 0x{id:02X}\n\
- ABI version: {abi}\n\
- Application support: {application_supported}\n\
- Kernel mapping status: pending typed HAL implementation\n\
- Hardware acceptance status: pending\n",
        board = profile.board,
        name = profile.name,
        mcu = profile.mcu,
        target = profile.rust_target,
        id = profile.amrn_target_id,
        abi = profile.abi_version,
        application_supported = profile.application_supported
    )
}

fn print_targets() -> Result<(), String> {
    println!("Supported Dali targets:");
    for target in dali_targets::SUPPORTED_TARGETS {
        println!("- {}", target.name);
        println!("  board: {}", target.board);
        println!("  mcu: {}", target.mcu);
        println!("  rust_target: {}", target.rust_target);
        println!("  amrn_target_id: 0x{:02X}", target.amrn_target_id);
        println!("  abi_version: {}", target.abi_version);
    }
    Ok(())
}

fn usage() -> String {
    "usage:\n  dali target list\n  dali target info <profile> --field probe-chip\n  dali target scaffold <profile> [--output <workspace-root>]"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{backend_template, documentation_template};
    use dali_targets::SUPPORTED_TARGETS;

    #[test]
    fn renders_reviewable_scaffolds_from_profile_metadata() {
        let profile = &SUPPORTED_TARGETS[0];
        assert!(backend_template(profile).contains(profile.registry_constant));
        assert!(documentation_template(profile).contains(profile.board));
    }
}
