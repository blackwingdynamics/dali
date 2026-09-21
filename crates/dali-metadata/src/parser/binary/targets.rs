//! Binary Metadata v2 targets role and cartridge-record codecs.

use super::{header, read_header, read_text, text};
use crate::codec::binary::{BodyReader, BodyWriter};
use crate::{
    BoundedText, DecodeError, EncodeError, KeyId, MetadataRole, Sha256Digest, TargetCartridge,
    TargetsMetadata, validate_targets_metadata,
};

/// Encodes the canonical Binary Metadata v2 targets body.
pub fn encode_binary_targets_body(
    metadata: TargetsMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_targets_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    writer.u16(metadata.delegation_count.into())?;
    for item in metadata
        .delegations
        .iter()
        .take(usize::from(metadata.delegation_count))
    {
        text(&mut writer, *item)?;
    }
    writer.u32(metadata.cartridge_count.into())?;
    for cartridge in metadata
        .cartridges
        .iter()
        .take(usize::from(metadata.cartridge_count))
    {
        let position = writer.position();
        writer.u16(0)?;
        let start = writer.position();
        encode_target_record(&mut writer, *cartridge)?;
        writer.patch_u16(
            position,
            u16::try_from(writer.position() - start).map_err(|_| EncodeError::InvalidValue)?,
        )?;
    }
    Ok(writer.position())
}

/// Parses the canonical Binary Metadata v2 targets body.
pub fn parse_binary_targets_body(bytes: &[u8]) -> Result<TargetsMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Targets)?;
    let delegation_count = reader.u16()?;
    if usize::from(delegation_count) > crate::MAX_DELEGATION_SCOPES {
        return Err(DecodeError::TooManyRecords);
    }
    let mut delegations = [BoundedText::default(); crate::MAX_DELEGATION_SCOPES];
    for item in delegations.iter_mut().take(usize::from(delegation_count)) {
        *item = read_text(&mut reader)?;
    }
    let cartridge_count = reader.u32()?;
    if cartridge_count > crate::MAX_TARGET_RECORDS as u32 {
        return Err(DecodeError::TooManyRecords);
    }
    let mut cartridges = [TargetCartridge::default(); crate::MAX_TARGET_RECORDS];
    for cartridge in cartridges.iter_mut().take(cartridge_count as usize) {
        let record_length = usize::from(reader.u16()?);
        let record = reader.array_slice(record_length)?;
        let mut record_reader = BodyReader::new(record);
        *cartridge = parse_target_record(&mut record_reader)?;
        if !record_reader.complete() {
            return Err(DecodeError::TrailingBytes);
        }
    }
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let metadata = TargetsMetadata {
        header,
        delegations,
        delegation_count: u8::try_from(delegation_count)
            .map_err(|_| DecodeError::TooManyRecords)?,
        cartridges,
        cartridge_count: u16::try_from(cartridge_count).map_err(|_| DecodeError::TooManyRecords)?,
    };
    validate_targets_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

/// Encodes one length-delimited target cartridge record.
pub fn encode_binary_target_record(
    value: TargetCartridge,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    let mut writer = BodyWriter::new(output);
    encode_target_record(&mut writer, value)?;
    Ok(writer.position())
}

/// Parses one complete length-delimited target cartridge record.
pub fn parse_binary_target_record(bytes: &[u8]) -> Result<TargetCartridge, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let target = parse_target_record(&mut reader)?;
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    Ok(target)
}

fn encode_target_record(
    writer: &mut BodyWriter<'_>,
    value: TargetCartridge,
) -> Result<(), EncodeError> {
    writer.bytes(&value.cartridge_id.0)?;
    text(writer, value.namespace)?;
    text(writer, value.developer_id)?;
    text(writer, value.delegation_id)?;
    writer.bytes(&value.developer_key_id.0)?;
    text(writer, value.target_profile)?;
    writer.u16(value.amrn_format)?;
    writer.u16(value.abi_version)?;
    text(writer, value.cartridge_version)?;
    text(writer, value.minimum_kernel_version)?;
    writer.u32(value.length)?;
    writer.bytes(&value.sha256.0)?;
    writer.u32(value.required_services)?;
    writer.u8(value.slot_id)
}

fn parse_target_record(reader: &mut BodyReader<'_>) -> Result<TargetCartridge, DecodeError> {
    Ok(TargetCartridge {
        cartridge_id: crate::CartridgeId(reader.array()?),
        namespace: read_text(reader)?,
        developer_id: read_text(reader)?,
        delegation_id: read_text(reader)?,
        developer_key_id: KeyId(reader.array()?),
        target_profile: read_text(reader)?,
        amrn_format: reader.u16()?,
        abi_version: reader.u16()?,
        cartridge_version: read_text(reader)?,
        minimum_kernel_version: read_text(reader)?,
        length: reader.u32()?,
        sha256: Sha256Digest(reader.array()?),
        required_services: reader.u32()?,
        slot_id: reader.u8()?,
    })
}
