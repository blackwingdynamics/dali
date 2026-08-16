use std::{env, fs, path::PathBuf};

use serde::Deserialize;

const MANIFEST_DIRECTORY: &str = "targets";
const GENERATED_FILE: &str = "target_profiles.rs";
const MINIMUM_MPU_REGION_BYTES: u32 = 32;

#[derive(Debug, Deserialize)]
struct Manifest {
    profile: Profile,
    artifacts: Option<Artifacts>,
    clock: Clock,
    memory: Memory,
    status_led: Pin,
    usb: Usb,
    storage: Option<Storage>,
}

#[derive(Debug, Deserialize)]
struct Artifacts {
    kernel_binary: String,
    kernel_elf: String,
}

#[derive(Debug, Deserialize)]
struct Profile {
    name: String,
    board: String,
    mcu: String,
    rust_target: String,
    probe_chip: Option<String>,
    dfu: Option<Dfu>,
    application_supported: bool,
    amrn_target_id: u8,
    abi_version: u8,
}

#[derive(Debug, Deserialize)]
struct Dfu {
    vendor_id: u16,
    product_id: u16,
    address: u32,
    alternate: u8,
    leave: bool,
}

#[derive(Debug, Deserialize)]
struct Clock {
    source: String,
    input_hz: u32,
    system_hz: u32,
    pclk1_hz: u32,
    pclk2_hz: u32,
    usb_hz: u32,
}

#[derive(Debug, Deserialize)]
struct Memory {
    kernel_origin: u32,
    kernel_length: u32,
    application_origin: u32,
    application_length: u32,
    runtime_origin: u32,
    runtime_length: u32,
    isolation: Option<IsolationMemory>,
}

#[derive(Debug, Deserialize)]
struct IsolationMemory {
    code_origin: u32,
    code_length: u32,
    data_origin: u32,
    data_length: u32,
    stack_length: u32,
    peripheral_origin: Option<u32>,
    peripheral_length: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Pin {
    port: String,
    number: u8,
    alternate_function: u8,
    active_high: bool,
}

#[derive(Debug, Deserialize)]
struct Usb {
    controller: String,
    dm: Pin,
    dp: Pin,
}

#[derive(Debug, Deserialize)]
struct Storage {
    controller: String,
    bus_width: u8,
    clock: Pin,
    command: Pin,
    data: [Pin; 4],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let manifest_directory = crate_directory.join("../..").join(MANIFEST_DIRECTORY);
    let mut paths = manifest_paths(&manifest_directory)?;
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "no target manifests found in {}",
            manifest_directory.display()
        )
        .into());
    }

    let manifests = paths
        .iter()
        .map(|path| read_manifest(path))
        .collect::<Result<Vec<_>, _>>()?;
    validate_manifests(&manifests)?;

    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed={}", manifest_directory.display());
    let generated = generate_registry(&manifests);
    let output = PathBuf::from(env::var("OUT_DIR")?).join(GENERATED_FILE);
    fs::write(output, generated)?;
    Ok(())
}

fn manifest_paths(directory: &PathBuf) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let entries = fs::read_dir(directory)?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "toml")
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn read_manifest(path: &PathBuf) -> Result<Manifest, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let manifest = toml::from_str(&contents)
        .map_err(|error| format!("invalid target manifest {}: {error}", path.display()))?;
    Ok(manifest)
}

fn validate_manifests(manifests: &[Manifest]) -> Result<(), Box<dyn std::error::Error>> {
    for (index, manifest) in manifests.iter().enumerate() {
        if manifest.profile.name.is_empty()
            || manifest.profile.board.is_empty()
            || manifest.profile.mcu.is_empty()
            || manifest.profile.rust_target.is_empty()
        {
            return Err(format!("target manifest {index} has an empty profile field").into());
        }
        if let Some(artifacts) = &manifest.artifacts
            && (artifacts.kernel_binary.is_empty() || artifacts.kernel_elf.is_empty())
        {
            return Err(format!(
                "target manifest {} has an empty kernel artifact",
                manifest.profile.name
            )
            .into());
        }
        if manifest.profile.application_supported
            && (manifest.profile.abi_version == 0 || manifest.profile.amrn_target_id == 0)
        {
            return Err(format!(
                "target manifest {} has an invalid contract identifier",
                manifest.profile.name
            )
            .into());
        }
        if let Some(dfu) = &manifest.profile.dfu
            && (dfu.vendor_id == 0 || dfu.product_id == 0 || dfu.address == 0)
        {
            return Err(format!(
                "target manifest {} has an invalid DFU configuration",
                manifest.profile.name
            )
            .into());
        }
        if let Some(storage) = &manifest.storage
            && (storage.bus_width == 0 || storage.bus_width > 4)
        {
            return Err(format!(
                "target manifest {} has an invalid storage width",
                manifest.profile.name
            )
            .into());
        }
        if let Some(isolation) = &manifest.memory.isolation {
            validate_isolation_memory(manifest, isolation)?;
        }
        for other in &manifests[..index] {
            if other.profile.name == manifest.profile.name {
                return Err(format!("duplicate target profile `{}`", manifest.profile.name).into());
            }
            if manifest.profile.amrn_target_id != 0
                && other.profile.amrn_target_id == manifest.profile.amrn_target_id
            {
                return Err(
                    format!("duplicate AMRN target id for `{}`", manifest.profile.name).into(),
                );
            }
        }
    }
    Ok(())
}

