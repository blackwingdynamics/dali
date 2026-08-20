use std::{path::Path, process::Command};

use dali_device::{Capability, Transport};

pub(super) const CONSOLE_COMMAND: &str = "console";

const PORT_FLAG: &str = "--port";
const PICOCOM_COMMAND: &str = "picocom";
const NO_CONSOLE_MESSAGE: &str =
    "No USB CDC console found. Connect a running Dali device or use --port <path>.";
const MULTIPLE_CONSOLES_MESSAGE: &str =
    "Multiple USB CDC consoles found. Select one with --port <path>.";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let port = requested_port(arguments)?.map_or_else(select_console_port, Ok)?;
    launch_console(&port)
}

fn requested_port(arguments: &[String]) -> Result<Option<String>, String> {
    match arguments {
        [_, command] if command == CONSOLE_COMMAND => Ok(None),
        [_, command, flag, port]
            if command == CONSOLE_COMMAND && flag == PORT_FLAG && !port.is_empty() =>
        {
            Ok(Some(port.clone()))
        }
        _ => Err(usage()),
    }
}

fn select_console_port() -> Result<String, String> {
    let records = super::cdc::discover()?;
    let ports = records
        .iter()
        .filter(|record| record.transport == Transport::Cdc)
        .filter(|record| record.capabilities.contains(&Capability::Console))
        .filter_map(|record| record.path.as_deref())
        .collect::<Vec<_>>();
    match ports.as_slice() {
        [] => Err(NO_CONSOLE_MESSAGE.to_owned()),
        [port] => Ok((*port).to_owned()),
        _ => Err(MULTIPLE_CONSOLES_MESSAGE.to_owned()),
    }
}

fn launch_console(port: &str) -> Result<(), String> {
    if !Path::new(port).exists() {
        return Err(format!("USB CDC console path does not exist: {port}"));
    }
    let status = Command::new(PICOCOM_COMMAND)
        .arg(port)
        .status()
        .map_err(|error| format!("failed to start {PICOCOM_COMMAND}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{PICOCOM_COMMAND} exited with {status}"))
    }
}

fn usage() -> String {
    "usage: dali device console [--port <path>]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::requested_port;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn accepts_automatic_console_selection() {
        assert_eq!(requested_port(&arguments(&["device", "console"])), Ok(None));
    }

    #[test]
    fn accepts_an_explicit_console_port() {
        assert_eq!(
            requested_port(&arguments(&["device", "console", "--port", "/dev/ttyACM0"])),
            Ok(Some("/dev/ttyACM0".to_owned()))
        );
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(requested_port(&arguments(&["device", "console", "--wait"])).is_err());
    }
}
