//! Canonical explicit revocation metadata encoding.

use crate::{
    EncodeError, MetadataRole, RevocationMetadata, SCHEMA_ID, validate_revocation_metadata,
};

use super::writer::Writer;

/// Encodes one canonical revocation metadata signed body.
pub fn encode_revocation_signed(
    output: &mut [u8],
    metadata: RevocationMetadata,
) -> Result<usize, EncodeError> {
    validate_revocation_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = Writer::new(output, crate::MAX_REVOCATION_BYTES);
    writer.object_start()?;
    writer.field_name("expires")?;
    writer.number(metadata.header.expires)?;
    writer.comma()?;
    writer.field_name("revocations")?;
    writer.array_start()?;
    for (index, record) in metadata.records[..usize::from(metadata.record_count)]
        .iter()
        .enumerate()
    {
        if index != 0 {
            writer.comma()?;
        }
        writer.object_start()?;
        writer.field_name("developer_id")?;
        writer.text(record.developer_id)?;
        writer.comma()?;
        writer.field_name("effective_version")?;
        writer.number(record.effective_version)?;
        writer.comma()?;
        writer.field_name("issuer_key_id")?;
        writer.hex(&record.issuer_key_id)?;
        writer.comma()?;
        writer.field_name("key_id")?;
        writer.hex(&record.key_id)?;
        writer.comma()?;
        writer.field_name("reason")?;
        writer.text(record.reason)?;
        writer.object_end()?;
    }
    writer.array_end()?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(MetadataRole::Revocation.as_str())?;
    writer.comma()?;
    writer.field_name("schema")?;
    writer.string(SCHEMA_ID)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(metadata.header.version)?;
    writer.object_end()?;
    Ok(writer.len())
}
