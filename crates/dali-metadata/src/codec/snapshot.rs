use crate::{
    DelegationReference, EncodeError, MAX_SNAPSHOT_BYTES, MetadataRole, SCHEMA_ID,
    SnapshotMetadata, validate_snapshot_metadata,
};

use super::writer::Writer;

/// Encodes the canonical signed body of snapshot metadata.
pub fn encode_snapshot_signed(
    output: &mut [u8],
    metadata: SnapshotMetadata,
) -> Result<usize, EncodeError> {
    validate_snapshot_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let active = &metadata.delegations[..usize::from(metadata.delegation_count)];
    if !active
        .windows(2)
        .all(|pair| pair[0].id.as_str() < pair[1].id.as_str())
    {
        return Err(EncodeError::NonCanonicalOrder);
    }
    let mut writer = Writer::new(output, MAX_SNAPSHOT_BYTES);
    writer.object_start()?;
    writer.field_name("expires")?;
    writer.number(metadata.header.expires)?;
    writer.comma()?;
    writer.field_name("metadata")?;
    writer.array_start()?;
    encode_targets_reference(&mut writer, metadata)?;
    for reference in active {
        writer.comma()?;
        encode_delegation_reference(&mut writer, reference)?;
    }
    writer.array_end()?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(MetadataRole::Snapshot.as_str())?;
    writer.comma()?;
    writer.field_name("schema")?;
    writer.string(SCHEMA_ID)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(metadata.header.version)?;
    writer.object_end()?;
    Ok(writer.len())
}

fn encode_targets_reference(
    writer: &mut Writer<'_>,
    metadata: SnapshotMetadata,
) -> Result<(), EncodeError> {
    writer.object_start()?;
    writer.field_name("length")?;
    writer.number(u64::from(metadata.targets.length))?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(MetadataRole::Targets.as_str())?;
    writer.comma()?;
    writer.field_name("sha256")?;
    writer.hex(&metadata.targets.sha256)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(metadata.targets.version)?;
    writer.object_end()
}

fn encode_delegation_reference(
    writer: &mut Writer<'_>,
    reference: &DelegationReference,
) -> Result<(), EncodeError> {
    writer.object_start()?;
    writer.field_name("id")?;
    writer.text(reference.id)?;
    writer.comma()?;
    writer.field_name("length")?;
    writer.number(u64::from(reference.length))?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(MetadataRole::Delegation.as_str())?;
    writer.comma()?;
    writer.field_name("sha256")?;
    writer.hex(&reference.sha256)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(reference.version)?;
    writer.object_end()
}
