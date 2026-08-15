use std::{fs, process::Command};

const DFU_TRANSPORT: &str = "dfu";
const PROBE_TRANSPORT: &str = "probe";
const DFU_UTIL_COMMAND: &str = "dfu-util";
const PROBE_RS_COMMAND: &str = "probe-rs";
const DEVICE_ARGUMENT: &str = "-d";
const ALTERNATE_ARGUMENT: &str = "-a";
const ADDRESS_ARGUMENT: &str = "-s";
const DOWNLOAD_ARGUMENT: &str = "-D";
const DOWNLOAD_ADDRESS_SEPARATOR: &str = ":";
const ADDRESS_PREFIX: &str = "0x";
const LEAVE_ARGUMENT: &str = "leave";
const DOWNLOAD_ARGUMENT_COUNT: usize = 8;

pub(super) fn flash(
    target: &dali_targets::TargetProfile,
    transport: &str,
    input: &str,
) -> Result<(), String> {
    match transport {
        DFU_TRANSPORT => flash_dfu(target, input),
        PROBE_TRANSPORT => flash_probe(target, input),
        _ => Err(format!("unsupported flash transport: {transport}")),
    }
}

fn flash_dfu(target: &dali_targets::TargetProfile, input: &str) -> Result<(), String> {
    let dfu = target
        .dfu
        .ok_or_else(|| format!("target '{}' has no DFU configuration.", target.name))?;
    ensure_regular_file(input)?;
    let status = Command::new(DFU_UTIL_COMMAND)
        .args(download_arguments(dfu, input))
        .status()
        .map_err(|error| format!("failed to start {DFU_UTIL_COMMAND}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{DFU_UTIL_COMMAND} exited with {status}"))
    }
}

fn flash_probe(target: &dali_targets::TargetProfile, input: &str) -> Result<(), String> {
    let chip = target
        .probe_chip
        .ok_or_else(|| format!("target '{}' has no probe chip configuration.", target.name))?;
    ensure_regular_file(input)?;
    let status = Command::new(PROBE_RS_COMMAND)
        .args(["run", "--chip", chip, input])
        .status()
        .map_err(|error| format!("failed to start {PROBE_RS_COMMAND}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{PROBE_RS_COMMAND} exited with {status}"))
    }
}

fn ensure_regular_file(input: &str) -> Result<(), String> {
    let metadata =
        fs::metadata(input).map_err(|error| format!("cannot read firmware '{input}': {error}"))?;
    if metadata.is_file() {
        Ok(())
    } else {
        Err(format!("firmware input is not a regular file: {input}"))
    }
}

fn download_arguments(
    dfu: dali_targets::DfuProfile,
    input: &str,
) -> [String; DOWNLOAD_ARGUMENT_COUNT] {
    let device = format!("{:04X}:{:04X}", dfu.vendor_id, dfu.product_id);
    let address = if dfu.leave {
        format!(
            "{ADDRESS_PREFIX}{:08X}{DOWNLOAD_ADDRESS_SEPARATOR}{LEAVE_ARGUMENT}",
            dfu.address
        )
    } else {
        format!("{ADDRESS_PREFIX}{:08X}", dfu.address)
    };
    [
        DEVICE_ARGUMENT.to_owned(),
        device,
        ALTERNATE_ARGUMENT.to_owned(),
        dfu.alternate.to_string(),
        ADDRESS_ARGUMENT.to_owned(),
        address,
        DOWNLOAD_ARGUMENT.to_owned(),
        input.to_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use super::download_arguments;

    #[test]
    fn renders_manifest_driven_download_arguments() {
        let dfu = dali_targets::DfuProfile {
            vendor_id: 0x0483,
            product_id: 0xDF11,
            address: 0x0800_0000,
            alternate: 0,
            leave: true,
        };
        assert_eq!(
            download_arguments(dfu, "firmware.bin"),
            [
                "-d",
                "0483:DF11",
                "-a",
                "0",
                "-s",
                "0x08000000:leave",
                "-D",
                "firmware.bin"
            ]
            .map(str::to_owned)
        );
    }
}
