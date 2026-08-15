use std::{env, path::Path, process::Command};

use dali_targets::SUPPORTED_TARGETS;

const RUSTC_COMMAND: &str = "rustc";
const CARGO_COMMAND: &str = "cargo";
const RUSTUP_COMMAND: &str = "rustup";
const CARGO_OBJCOPY_COMMAND: &str = "cargo objcopy";
const PROBE_RS_COMMAND: &str = "probe-rs";
const DFU_UTIL_COMMAND: &str = "dfu-util";
const PICOCOM_COMMAND: &str = "picocom";

struct Check {
    label: &'static str,
    required: bool,
    result: Result<String, String>,
}

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    if arguments.len() != 1 {
        return Err("usage:\n  dali doctor".to_owned());
    }
    let mut checks = vec![
        Check {
            label: RUSTC_COMMAND,
            required: true,
            result: command_version(RUSTC_COMMAND),
        },
        Check {
            label: CARGO_COMMAND,
            required: true,
            result: command_version(CARGO_COMMAND),
        },
        Check {
            label: CARGO_OBJCOPY_COMMAND,
            required: true,
            result: cargo_objcopy_version(),
        },
        Check {
            label: PROBE_RS_COMMAND,
            required: false,
            result: command_version(PROBE_RS_COMMAND),
        },
        Check {
            label: DFU_UTIL_COMMAND,
            required: false,
            result: command_version(DFU_UTIL_COMMAND),
        },
        Check {
            label: PICOCOM_COMMAND,
            required: false,
            result: command_version(PICOCOM_COMMAND),
        },
    ];
    for target in SUPPORTED_TARGETS {
        checks.insert(
            2,
            Check {
                label: target.rust_target,
                required: true,
                result: installed_target(target.rust_target),
            },
        );
    }
    let failures = print_checks(&checks);
    if failures == 0 {
        Ok(())
    } else {
        Err(format!("doctor found {failures} required problem(s)"))
    }
}

fn command_version(command: &str) -> Result<String, String> {
    let executable = command.split_whitespace().next().unwrap_or(command);
    let output = Command::new(executable)
        .arg("--version")
        .output()
        .map_err(|error| error.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if let Some(line) = stdout
        .lines()
        .chain(stderr.lines())
        .find(|line| !line.trim().is_empty())
    {
        return Ok(if output.status.success() {
            line.to_owned()
        } else {
            "available".to_owned()
        });
    }
    if executable_on_path(executable) {
        Ok("available".to_owned())
    } else {
        Err(format!("command exited with {}", output.status))
    }
}

fn executable_on_path(executable: &str) -> bool {
    let path = Path::new(executable);
    if path.components().count() > 1 {
        return path.is_file();
    }
    env::var_os("PATH")
        .map(|paths| env::split_paths(&paths).any(|path| path.join(executable).is_file()))
        .unwrap_or(false)
}

fn installed_target(required_target: &str) -> Result<String, String> {
    let output = Command::new(RUSTUP_COMMAND)
        .args(["target", "list", "--installed"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("command exited with {}", output.status));
    }
    let installed = String::from_utf8_lossy(&output.stdout);
    if installed
        .lines()
        .any(|target| target.trim() == required_target)
    {
        Ok("installed".to_owned())
    } else {
        Err("not installed".to_owned())
    }
}

fn cargo_objcopy_version() -> Result<String, String> {
    let output = Command::new(CARGO_COMMAND)
        .args(["objcopy", "--version"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("command exited with {}", output.status));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().next().unwrap_or("available").to_owned())
}

fn print_checks(checks: &[Check]) -> usize {
    let mut failures = 0;
    for check in checks {
        match &check.result {
            Ok(detail) => println!("[OK] {}: {detail}", check.label),
            Err(error) if check.required => {
                failures += 1;
                println!("[FAIL] {}: {error}", check.label);
            }
            Err(error) => println!("[WARN] {}: {error}", check.label),
        }
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::{command_version, installed_target};
    use dali_targets::SUPPORTED_TARGETS;

    #[test]
    fn detects_rustc() {
        assert!(command_version("rustc").is_ok());
    }

    #[test]
    fn checks_the_configured_embedded_target() {
        let result = installed_target(SUPPORTED_TARGETS[0].rust_target);
        assert!(
            result.is_ok(),
            "embedded target should be installed: {result:?}"
        );
    }
}
