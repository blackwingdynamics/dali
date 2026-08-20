use crate::{
    EncodeError, MAX_TIMESTAMP_BYTES, MetadataRole, SCHEMA_ID, TimestampMetadata,
    validate_timestamp_metadata,
};

use super::writer::Writer;

/// Encodes the canonical signed body of timestamp metadata.
pub fn encode_timestamp_signed(
    output: &mut [u8],
    metadata: TimestampMetadata,
) -> Result<usize, EncodeError> {
    validate_timestamp_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = Writer::new(output, MAX_TIMESTAMP_BYTES);
    writer.object_start()?;
    writer.field_name("expires")?;
    writer.number(metadata.header.expires)?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(MetadataRole::Timestamp.as_str())?;
    writer.comma()?;
    writer.field_name("schema")?;
    writer.string(SCHEMA_ID)?;
    writer.comma()?;
    writer.field_name("snapshot")?;
    writer.object_start()?;
    writer.field_name("length")?;
    writer.number(u64::from(metadata.snapshot_length))?;
    writer.comma()?;
    writer.field_name("sha256")?;
    writer.hex(&metadata.snapshot_sha256)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(metadata.snapshot_version)?;
    writer.object_end()?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(metadata.header.version)?;
    writer.object_end()?;
    Ok(writer.len())
}