fn validate_isolation_memory(
    manifest: &Manifest,
    isolation: &IsolationMemory,
) -> Result<(), Box<dyn std::error::Error>> {
    let application_end = manifest
        .memory
        .application_origin
        .checked_add(manifest.memory.application_length)
        .ok_or_else(|| {
            format!(
                "target {} application memory overflows",
                manifest.profile.name
            )
        })?;
    let code_end = isolation
        .code_origin
        .checked_add(isolation.code_length)
        .ok_or_else(|| {
            format!(
                "target {} isolation code memory overflows",
                manifest.profile.name
            )
        })?;
    let data_end = isolation
        .data_origin
        .checked_add(isolation.data_length)
        .ok_or_else(|| {
            format!(
                "target {} isolation data memory overflows",
                manifest.profile.name
            )
        })?;
    match (isolation.peripheral_origin, isolation.peripheral_length) {
        (Some(origin), Some(length)) if valid_mpu_region(origin, length) => {}
        _ => {
            return Err(format!(
                "target {} isolation memory must declare an aligned peripheral region",
                manifest.profile.name
            )
            .into());
        }
    }
    if isolation.code_length == 0
        || isolation.data_length == 0
        || isolation.stack_length == 0
        || isolation.stack_length > isolation.data_length
        || isolation.code_origin != manifest.memory.application_origin
        || isolation.data_origin != code_end
        || data_end != application_end
        || !valid_mpu_region(isolation.code_origin, isolation.code_length)
        || !valid_mpu_region(isolation.data_origin, isolation.data_length)
    {
        return Err(format!(
            "target {} isolation memory must cover application memory contiguously",
            manifest.profile.name
        )
        .into());
    }
    Ok(())
}

fn valid_mpu_region(origin: u32, length: u32) -> bool {
    length >= MINIMUM_MPU_REGION_BYTES
        && (length & (length - 1)) == 0
        && origin.is_multiple_of(length)
}

fn generate_registry(manifests: &[Manifest]) -> String {
    let definitions = manifests
        .iter()
        .map(generate_profile)
        .collect::<Vec<_>>()
        .join("\n");
    let names = manifests
        .iter()
        .map(|manifest| constant_name(&manifest.profile.name))
        .collect::<Vec<_>>()
        .join(", ");
    let supported = manifests
        .iter()
        .filter(|manifest| manifest.profile.application_supported)
        .map(|manifest| constant_name(&manifest.profile.name))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{definitions}\n\npub const ALL_TARGETS: &[TargetProfile] = &[{names}];\npub const SUPPORTED_TARGETS: &[TargetProfile] = &[{supported}];\n"
    )
}

