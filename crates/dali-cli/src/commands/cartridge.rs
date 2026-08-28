use std::fs;

use super::{ENTRY_OFFSET_FLAG, INPUT_FLAG, OUTPUT_FLAG, required_flag};

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let input = required_flag(arguments, INPUT_FLAG)?;
    let output = required_flag(arguments, OUTPUT_FLAG)?;
    let entry_offset = required_flag(arguments, ENTRY_OFFSET_FLAG)?
        .parse::<u32>()
        .map_err(|_| format!("invalid {ENTRY_OFFSET_FLAG} value"))?;
    let payload = fs::read(input).map_err(|error| format!("cannot read input: {error}"))?;
    let cartridge = build_cartridge(&payload, entry_offset)?;
    fs::write(output, cartridge).map_err(|error| format!("cannot write output: {error}"))
}

pub(super) fn build_cartridge(payload: &[u8], entry_offset: u32) -> Result<Vec<u8>, String> {
    let output_size = dali_amrn::HEADER_SIZE
        .checked_add(payload.len())
        .ok_or_else(|| "cartridge size overflow".to_owned())?;
    let mut cartridge = vec![0; output_size];
    let written = dali_amrn::encode_cartridge(payload, entry_offset, &mut cartridge)
        .map_err(|error| format!("cannot encode cartridge: {error:?}"))?;
    cartridge.truncate(written);
    Ok(cartridge)
}
