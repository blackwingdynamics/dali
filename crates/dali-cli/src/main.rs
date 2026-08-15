//! Dali OS package and device CLI.

use std::{env, fs, process};

const PACKAGE_COMMAND: &str = "package";
const INPUT_FLAG: &str = "--input";
const OUTPUT_FLAG: &str = "--output";
const ENTRY_OFFSET_FLAG: &str = "--entry-offset";

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("dali-cli: {error}");
        process::exit(1);
    }
}

fn run(arguments: Vec<String>) -> Result<(), String> {
    let Some(command) = arguments.first() else {
        return Err(usage());
    };
    if command != PACKAGE_COMMAND {
        return Err(usage());
    }
    let input = required_flag(&arguments, INPUT_FLAG)?;
    let output = required_flag(&arguments, OUTPUT_FLAG)?;
    let entry_offset = required_flag(&arguments, ENTRY_OFFSET_FLAG)?
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

fn required_flag(arguments: &[String], flag: &str) -> Result<String, String> {
    let position = arguments
        .iter()
        .position(|argument| argument == flag)
        .ok_or_else(usage)?;
    arguments.get(position + 1).cloned().ok_or_else(usage)
}

fn usage() -> String {
    format!(
        "usage: dali-cli package {INPUT_FLAG} <payload> {OUTPUT_FLAG} <package> {ENTRY_OFFSET_FLAG} <bytes>"
    )
}
