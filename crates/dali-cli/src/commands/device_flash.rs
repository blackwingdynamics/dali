use std::{env, path::PathBuf};

const TRANSPORT_FLAG: &str = "--transport";
const DFU_TRANSPORT: &str = "dfu";
const PROBE_TRANSPORT: &str = "probe";
const TARGET_FLAG: &str = "--target";
const INPUT_FLAG: &str = "--input";
const TARGET_DIRECTORY: &str = "target";
const DEBUG_PROFILE_DIRECTORY: &str = "debug";
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
    let transport = request.transport.as_deref().unwrap_or(DFU_TRANSPORT);
    let input = request
        .input
        .or_else(|| default_input_path(target, transport))
        .ok_or_else(|| {
            "cannot determine the default firmware path; use --input <firmware>.".to_owned()
        })?;
    crate::commands::device_flash_transport::flash(target, transport, &input)
}

struct FlashRequest {
    target: String,
    input: Option<String>,
    transport: Option<String>,
}

impl FlashRequest {
    fn parse(arguments: &[String]) -> Result<Self, String> {
        match arguments {
            [_, command, target] if command == FLASH_COMMAND && !target.is_empty() => Ok(Self {
                target: target.clone(),
                input: None,
                transport: None,
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
                    transport: None,
                })
            }
            [_, command, target, transport_flag, transport]
                if command == FLASH_COMMAND
                    && transport_flag == TRANSPORT_FLAG
                    && !target.is_empty()
                    && !transport.is_empty() =>
            {
                Ok(Self {
                    target: target.clone(),
                    input: None,
                    transport: Some(transport.clone()),
                })
            }
            [
                _,
                command,
                target,
                transport_flag,
                transport,
                input_flag,
                input,
            ] if command == FLASH_COMMAND
                && transport_flag == TRANSPORT_FLAG
                && input_flag == INPUT_FLAG
                && !target.is_empty()
                && !transport.is_empty()
                && !input.is_empty() =>
            {
                Ok(Self {
                    target: target.clone(),
                    input: Some(input.clone()),
                    transport: Some(transport.clone()),
                })
            }
            _ if arguments.len() == FLASH_ARGUMENT_COUNT
                && arguments.get(COMMAND_INDEX).map(String::as_str) == Some(FLASH_COMMAND)
                && arguments.get(TRANSPORT_FLAG_INDEX).map(String::as_str)
                    == Some(TRANSPORT_FLAG)
                && arguments
                    .get(TRANSPORT_VALUE_INDEX)
                    .is_some_and(|value| !value.is_empty())
                && arguments.get(TARGET_FLAG_INDEX).map(String::as_str) == Some(TARGET_FLAG)
                && arguments.get(INPUT_FLAG_INDEX).map(String::as_str) == Some(INPUT_FLAG) =>
            {
                let target = arguments[TARGET_VALUE_INDEX].clone();
                let transport = arguments[TRANSPORT_VALUE_INDEX].clone();
                let input = arguments[INPUT_VALUE_INDEX].clone();
                if target.is_empty() || input.is_empty() {
                    return Err(usage());
                }
                Ok(Self {
                    target,
                    input: Some(input),
                    transport: Some(transport),
                })
            }
            _ => Err(usage()),
        }
    }
}

fn default_input_path(target: &dali_targets::TargetProfile, transport: &str) -> Option<String> {
    let binary = match transport {
        DFU_TRANSPORT => target.kernel_binary?,
        PROBE_TRANSPORT => target.kernel_elf?,
        _ => return None,
    };
    let workspace = workspace_root(env::current_dir().ok()?)?;
    Some(
        workspace
            .join(TARGET_DIRECTORY)
            .join(target.rust_target)
            .join(DEBUG_PROFILE_DIRECTORY)
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

fn usage() -> String {
    "usage:\n  dali device flash <target> [--transport <transport>] [--input <firmware>]\n  dali device flash --transport <transport> --target <target> --input <firmware>"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::FlashRequest;

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
        assert_eq!(request.transport.as_deref(), Some("dfu"));
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
        assert_eq!(request.transport, None);
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
    fn accepts_probe_transport_with_a_target_and_input() {
        let arguments = [
            "device",
            "flash",
            "f405",
            "--transport",
            "probe",
            "--input",
            "kernel.elf",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        let request = FlashRequest::parse(&arguments).expect("probe flash request");
        assert_eq!(request.target, "f405");
        assert_eq!(request.transport.as_deref(), Some("probe"));
        assert_eq!(request.input.as_deref(), Some("kernel.elf"));
    }

    #[test]
    fn parses_non_dfu_transport_for_dispatch() {
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
        let request = FlashRequest::parse(&arguments).expect("probe transport request");
        assert_eq!(request.transport.as_deref(), Some("probe"));
    }
}
