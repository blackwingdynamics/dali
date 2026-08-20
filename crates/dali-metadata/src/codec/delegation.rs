//! Canonical developer-delegation encoding.

use crate::{DelegationMetadata, EncodeError, MetadataRole, SCHEMA_ID, validate_delegation};

use super::writer::Writer;

/// Encodes one canonical developer-delegation signed body.
pub fn encode_delegation_signed(
    output: &mut [u8],
    metadata: DelegationMetadata,
) -> Result<usize, EncodeError> {
    validate_delegation(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = Writer::new(output, crate::MAX_DELEGATION_BYTES);
    writer.object_start()?;
    writer.field_name("allowed_abis")?;
    encode_abis(&mut writer, metadata)?;
    writer.comma()?;
    writer.field_name("allowed_namespaces")?;
    encode_namespaces(&mut writer, metadata)?;
    writer.comma()?;
    writer.field_name("allowed_targets")?;
    encode_targets(&mut writer, metadata)?;
    writer.comma()?;
    writer.field_name("developer_id")?;
    writer.text(metadata.developer_id)?;
    writer.comma()?;
    writer.field_name("expires")?;
    writer.number(metadata.header.expires)?;
    writer.comma()?;
    writer.field_name("key_id")?;
    writer.hex(&metadata.key_id)?;
    writer.comma()?;
    writer.field_name("not_after")?;
    writer.number(metadata.not_after)?;
    writer.comma()?;
    writer.field_name("not_before")?;
    writer.number(metadata.not_before)?;
    writer.comma()?;
    writer.field_name("public_key")?;
    writer.hex(&metadata.public_key)?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(MetadataRole::Delegation.as_str())?;
    writer.comma()?;
    writer.field_name("schema")?;
    writer.string(SCHEMA_ID)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(metadata.header.version)?;
    writer.object_end()?;
    Ok(writer.len())
}

fn encode_abis(writer: &mut Writer<'_>, metadata: DelegationMetadata) -> Result<(), EncodeError> {
    writer.array_start()?;
    for (index, abi) in metadata.allowed_abis[..usize::from(metadata.abi_count)]
        .iter()
        .enumerate()
    {
        if index != 0 {
            writer.comma()?;
        }
        writer.number(u64::from(*abi))?;
    }
    writer.array_end()
}

fn encode_namespaces(
    writer: &mut Writer<'_>,
    metadata: DelegationMetadata,
) -> Result<(), EncodeError> {
    encode_texts(
        writer,
        &metadata.allowed_namespaces[..usize::from(metadata.namespace_count)],
    )
}

fn encode_targets(
    writer: &mut Writer<'_>,
    metadata: DelegationMetadata,
) -> Result<(), EncodeError> {
    encode_texts(
        writer,
        &metadata.allowed_targets[..usize::from(metadata.target_count)],
    )
}

fn encode_texts<const CAPACITY: usize>(
    writer: &mut Writer<'_>,
    values: &[crate::BoundedText<CAPACITY>],
) -> Result<(), EncodeError> {
    writer.array_start()?;
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            writer.comma()?;
        }
        writer.text(*value)?;
    }
    writer.array_end()
}
