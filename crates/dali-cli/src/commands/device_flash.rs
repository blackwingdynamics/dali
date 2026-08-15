use std::{env, fs, path::PathBuf, process::Command};

const TRANSPORT_FLAG: &str = "--transport";
const DFU_TRANSPORT: &str = "dfu";
const TARGET_FLAG: &str = "--target";
const INPUT_FLAG: &str = "--input";
const DFU_UTIL_COMMAND: &str = "dfu-util";
const DEVICE_ARGUMENT: &str = "-d";
const ALTERNATE_ARGUMENT: &str = "-a";
const ADDRESS_ARGUMENT: &str = "-s";
const DOWNLOAD_ARGUMENT: &str = "-D";
const DOWNLOAD_ADDRESS_SEPARATOR: &str = ":";
const ADDRESS_PREFIX: &str = "0x";
const LEAVE_ARGUMENT: &str = "leave";
const FLASH_ARGUMENT_COUNT: usize = 8;
const COMMAND_INDEX: usize = 1;
const TRANSPORT_FLAG_INDEX: usize = 2;
const TRANSPORT_VALUE_INDEX: usize = 3;
const TARGET_FLAG_INDEX: usize = 4;
const TARGET_VALUE_INDEX: usize = 5;
const INPUT_FLAG_INDEX: usize = 6;
const INPUT_VALUE_INDEX: usize = 7;

pub(super) const FLASH_COMMAND: &str = "flash";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let request = FlashRequest::parse(arguments)?;
    let target = dali_targets::find_board(&request.target)
        .ok_or_else(|| format!("unknown target profile '{}'.", request.target))?;
    let dfu = target
        .dfu
        .ok_or_else(|| format!("target '{}' has no DFU configuration.", target.name))?;
    let input = request
        .input
        .or_else(|| default_input_path(target))
        .ok_or_else(|| {
            "cannot determine the default firmware path; use --input <firmware>.".to_owned()
        })?;
    let metadata =
        fs::metadata(&input).map_err(|error| format!("cannot read firmware '{input}': {error}"))?;
    if !metadata.is_file() {
        return Err(format!("firmware input is not a regular file: {input}"));
    }
    let status = Command::new(DFU_UTIL_COMMAND)
        .args(download_arguments(dfu, &input))
        .status()
        .map_err(|error| format!("failed to start {DFU_UTIL_COMMAND}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{DFU_UTIL_COMMAND} exited with {status}"))
    }
}

struct FlashRequest {
    target: String,
    input: Option<String>,
}

impl FlashRequest {
    fn parse(arguments: &[String]) -> Result<Self, String> {
        match arguments {
            [_, command, target] if command == FLASH_COMMAND && !target.is_empty() => Ok(Self {
                target: target.clone(),
                input: None,
            }),
            [_, command, target, flag, input]
                if command == FLASH_COMMAND
                    && flag == INPUT_FLAG
                    && !target.is_empty()
                    && !input.is_empty() =>
            {
                Ok(Self {
                    target: target.clone(),
                    input: Some(input.clone()),
                })
            }
            _ if arguments.len() == FLASH_ARGUMENT_COUNT
                && arguments.get(COMMAND_INDEX).map(String::as_str) == Some(FLASH_COMMAND)
                && arguments.get(TRANSPORT_FLAG_INDEX).map(String::as_str)
                    == Some(TRANSPORT_FLAG)
                && arguments.get(TRANSPORT_VALUE_INDEX).map(String::as_str)
                    == Some(DFU_TRANSPORT)
                && arguments.get(TARGET_FLAG_INDEX).map(String::as_str) == Some(TARGET_FLAG)
                && arguments.get(INPUT_FLAG_INDEX).map(String::as_str) == Some(INPUT_FLAG) =>
            {
                let target = arguments[TARGET_VALUE_INDEX].clone();
                let input = arguments[INPUT_VALUE_INDEX].clone();
                if target.is_empty() || input.is_empty() {
                    return Err(usage());
                }
                Ok(Self {
                    target,
                    input: Some(input),
                })
            }
            _ => Err(usage()),
        }
    }
}

fn default_input_path(target: &dali_targets::TargetProfile) -> Option<String> {
    let binary = target.kernel_binary?;
    let workspace = workspace_root(env::current_dir().ok()?)?;
    Some(
        workspace
            .join("target")
            .join(target.rust_target)
            .join("debug")
            .join(binary)
            .display()
            .to_string(),
    )
}

fn workspace_root(mut directory: PathBuf) -> Option<PathBuf> {
    loop {
        if directory.join("Cargo.toml").is_file() && directory.join("targets").is_dir() {
            return Some(directory);
        }
        if !directory.pop() {
            return None;
        }
    }
}

fn download_arguments(dfu: dali_targets::DfuProfile, input: &str) -> [String; 8] {
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

fn usage() -> String {
    "usage:\n  dali device flash <target> [--input <firmware>]\n  dali device flash --transport dfu --target <target> --input <firmware>".to_owned()
}

#[cfg(test)]
mod tests {
    use super::{FlashRequest, download_arguments};

    #[test]
    fn parses_the_explicit_dfu_request() {
        let arguments = [
            "device",
            "flash",
            "--transport",
            "dfu",
            "--target",
            "f405",
            "--input",
            "firmware.bin",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        let request = FlashRequest::parse(&arguments).expect("documented flash request");
        assert_eq!(request.target, "f405");
        assert_eq!(request.input.as_deref(), Some("firmware.bin"));
    }

    #[test]
    fn accepts_the_short_request_without_an_input_path() {
        let arguments = ["device", "flash", "f405"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let request = FlashRequest::parse(&arguments).expect("short flash request");
        assert_eq!(request.target, "f405");
        assert_eq!(request.input, None);
    }

    #[test]
    fn accepts_a_target_with_an_explicit_input_path() {
        let arguments = ["device", "flash", "f405", "--input", "custom.bin"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let request = FlashRequest::parse(&arguments).expect("target and input request");
        assert_eq!(request.target, "f405");
        assert_eq!(request.input.as_deref(), Some("custom.bin"));
    }

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

    #[test]
    fn rejects_non_dfu_transport() {
        let arguments = [
            "device",
            "flash",
            "--transport",
            "probe",
            "--target",
            "f405",
            "--input",
            "firmware.bin",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        assert!(FlashRequest::parse(&arguments).is_err());
    }
}
