const FIELD_FLAG: &str = "--field";
const PROBE_CHIP_FIELD: &str = "probe-chip";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let profile_name = arguments.get(2).ok_or_else(usage)?;
    let profile = dali_targets::find_board(profile_name)
        .ok_or_else(|| format!("unknown board profile '{profile_name}'"))?;
    if arguments.len() == 3 {
        println!("{}", render_profile(profile));
        return Ok(());
    }
    if arguments.len() == 5
        && arguments.get(3).map(String::as_str) == Some(FIELD_FLAG)
        && arguments.get(4).map(String::as_str) == Some(PROBE_CHIP_FIELD)
    {
        return profile
            .probe_chip
            .ok_or_else(|| format!("board profile '{profile_name}' has no probe chip"))
            .map(|chip| println!("{chip}"));
    }
    Err(usage())
}

pub(super) fn render_profile(profile: &dali_targets::TargetProfile) -> String {
    let storage = profile.storage.map_or_else(
        || "storage: not declared".to_owned(),
        |storage| {
            format!(
                "storage: {} ({}-bit), clock {}, command {}, data [{}, {}, {}, {}]",
                storage.controller,
                storage.bus_width,
                render_pin(storage.clock),
                render_pin(storage.command),
                render_pin(storage.data[0]),
                render_pin(storage.data[1]),
                render_pin(storage.data[2]),
                render_pin(storage.data[3]),
            )
        },
    );
    format!(
        "profile: {name}\nbackend: {backend}\nboard: {board}\nmcu: {mcu}\nrust_target: {rust_target}\nprobe_chip: {probe_chip}\ndfu: {dfu}\napplication_supported: {application_supported}\namrn_target_id: 0x{target_id:02X}\nabi_version: {abi}\nclock: {source}, input {input} Hz, system {system} Hz, APB1 {pclk1} Hz, APB2 {pclk2} Hz, USB {usb} Hz\nmemory: kernel 0x{kernel_origin:08X}+{kernel_length}, application 0x{application_origin:08X}+{application_length}, runtime 0x{runtime_origin:08X}+{runtime_length}\nstatus_led: {status_led}\nusb: {usb_controller}, D- {usb_dm}, D+ {usb_dp}\n{storage}",
        name = profile.name,
        backend = profile.backend,
        board = profile.board,
        mcu = profile.mcu,
        rust_target = profile.rust_target,
        probe_chip = profile.probe_chip.unwrap_or("not declared"),
        dfu = profile.dfu.map_or_else(
            || "not declared".to_owned(),
            |dfu| format!("0x{:04X}:0x{:04X}", dfu.vendor_id, dfu.product_id),
        ),
        application_supported = profile.application_supported,
        target_id = profile.amrn_target_id,
        abi = profile.abi_version,
        source = profile.clock.source,
        input = profile.clock.input_hz,
        system = profile.clock.system_hz,
        pclk1 = profile.clock.pclk1_hz,
        pclk2 = profile.clock.pclk2_hz,
        usb = profile.clock.usb_hz,
        kernel_origin = profile.memory.kernel_origin,
        kernel_length = profile.memory.kernel_length,
        application_origin = profile.memory.application_origin,
        application_length = profile.memory.application_length,
        runtime_origin = profile.memory.runtime_origin,
        runtime_length = profile.memory.runtime_length,
        status_led = render_pin(profile.status_led),
        usb_controller = profile.usb.controller,
        usb_dm = render_pin(profile.usb.dm),
        usb_dp = render_pin(profile.usb.dp),
        storage = storage,
    )
}

fn render_pin(pin: dali_targets::PinProfile) -> String {
    format!(
        "{}{} AF{} {}",
        pin.port,
        pin.number,
        pin.alternate_function,
        if pin.active_high {
            "active-high"
        } else {
            "active-low"
        }
    )
}

fn usage() -> String {
    "usage: dali target info <profile> [--field probe-chip]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::render_profile;

    #[test]
    fn renders_reference_profile_metadata() {
        let profile = dali_targets::find_board("f405").expect("generated F405 profile");
        let output = render_profile(profile);
        assert!(output.contains("board: WeAct Studio STM32F405RGT6 Core Board"));
        assert!(output.contains("dfu: 0x0483:0xDF11"));
        assert!(output.contains("status_led: PB2 AF0 active-high"));
        assert!(output.contains("storage: SDIO (4-bit)"));
    }
}
