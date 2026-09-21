use dali_device::{DeviceRecord, normalize};

pub(super) const INFO_COMMAND: &str = "info";
const UNDECLARED_VALUE: &str = "not declared";
const TARGET_METADATA_HEADER: &str = "target metadata:";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let selector = arguments.get(2).ok_or_else(usage)?;
    if arguments.len() != 3 {
        return Err(usage());
    }
    let (records, failures) = crate::commands::device::discover_records();
    let records = normalize(records);
    let record = select_record(&records, selector)?;
    println!("{}", crate::commands::device::render_record(record));
    if record.transport == dali_device::Transport::Bulk {
        super::bulk::verify(record)?;
        println!();
        println!("installer interface: verified");
    }
    println!();
    println!("{TARGET_METADATA_HEADER}");
    match record.target.as_deref() {
        Some(target) => {
            let profile = dali_targets::find_board(target)
                .ok_or_else(|| format!("target profile '{target}' is not declared"))?;
            println!("{}", crate::commands::target::render_profile(profile));
        }
        None => println!("target: {UNDECLARED_VALUE}"),
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn select_record<'a>(
    records: &'a [DeviceRecord],
    selector: &str,
) -> Result<&'a DeviceRecord, String> {
    let matches = records
        .iter()
        .filter(|record| record.id == selector || record.path.as_deref() == Some(selector))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [record] => Ok(record),
        [] => Err(format!("device '{selector}' was not found")),
        _ => Err(format!("device selector '{selector}' is ambiguous")),
    }
}

fn usage() -> String {
    "usage: dali device info <id-or-path>".to_owned()
}

#[cfg(test)]
mod tests {
    use super::select_record;
    use dali_device::{Capability, DeviceRecord, State, Transport};

    fn record(id: &str, path: Option<&str>) -> DeviceRecord {
        DeviceRecord {
            id: id.to_owned(),
            transport: Transport::Dfu,
            target: Some("f405".to_owned()),
            vendor: Some("0483".to_owned()),
            product: Some("DFU device".to_owned()),
            serial: None,
            path: path.map(str::to_owned),
            state: State::Available,
            capabilities: vec![Capability::Flash],
            diagnostic: None,
        }
    }

    #[test]
    fn selects_by_transport_identifier_or_path() {
        let records = [record("0483:df11", Some("1-10.1"))];
        assert_eq!(
            select_record(&records, "0483:df11").unwrap().id,
            "0483:df11"
        );
        assert_eq!(select_record(&records, "1-10.1").unwrap().id, "0483:df11");
    }

    #[test]
    fn rejects_missing_and_ambiguous_selectors() {
        let records = [
            record("same", Some("path-a")),
            record("same", Some("path-b")),
        ];
        assert!(select_record(&records, "missing").is_err());
        assert!(select_record(&records, "same").is_err());
    }
}
