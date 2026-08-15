use std::{collections::BTreeMap, env, fs, path::Path, process::Command};

use dali_device::{Capability, DeviceRecord, State, Transport};

const LINUX_OS: &str = "linux";
const LINUX_DEVICE_DIRECTORY: &str = "/dev";
const CDC_DEVICE_PREFIX: &str = "ttyACM";
const UDEVADM_COMMAND: &str = "udevadm";
const UDEV_INFO_ARGUMENTS: &[&str] = &["info", "--query=property", "--name"];
const UDEV_SERIAL_PROPERTY: &str = "ID_SERIAL_SHORT";
const UDEV_PATH_PROPERTY: &str = "ID_PATH";
const UDEV_VENDOR_PROPERTY: &str = "ID_VENDOR_ID";
const CDC_PRODUCT_LABEL: &str = "USB CDC console";

pub(super) fn discover() -> Result<Vec<DeviceRecord>, String> {
    if env::consts::OS != LINUX_OS {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(LINUX_DEVICE_DIRECTORY)
        .map_err(|error| format!("cdc discovery failed: {error}"))?;
    let mut records = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("cdc discovery failed: {error}"))?
            .path();
        if is_cdc_device(&path) {
            records.push(read_record(&path)?);
        }
    }
    Ok(records)
}

fn is_cdc_device(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(CDC_DEVICE_PREFIX))
}

fn read_record(path: &Path) -> Result<DeviceRecord, String> {
    let properties = read_properties(path)?;
    let serial = properties.get(UDEV_SERIAL_PROPERTY).cloned();
    let topology = properties.get(UDEV_PATH_PROPERTY).cloned();
    let id = serial
        .as_ref()
        .map(|value| format!("serial:{value}"))
        .or_else(|| topology.as_ref().map(|value| format!("path:{value}")))
        .ok_or_else(|| format!("cdc device has no stable identity: {}", path.display()))?;
    let state = if serial.is_some() {
        State::Available
    } else {
        State::Unidentified
    };
    let diagnostic = if serial.is_some() {
        None
    } else {
        Some("connection-scoped identity; serial not declared".to_owned())
    };
    Ok(DeviceRecord {
        id,
        transport: Transport::Cdc,
        target: None,
        vendor: properties.get(UDEV_VENDOR_PROPERTY).cloned(),
        product: Some(CDC_PRODUCT_LABEL.to_owned()),
        serial,
        path: Some(path.display().to_string()),
        state,
        capabilities: vec![Capability::Console],
        diagnostic,
    })
}

fn read_properties(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let path_value = path
        .to_str()
        .ok_or_else(|| format!("invalid CDC device path: {}", path.display()))?;
    let output = Command::new(UDEVADM_COMMAND)
        .args(UDEV_INFO_ARGUMENTS)
        .arg(path_value)
        .output()
        .map_err(|error| format!("cdc identity lookup failed: {error}"))?;
    if !output.status.success() {
        return Err(format!("cdc identity lookup failed for {}", path.display()));
    }
    Ok(parse_properties(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_properties(output: &str) -> BTreeMap<String, String> {
    output
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_properties;

    #[test]
    fn parses_udev_properties() {
        let properties =
            parse_properties("ID_SERIAL_SHORT=ABC\nID_PATH=usb-1-2\nID_VENDOR_ID=0483\n");
        assert_eq!(
            properties.get("ID_SERIAL_SHORT").map(String::as_str),
            Some("ABC")
        );
        assert_eq!(
            properties.get("ID_PATH").map(String::as_str),
            Some("usb-1-2")
        );
    }
}
