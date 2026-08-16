use std::{
    env, fs,
    path::{Path, PathBuf},
};

const LIST_COMMAND: &str = "list";
const SCAFFOLD_COMMAND: &str = "scaffold";
const OUTPUT_FLAG: &str = "--output";
const BOARD_DIRECTORY: &str = "kernel/src/board";
const DOCUMENTATION_DIRECTORY: &str = "docs/boards";
const BACKEND_SUFFIX: &str = ".rs.template";
const DOCUMENTATION_SUFFIX: &str = ".md";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(1).map(String::as_str) {
        Some(LIST_COMMAND) if arguments.len() == 2 => print_targets(),
        Some("info") => crate::commands::target_info::run(arguments),
        Some(SCAFFOLD_COMMAND) => scaffold(arguments),
        _ => Err(usage()),
    }
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
    let metadata = metadata_template(profile);
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
{metadata}\n\
use dali_targets::{constant};\n\
\n\
/// Declarative profile selected for this backend.\n\
pub const TARGET_PROFILE: &dali_targets::TargetProfile = &{constant};\n",
        name = profile.name,
        constant = profile.registry_constant,
        metadata = metadata
    )
}

fn metadata_template(profile: &dali_targets::TargetProfile) -> String {
    let storage = profile.storage.map_or_else(
        || "pub const STORAGE_SUPPORTED: bool = false;\n".to_owned(),
        |storage| {
            format!(
                "pub const STORAGE_SUPPORTED: bool = true;\n\
pub const STORAGE_CONTROLLER: &str = {controller:?};\n\
pub const STORAGE_BUS_WIDTH: u8 = {bus_width};\n\
{clock}{command}{data}",
                controller = storage.controller,
                bus_width = storage.bus_width,
                clock = pin_constants("STORAGE_CLOCK", storage.clock),
                command = pin_constants("STORAGE_COMMAND", storage.command),
                data = storage
                    .data
                    .iter()
                    .enumerate()
                    .map(|(index, pin)| pin_constants(&format!("STORAGE_DATA_{index}"), *pin))
                    .collect::<String>()
            )
        },
    );
    let dfu = profile.dfu.map_or_else(
        || "pub const DFU_SUPPORTED: bool = false;\n".to_owned(),
        |dfu| {
            format!(
                "pub const DFU_SUPPORTED: bool = true;\n\
pub const DFU_VENDOR_ID: u16 = 0x{vendor_id:04X};\n\
pub const DFU_PRODUCT_ID: u16 = 0x{product_id:04X};\n",
                vendor_id = dfu.vendor_id,
                product_id = dfu.product_id
            )
        },
    );
    format!(
        "pub const PROBE_CHIP: Option<&str> = {probe_chip:?};\n\
{dfu}\
pub const CLOCK_SOURCE: &str = {clock_source:?};\n\
pub const CLOCK_INPUT_HZ: u32 = {input_hz};\n\
pub const SYSTEM_CLOCK_HZ: u32 = {system_hz};\n\
pub const PCLK1_HZ: u32 = {pclk1_hz};\n\
pub const PCLK2_HZ: u32 = {pclk2_hz};\n\
pub const USB_CLOCK_HZ: u32 = {usb_hz};\n\
{memory}{status_led}pub const USB_CONTROLLER: &str = {usb_controller:?};\n\
{usb_dm}{usb_dp}{storage}",
        probe_chip = profile.probe_chip,
        dfu = dfu,
        clock_source = profile.clock.source,
        input_hz = profile.clock.input_hz,
        system_hz = profile.clock.system_hz,
        pclk1_hz = profile.clock.pclk1_hz,
        pclk2_hz = profile.clock.pclk2_hz,
        usb_hz = profile.clock.usb_hz,
        memory = memory_constants(profile),
        status_led = pin_constants("STATUS_LED", profile.status_led),
        usb_controller = profile.usb.controller,
        usb_dm = pin_constants("USB_DM", profile.usb.dm),
        usb_dp = pin_constants("USB_DP", profile.usb.dp),
        storage = storage,
    )
}

fn memory_constants(profile: &dali_targets::TargetProfile) -> String {
    format!(
        "pub const KERNEL_ORIGIN: u32 = 0x{:08X};\n\
pub const KERNEL_LENGTH: u32 = {};\n\
pub const APPLICATION_ORIGIN: u32 = 0x{:08X};\n\
pub const APPLICATION_LENGTH: u32 = {};\n\
pub const RUNTIME_ORIGIN: u32 = 0x{:08X};\n\
pub const RUNTIME_LENGTH: u32 = {};\n\
",
        profile.memory.kernel_origin,
        profile.memory.kernel_length,
        profile.memory.application_origin,
        profile.memory.application_length,
        profile.memory.runtime_origin,
        profile.memory.runtime_length,
    )
}

fn pin_constants(prefix: &str, pin: dali_targets::PinProfile) -> String {
    format!(
        "pub const {prefix}_PORT: &str = {port:?};\n\
pub const {prefix}_NUMBER: u8 = {number};\n\
pub const {prefix}_ALTERNATE_FUNCTION: u8 = {alternate_function};\n\
pub const {prefix}_ACTIVE_HIGH: bool = {active_high};\n\
",
        port = pin.port,
        number = pin.number,
        alternate_function = pin.alternate_function,
        active_high = pin.active_high,
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
    "usage:\n  dali target list\n  dali target info <profile> [--field probe-chip]\n  dali target scaffold <profile> [--output <workspace-root>]"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{backend_template, documentation_template};
    use dali_targets::{SUPPORTED_TARGETS, find_board};

    #[test]
    fn renders_reviewable_scaffolds_from_profile_metadata() {
        let profile = &SUPPORTED_TARGETS[0];
        assert!(backend_template(profile).contains(profile.registry_constant));
        assert!(documentation_template(profile).contains(profile.board));
    }

    #[test]
    fn renders_declared_board_values() {
        let f405 = backend_template(&SUPPORTED_TARGETS[0]);
        assert!(f405.contains("CLOCK_INPUT_HZ: u32 = 8000000"));
        assert!(f405.contains("DFU_VENDOR_ID: u16 = 0x0483"));
        assert!(f405.contains("STATUS_LED_PORT: &str = \"PB\""));
        assert!(f405.contains("STORAGE_DATA_3_NUMBER: u8 = 11"));

        let f411 = backend_template(find_board("f411").expect("generated test profile"));
        assert!(f411.contains("STORAGE_SUPPORTED: bool = false"));
    }
}
