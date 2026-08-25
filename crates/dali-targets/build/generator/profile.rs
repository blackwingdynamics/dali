use super::super::manifest::Manifest;
use super::super::render::{constant_name, string_literal};
use super::{authentication, hardware};

pub(super) fn generate_profile(manifest: &Manifest) -> String {
    let profile = &manifest.profile;
    format!(
        "pub const {constant}: TargetProfile = TargetProfile {{ name: {name}, backend: {backend}, registry_constant: {constant_literal}, board: {board}, mcu: {mcu}, rust_target: {target}, kernel_binary: {kernel_binary}, kernel_elf: {kernel_elf}, probe_chip: {probe_chip}, dfu: {dfu}, application_supported: {application_supported}, amrn_target_id: {id}, abi_version: {abi}, capabilities: {capabilities}, clock: {clock}, i2c: {i2c}, display: {display}, driver_probe: {driver_probe}, memory: {memory}, status_led: {led}, user_key: {user_key}, usb: {usb}, storage: {storage}, scheduler: {scheduler}, watchdog: {watchdog}, authentication: {authentication} }};",
        constant = constant_name(&profile.name),
        constant_literal = string_literal(&constant_name(&profile.name)),
        name = string_literal(&profile.name),
        backend = string_literal(&profile.backend),
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
            .map(hardware::generate_dfu)
            .map_or_else(|| "None".to_owned(), |identity| format!("Some({identity})")),
        application_supported = profile.application_supported,
        id = profile.amrn_target_id,
        abi = profile.abi_version,
        capabilities = hardware::generate_capabilities(&manifest.capabilities),
        clock = hardware::generate_clock(&manifest.clock),
        i2c = hardware::generate_i2c(&manifest.i2c),
        display = hardware::generate_display(&manifest.display),
        driver_probe = hardware::generate_driver_probe(&manifest.driver_probe),
        memory = hardware::generate_memory(&manifest.memory),
        led = hardware::generate_pin(&manifest.status_led),
        user_key = hardware::generate_user_key(&manifest.user_key),
        usb = hardware::generate_usb(&manifest.usb),
        storage = manifest
            .storage
            .as_ref()
            .map(hardware::generate_storage)
            .map_or_else(|| "None".to_owned(), |storage| format!("Some({storage})")),
        scheduler = manifest
            .scheduler
            .as_ref()
            .map(hardware::generate_scheduler)
            .map_or_else(
                || "None".to_owned(),
                |scheduler| format!("Some({scheduler})")
            ),
        watchdog = manifest
            .watchdog
            .as_ref()
            .map(hardware::generate_watchdog)
            .map_or_else(|| "None".to_owned(), |watchdog| format!("Some({watchdog})")),
        authentication = authentication::generate_authentication(&manifest.authentication),
    )
}
