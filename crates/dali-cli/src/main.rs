//! Dali OS package and device CLI.

use std::{env, fs, process};

const PACKAGE_COMMAND: &str = "package";
const INSPECT_COMMAND: &str = "inspect";
const INPUT_FLAG: &str = "--input";
const OUTPUT_FLAG: &str = "--output";
const ENTRY_OFFSET_FLAG: &str = "--entry-offset";

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("dali: {error}");
        process::exit(1);
    }
}

fn run(arguments: Vec<String>) -> Result<(), String> {
    let Some(command) = arguments.first() else {
        return Err(usage());
    };
    match command.as_str() {
        PACKAGE_COMMAND => package(&arguments),
        INSPECT_COMMAND => {
            let input = required_flag(&arguments, INPUT_FLAG)?;
            println!("{}", inspect_package(&input)?);
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn package(arguments: &[String]) -> Result<(), String> {
    let input = required_flag(arguments, INPUT_FLAG)?;
    let output = required_flag(arguments, OUTPUT_FLAG)?;
    let entry_offset = required_flag(arguments, ENTRY_OFFSET_FLAG)?
        .parse::<u32>()
        .map_err(|_| format!("invalid {ENTRY_OFFSET_FLAG} value"))?;
    let payload = fs::read(input).map_err(|error| format!("cannot read input: {error}"))?;
    let output_size = dali_amrn::HEADER_SIZE
        .checked_add(payload.len())
        .ok_or_else(|| "package size overflow".to_owned())?;
    let mut package = vec![0; output_size];
    let written = dali_amrn::encode_package(&payload, entry_offset, &mut package)
        .map_err(|error| format!("cannot encode package: {error:?}"))?;
    fs::write(output, &package[..written]).map_err(|error| format!("cannot write output: {error}"))
}

fn inspect_package(input: &str) -> Result<String, String> {
    let package = fs::read(input).map_err(|error| format!("cannot read input: {error}"))?;
    inspect_bytes(&package)
}

fn inspect_bytes(package: &[u8]) -> Result<String, String> {
    let header = dali_amrn::parse_header(package)
        .map_err(|error| format!("invalid AMRN package: {error:?}"))?;
    let expected_size = dali_amrn::HEADER_SIZE
        .checked_add(header.payload_size as usize)
        .ok_or_else(|| "package size overflow".to_owned())?;
    if package.len() != expected_size {
        return Err(format!(
            "invalid AMRN package: file length is {}, expected {expected_size}",
            package.len()
        ));
    }
    let parsed =
        dali_amrn::parse(package).map_err(|error| format!("invalid AMRN package: {error:?}"))?;
    Ok(format!(
        "AMRN package valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\npayload_size: {}\nload_address: 0x{:08X}\nexecution_offset: {}\nentry_address: 0x{:08X}\nabi_version: {}\ncrc32: 0x{:08X}",
        header.format_version,
        header.target_id,
        header.header_size,
        header.payload_size,
        header.load_address,
        header.execution_offset,
        parsed.entry_address,
        header.abi_version,
        header.crc32
    ))
}

fn required_flag(arguments: &[String], flag: &str) -> Result<String, String> {
    let position = arguments
        .iter()
        .position(|argument| argument == flag)
        .ok_or_else(usage)?;
    arguments.get(position + 1).cloned().ok_or_else(usage)
}

fn usage() -> String {
    format!(
        "usage:\n  dali package {INPUT_FLAG} <payload> {OUTPUT_FLAG} <package> {ENTRY_OFFSET_FLAG} <bytes>\n  dali inspect {INPUT_FLAG} <package>"
    )
}

#[cfg(test)]
mod tests {
    use super::inspect_bytes;

    #[test]
    fn inspect_reports_contract_fields_for_valid_package() {
        let payload = [0x00, 0xBF, 0x00, 0xBF];
        let mut package = vec![0; dali_amrn::HEADER_SIZE + payload.len()];
        let result = dali_amrn::encode_package(&payload, 0, &mut package);
        assert!(result.is_ok(), "test payload must produce a valid package");
        let written = result.unwrap_or_default();

        let result = inspect_bytes(&package[..written]);
        assert!(result.is_ok(), "package should inspect");
        let report = result.unwrap_or_default();

        assert!(report.contains("AMRN package valid"));
        assert!(report.contains("target_id: 0x02"));
        assert!(report.contains("abi_version: 2"));
        assert!(report.contains("payload_size: 4"));
    }

    #[test]
    fn inspect_rejects_trailing_bytes() {
        let payload = [0x00, 0xBF, 0x00, 0xBF];
        let mut package = vec![0; dali_amrn::HEADER_SIZE + payload.len()];
        let result = dali_amrn::encode_package(&payload, 0, &mut package);
        assert!(result.is_ok(), "test payload must produce a valid package");
        let written = result.unwrap_or_default();
        package.push(0);

        let result = inspect_bytes(&package[..written + 1]);
        assert!(result.is_err(), "trailing data must fail");
        let error = match result {
            Ok(_) => String::new(),
            Err(error) => error,
        };

        assert!(error.contains("file length"));
    }
}
