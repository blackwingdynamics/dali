//! Binary Metadata v2 revocation role body codec.

use super::{header, read_header, read_text, text};
use crate::codec::binary::{BodyReader, BodyWriter};
use crate::{
    DecodeError, EncodeError, KeyId, MetadataRole, RevocationMetadata, RevocationRecord,
    validate_revocation_metadata,
};

/// Encodes the canonical Binary Metadata v2 revocation body.
pub fn encode_binary_revocation_body(
    metadata: RevocationMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_revocation_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    writer.u16(u16::from(metadata.record_count))?;
    for record in metadata
        .records
        .iter()
        .take(usize::from(metadata.record_count))
    {
        text(&mut writer, record.developer_id)?;
        writer.u64(record.effective_version)?;
        writer.bytes(&record.issuer_key_id.0)?;
        writer.bytes(&record.key_id.0)?;
        text(&mut writer, record.reason)?;
    }
    Ok(writer.position())
}

/// Parses the canonical Binary Metadata v2 revocation body.
pub fn parse_binary_revocation_body(bytes: &[u8]) -> Result<RevocationMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Revocation)?;
    let record_count = reader.u16()?;
    if usize::from(record_count) > crate::MAX_REVOCATIONS {
        return Err(DecodeError::TooManyRecords);
    }
    let mut records = [RevocationRecord::default(); crate::MAX_REVOCATIONS];
    for record in records.iter_mut().take(usize::from(record_count)) {
        record.developer_id = read_text(&mut reader)?;
        record.effective_version = reader.u64()?;
        record.issuer_key_id = KeyId(reader.array()?);
        record.key_id = KeyId(reader.array()?);
        record.reason = read_text(&mut reader)?;
    }
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let metadata = RevocationMetadata {
        header,
        records,
        record_count: u8::try_from(record_count).map_err(|_| DecodeError::TooManyRecords)?,
    };
    validate_revocation_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}
