use std::process::Command;

use dali_device::{Capability, DeviceRecord, State, Transport, normalize};

const LIST_COMMAND: &str = "list";
const PROBE_RS_COMMAND: &str = "probe-rs";
const PROBE_RS_LIST_ARGUMENTS: &[&str] = &["list"];
const DFU_UTIL_COMMAND: &str = "dfu-util";
const DFU_UTIL_LIST_ARGUMENTS: &[&str] = &["-l"];

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    if arguments.get(1).map(String::as_str) != Some(LIST_COMMAND) || arguments.len() != 2 {
        return Err(usage());
    }
    let (mut records, failures) = discover_all();
    records.extend(discover_cdc());
    for record in normalize(records) {
        println!("{}", render_record(&record));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn discover_all() -> (Vec<DeviceRecord>, Vec<String>) {
    let mut records = Vec::new();
    let mut failures = Vec::new();
    match discover_command(Transport::Probe, PROBE_RS_COMMAND, PROBE_RS_LIST_ARGUMENTS) {
        Ok(found) => records.extend(found),
        Err(error) => failures.push(error),
    }
    match discover_command(Transport::Dfu, DFU_UTIL_COMMAND, DFU_UTIL_LIST_ARGUMENTS) {
        Ok(found) => records.extend(found),
        Err(error) => failures.push(error),
    }
    (records, failures)
}

fn discover_command(
    transport: Transport,
    command: &str,
    arguments: &[&str],
) -> Result<Vec<DeviceRecord>, String> {
    let output = Command::new(command)
        .args(arguments)
        .output()
        .map_err(|error| {
            format!(
                "{transport_name} discovery failed: {error}",
                transport_name = transport.as_str()
            )
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() && !is_empty_inventory(transport, &stdout, &stderr) {
        return Err(format!(
            "{transport_name} discovery failed: {detail}",
            transport_name = transport.as_str(),
            detail = first_line(&stderr).unwrap_or_else(|| "command failed".to_owned())
        ));
    }
    Ok(match transport {
        Transport::Probe => parse_probe_inventory(&stdout),
        Transport::Dfu => parse_dfu_inventory(&stdout),
        Transport::Cdc => Vec::new(),
    })
}

fn is_empty_inventory(transport: Transport, stdout: &str, stderr: &str) -> bool {
    let text = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    match transport {
        Transport::Probe => text.contains("no debug probes") || text.contains("no probes"),
        Transport::Dfu => text.contains("no dfu capable") || text.contains("no devices"),
        Transport::Cdc => false,
    }
}

fn parse_probe_inventory(output: &str) -> Vec<DeviceRecord> {
    output
        .lines()
        .filter_map(|line| {
            let (product, id) = line.split_once("--")?;
            let id = id.split_whitespace().next()?.to_owned();
            if id.is_empty() {
                return None;
            }
            Some(DeviceRecord {
                id,
                transport: Transport::Probe,
                target: None,
                vendor: None,
                product: Some(product.split_once(']')?.1.trim().to_owned()),
                serial: None,
                path: None,
                state: State::Available,
                capabilities: vec![Capability::Attach, Capability::Flash],
                diagnostic: None,
            })
        })
        .collect()
}

fn parse_dfu_inventory(output: &str) -> Vec<DeviceRecord> {
    output
        .lines()
        .filter_map(|line| {
            let start = line.find('[')?;
            let end = line[start..].find(']')? + start;
            let id = line[start + 1..end].trim();
            if id.is_empty() {
                return None;
            }
            Some(DeviceRecord {
                id: id.to_owned(),
                transport: Transport::Dfu,
                target: None,
                vendor: None,
                product: Some(line.trim().to_owned()),
                serial: None,
                path: None,
                state: State::Available,
                capabilities: vec![Capability::Flash],
                diagnostic: None,
            })
        })
        .collect()
}

fn discover_cdc() -> Vec<DeviceRecord> {
    Vec::new()
}

fn render_record(record: &DeviceRecord) -> String {
    let product = record.product.as_deref().unwrap_or("not declared");
    let target = record.target.as_deref().unwrap_or("not identified");
    let capabilities = record
        .capabilities
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "transport={} id={} state={} target={} product={} capabilities={capabilities}",
        record.transport.as_str(),
        record.id,
        record.state.as_str(),
        target,
        product,
    )
}

fn first_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

fn usage() -> String {
    "usage: dali device list".to_owned()
}

#[cfg(test)]
mod tests {
    use super::{parse_dfu_inventory, parse_probe_inventory, render_record};

    #[test]
    fn parses_probe_inventory_records() {
        let records =
            parse_probe_inventory("[0]: Debugprobe on Pico -- 2e8a:000c-0:SERIAL (CMSIS-DAP)");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "2e8a:000c-0:SERIAL");
        assert!(render_record(&records[0]).contains("transport=probe"));
    }

    #[test]
    fn parses_dfu_inventory_records() {
        let records = parse_dfu_inventory("Found DFU: [0483:df11] ver=011a, devnum=1");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "0483:df11");
        assert!(render_record(&records[0]).contains("capabilities=flash"));
    }
}
