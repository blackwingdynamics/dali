use std::{
    fs::OpenOptions,
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use dali_device::{Capability, Transport};

pub(super) const CONSOLE_COMMAND: &str = "console";

const PORT_FLAG: &str = "--port";
const CONSOLE_BAUD: u32 = 115_200;
const CONSOLE_LINE_CAPACITY: usize = 256;
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
    let terminal = RawTerminal::enter()?;
    let serial = serialport::new(port, CONSOLE_BAUD)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|error| format!("failed to open USB CDC console {port}: {error}"))?;
    let input_serial = serial
        .try_clone()
        .map_err(|error| format!("failed to clone USB CDC console {port}: {error}"))?;
    let stop = Arc::new(AtomicBool::new(false));
    let reader_stop = Arc::clone(&stop);
    let reader = thread::spawn(move || read_console(serial, reader_stop));
    let result = forward_input(input_serial, &stop);
    stop.store(true, Ordering::Relaxed);
    let _ = reader.join();
    drop(terminal);
    result
}

fn read_console(mut serial: Box<dyn serialport::SerialPort>, stop: Arc<AtomicBool>) {
    let mut bytes = [0; 64];
    let mut line = [0; CONSOLE_LINE_CAPACITY];
    let mut length = 0;
    let mut discard = false;
    while !stop.load(Ordering::Relaxed) {
        match serial.read(&mut bytes) {
            Ok(count) => {
                for byte in &bytes[..count] {
                    if *byte == b'\n' {
                        if !discard && line.first() == Some(&b'[') {
                            let _ = io::stdout().write_all(&line[..length]);
                            let _ = io::stdout().write_all(b"\n");
                            let _ = io::stdout().flush();
                        }
                        length = 0;
                        discard = false;
                    } else if !discard {
                        if length == line.len() {
                            discard = true;
                        } else {
                            line[length] = *byte;
                            length += 1;
                        }
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
}

fn forward_input(
    mut serial: Box<dyn serialport::SerialPort>,
    stop: &Arc<AtomicBool>,
) -> Result<(), String> {
    let mut input = io::stdin();
    let mut byte = [0; 1];
    while !stop.load(Ordering::Relaxed) {
        match input.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => serial
                .write_all(&byte)
                .map_err(|error| format!("failed to write USB CDC console: {error}"))?,
            Err(error) => return Err(format!("failed to read terminal input: {error}")),
        }
    }
    Ok(())
}

struct RawTerminal {
    state: String,
}

impl RawTerminal {
    fn enter() -> Result<Self, String> {
        let tty = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .map_err(|error| format!("failed to open controlling terminal: {error}"))?;
        let output = Command::new("stty")
            .arg("-g")
            .stdin(Stdio::from(tty.try_clone().map_err(|error| {
                format!("failed to access terminal settings: {error}")
            })?))
            .stdout(Stdio::piped())
            .output()
            .map_err(|error| format!("failed to read terminal settings: {error}"))?;
        if !output.status.success() {
            return Err("failed to read terminal settings".to_owned());
        }
        let state = String::from_utf8(output.stdout)
            .map_err(|_| "terminal settings were not valid UTF-8".to_owned())?
            .trim()
            .to_owned();
        let status = Command::new("stty")
            .args(["-icanon", "-echo", "min", "1", "time", "0"])
            .stdin(Stdio::from(tty))
            .status()
            .map_err(|error| format!("failed to configure terminal: {error}"))?;
        if !status.success() {
            return Err("failed to configure terminal".to_owned());
        }
        Ok(Self { state })
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        if let Ok(tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") {
            let _ = Command::new("stty")
                .arg(&self.state)
                .stdin(Stdio::from(tty))
                .status();
        }
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
