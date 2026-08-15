use std::fs;

use super::{ENTRY_OFFSET_FLAG, INPUT_FLAG, OUTPUT_FLAG, required_flag};

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let input = required_flag(arguments, INPUT_FLAG)?;
    let output = required_flag(arguments, OUTPUT_FLAG)?;
    let entry_offset = required_flag(arguments, ENTRY_OFFSET_FLAG)?
        .parse::<u32>()
        .map_err(|_| format!("invalid {ENTRY_OFFSET_FLAG} value"))?;
    let payload = fs::read(input).map_err(|error| format!("cannot read input: {error}"))?;
    let package = build_package(&payload, entry_offset)?;
    fs::write(output, package).map_err(|error| format!("cannot write output: {error}"))
}

pub(super) fn build_package(payload: &[u8], entry_offset: u32) -> Result<Vec<u8>, String> {
    let output_size = dali_amrn::HEADER_SIZE
        .checked_add(payload.len())
        .ok_or_else(|| "package size overflow".to_owned())?;
    let mut package = vec![0; output_size];
    let written = dali_amrn::encode_package(payload, entry_offset, &mut package)
        .map_err(|error| format!("cannot encode package: {error:?}"))?;
    package.truncate(written);
    Ok(package)
}
