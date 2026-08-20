//! Canonical offline bundle-manifest encoding.

use crate::{BundleMetadata, EncodeError, MetadataRole, SCHEMA_ID, validate_bundle_metadata};

use super::writer::Writer;

/// Encodes one canonical offline bundle manifest signed body.
pub fn encode_bundle_signed(
    output: &mut [u8],
    metadata: BundleMetadata,
) -> Result<usize, EncodeError> {
    validate_bundle_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = Writer::new(output, crate::MAX_BUNDLE_BYTES);
    writer.object_start()?;
    writer.field_name("files")?;
    writer.array_start()?;
    for (index, file) in metadata.files[..usize::from(metadata.file_count)]
        .iter()
        .enumerate()
    {
        if index != 0 {
            writer.comma()?;
        }
        writer.object_start()?;
        writer.field_name("id")?;
        writer.text(file.id)?;
        writer.comma()?;
        writer.field_name("kind")?;
        writer.string(file.kind.as_str())?;
        writer.comma()?;
        writer.field_name("length")?;
        writer.number(u64::from(file.length))?;
        writer.comma()?;
        writer.field_name("sha256")?;
        writer.hex(&file.sha256)?;
        writer.object_end()?;
    }
    writer.array_end()?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(MetadataRole::Bundle.as_str())?;
    writer.comma()?;
    writer.field_name("schema")?;
    writer.string(SCHEMA_ID)?;
    writer.comma()?;
    writer.field_name("target_profile")?;
    writer.text(metadata.target_profile)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(metadata.header.version)?;
    writer.object_end()?;
    Ok(writer.len())
}
