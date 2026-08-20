use std::process::Command;

const TARGET_FLAG: &str = "--target";
const PROBE_RS_COMMAND: &str = "probe-rs";
const ATTACH_ARGUMENT: &str = "attach";
const CHIP_ARGUMENT: &str = "--chip";

pub(super) const ATTACH_COMMAND: &str = "attach";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let target_name = parse_target(arguments)?;
    let target = dali_targets::find_board(target_name)
        .ok_or_else(|| format!("unknown board profile '{target_name}'"))?;
    let chip = target
        .probe_chip
        .ok_or_else(|| format!("target '{target_name}' has no probe chip configuration"))?;
    let status = Command::new(PROBE_RS_COMMAND)
        .args([ATTACH_ARGUMENT, CHIP_ARGUMENT, chip])
        .status()
        .map_err(|error| format!("failed to start {PROBE_RS_COMMAND}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{PROBE_RS_COMMAND} exited with {status}"))
    }
}

fn parse_target(arguments: &[String]) -> Result<&str, String> {
    if arguments.len() != 4 || arguments.get(2).map(String::as_str) != Some(TARGET_FLAG) {
        return Err(usage());
    }
    arguments.get(3).map(String::as_str).ok_or_else(usage)
}

fn usage() -> String {
    "usage: dali device attach --target <target>".to_owned()
}

#[cfg(test)]
mod tests {
    use super::parse_target;

    #[test]
    fn accepts_the_documented_target_form() {
        let arguments = ["device", "attach", "--target", "f405"].map(str::to_owned);
        assert_eq!(parse_target(&arguments), Ok("f405"));
    }

    #[test]
    fn rejects_missing_or_unknown_flags() {
        let missing = ["device", "attach"].map(str::to_owned);
        let wrong_flag = ["device", "attach", "--profile", "f405"].map(str::to_owned);
        assert!(parse_target(&missing).is_err());
        assert!(parse_target(&wrong_flag).is_err());
    }
}