fn generate_profile(manifest: &Manifest) -> String {
    let profile = &manifest.profile;
    format!(
        "pub const {constant}: TargetProfile = TargetProfile {{ name: {name}, registry_constant: {constant_literal}, board: {board}, mcu: {mcu}, rust_target: {target}, kernel_binary: {kernel_binary}, kernel_elf: {kernel_elf}, probe_chip: {probe_chip}, dfu: {dfu}, application_supported: {application_supported}, amrn_target_id: {id}, abi_version: {abi}, clock: {clock}, memory: {memory}, status_led: {led}, usb: {usb}, storage: {storage} }};",
        constant = constant_name(&profile.name),
        constant_literal = string_literal(&constant_name(&profile.name)),
        name = string_literal(&profile.name),
        board = string_literal(&profile.board),
        mcu = string_literal(&profile.mcu),
        target = string_literal(&profile.rust_target),
        kernel_binary = manifest
            .artifacts
            .as_ref()
            .map(|artifacts| string_literal(&artifacts.kernel_binary))
            .map_or_else(|| "None".to_owned(), |binary| format!("Some({binary})")),
        kernel_elf = manifest
            .artifacts
            .as_ref()
            .map(|artifacts| string_literal(&artifacts.kernel_elf))
            .map_or_else(|| "None".to_owned(), |elf| format!("Some({elf})")),
        probe_chip = profile
            .probe_chip
            .as_deref()
            .map(string_literal)
            .map_or_else(|| "None".to_owned(), |chip| format!("Some({chip})")),
        dfu = profile
            .dfu
            .as_ref()
            .map(generate_dfu)
            .map_or_else(|| "None".to_owned(), |identity| format!("Some({identity})")),
        application_supported = profile.application_supported,
        id = profile.amrn_target_id,
        abi = profile.abi_version,
        clock = generate_clock(&manifest.clock),
        memory = generate_memory(&manifest.memory),
        led = generate_pin(&manifest.status_led),
        usb = generate_usb(&manifest.usb),
        storage = manifest
            .storage
            .as_ref()
            .map(generate_storage)
            .map_or_else(|| "None".to_owned(), |storage| format!("Some({storage})")),
    )
}

fn generate_dfu(dfu: &Dfu) -> String {
    format!(
        "DfuProfile {{ vendor_id: 0x{:04X}, product_id: 0x{:04X}, address: 0x{:08X}, alternate: {}, leave: {} }}",
        dfu.vendor_id, dfu.product_id, dfu.address, dfu.alternate, dfu.leave
    )
}

fn generate_clock(clock: &Clock) -> String {
    format!(
        "ClockProfile {{ source: {}, input_hz: {}, system_hz: {}, pclk1_hz: {}, pclk2_hz: {}, usb_hz: {} }}",
        string_literal(&clock.source),
        clock.input_hz,
        clock.system_hz,
        clock.pclk1_hz,
        clock.pclk2_hz,
        clock.usb_hz
    )
}

fn generate_memory(memory: &Memory) -> String {
    format!(
        "MemoryProfile {{ kernel_origin: 0x{:08X}, kernel_length: {}, application_origin: 0x{:08X}, application_length: {}, runtime_origin: 0x{:08X}, runtime_length: {}, isolation: {} }}",
        memory.kernel_origin,
        memory.kernel_length,
        memory.application_origin,
        memory.application_length,
        memory.runtime_origin,
        memory.runtime_length,
        memory
            .isolation
            .as_ref()
            .map(generate_isolation_memory)
            .map_or_else(|| "None".to_owned(), |value| format!("Some({value})"))
    )
}

fn generate_isolation_memory(memory: &IsolationMemory) -> String {
    format!(
        "IsolationMemoryProfile {{ code_origin: 0x{:08X}, code_length: {}, data_origin: 0x{:08X}, data_length: {}, stack_length: {}, peripheral_origin: {}, peripheral_length: {} }}",
        memory.code_origin,
        memory.code_length,
        memory.data_origin,
        memory.data_length,
        memory.stack_length,
        memory
            .peripheral_origin
            .map_or_else(|| "None".to_owned(), |value| format!("Some(0x{value:08X})")),
        memory
            .peripheral_length
            .map_or_else(|| "None".to_owned(), |value| format!("Some({value})")),
    )
}

fn generate_pin(pin: &Pin) -> String {
    format!(
        "PinProfile {{ port: {}, number: {}, alternate_function: {}, active_high: {} }}",
        string_literal(&pin.port),
        pin.number,
        pin.alternate_function,
        pin.active_high
    )
}

fn generate_usb(usb: &Usb) -> String {
    format!(
        "UsbProfile {{ controller: {}, dm: {}, dp: {} }}",
        string_literal(&usb.controller),
        generate_pin(&usb.dm),
        generate_pin(&usb.dp)
    )
}

fn generate_storage(storage: &Storage) -> String {
    let data = storage
        .data
        .iter()
        .map(generate_pin)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "StorageProfile {{ controller: {}, bus_width: {}, clock: {}, command: {}, data: [{}] }}",
        string_literal(&storage.controller),
        storage.bus_width,
        generate_pin(&storage.clock),
        generate_pin(&storage.command),
        data
    )
}

fn string_literal(value: &str) -> String {
    format!("{:?}", value)
}

fn constant_name(profile_name: &str) -> String {
    let suffix = profile_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("TARGET_{suffix}")
}
