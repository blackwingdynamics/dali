use super::{HEADER_SIZE, MAGIC, MAX_PAYLOAD_SIZE, stream::checksum, validate_execution_offset};

const FORMAT_VERSION_OFFSET: usize = 4;
const TARGET_ID_OFFSET: usize = 5;
const HEADER_SIZE_OFFSET: usize = 6;
const PAYLOAD_SIZE_OFFSET: usize = 8;
const LOAD_ADDRESS_OFFSET: usize = 12;
const EXECUTION_OFFSET_OFFSET: usize = 16;
const CRC32_OFFSET: usize = 20;
const ABI_VERSION_OFFSET: usize = 24;
const FLAGS_OFFSET: usize = 25;
const RESERVED_U16_OFFSET: usize = 26;
const RESERVED_U32_OFFSET: usize = 28;

/// Errors returned while constructing an AMRN package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildError {
    /// The payload is empty.
    EmptyPayload,
    /// The payload exceeds the format limit.
    PayloadTooLarge,
    /// The output buffer cannot contain the complete package.
    OutputTooSmall,
    /// The requested entry offset is invalid for the payload.
    InvalidExecutionOffset,
}

/// Encodes an AMRN v1 header and payload into caller-provided storage.
pub fn encode_package(
    payload: &[u8],
    execution_offset: u32,
    output: &mut [u8],
) -> Result<usize, BuildError> {
    if payload.is_empty() {
        return Err(BuildError::EmptyPayload);
    }
    if payload.len() > MAX_PAYLOAD_SIZE {
        return Err(BuildError::PayloadTooLarge);
    }
    validate_execution_offset(execution_offset, payload.len())
        .map_err(|_| BuildError::InvalidExecutionOffset)?;
    let package_size = HEADER_SIZE
        .checked_add(payload.len())
        .ok_or(BuildError::OutputTooSmall)?;
    if output.len() < package_size {
        return Err(BuildError::OutputTooSmall);
    }

    write_header(output, payload, execution_offset);
    output[HEADER_SIZE..package_size].copy_from_slice(payload);
    Ok(package_size)
}

fn write_header(output: &mut [u8], payload: &[u8], execution_offset: u32) {
    output[..MAGIC.len()].copy_from_slice(&MAGIC);
    output[FORMAT_VERSION_OFFSET] = super::FORMAT_VERSION;
    output[TARGET_ID_OFFSET] = super::TARGET_ID;
    output[HEADER_SIZE_OFFSET..PAYLOAD_SIZE_OFFSET]
        .copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
    output[PAYLOAD_SIZE_OFFSET..LOAD_ADDRESS_OFFSET]
        .copy_from_slice(&(payload.len() as u32).to_le_bytes());
    output[LOAD_ADDRESS_OFFSET..EXECUTION_OFFSET_OFFSET]
        .copy_from_slice(&super::LOAD_ADDRESS.to_le_bytes());
    output[EXECUTION_OFFSET_OFFSET..CRC32_OFFSET].copy_from_slice(&execution_offset.to_le_bytes());
    output[CRC32_OFFSET..ABI_VERSION_OFFSET].copy_from_slice(&checksum(payload).to_le_bytes());
    output[ABI_VERSION_OFFSET] = super::ABI_VERSION;
    output[FLAGS_OFFSET] = super::RESERVED_FLAGS;
    output[RESERVED_U16_OFFSET..RESERVED_U32_OFFSET]
        .copy_from_slice(&super::RESERVED_U16.to_le_bytes());
    output[RESERVED_U32_OFFSET..HEADER_SIZE].copy_from_slice(&super::RESERVED_U32.to_le_bytes());
}
