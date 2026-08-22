//! Binary Metadata v2 developer-delegation body codec.

use super::{header, read_header, read_text, text};
use crate::codec::binary::{BodyReader, BodyWriter};
use crate::{
    BoundedText, DecodeError, DelegationMetadata, EncodeError, KeyId, MetadataRole, PublicKey,
    validate_delegation,
};

/// Encodes the canonical Binary Metadata v2 delegation body.
pub fn encode_binary_delegation_body(
    metadata: DelegationMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_delegation(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    text(&mut writer, metadata.developer_id)?;
    writer.bytes(&metadata.key_id.0)?;
    writer.bytes(&metadata.public_key.0)?;
    writer.u8(metadata.namespace_count)?;
    for item in metadata
        .allowed_namespaces
        .iter()
        .take(usize::from(metadata.namespace_count))
    {
        text(&mut writer, *item)?;
    }
    writer.u8(metadata.target_count)?;
    for item in metadata
        .allowed_targets
        .iter()
        .take(usize::from(metadata.target_count))
    {
        text(&mut writer, *item)?;
    }
    writer.u8(metadata.abi_count)?;
    for item in metadata
        .allowed_abis
        .iter()
        .take(usize::from(metadata.abi_count))
    {
        writer.u16(*item)?;
    }
    writer.u64(metadata.not_before)?;
    writer.u64(metadata.not_after)?;
    Ok(writer.position())
}

/// Parses the canonical Binary Metadata v2 delegation body.
pub fn parse_binary_delegation_body(bytes: &[u8]) -> Result<DelegationMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Delegation)?;
    let developer_id = read_text(&mut reader)?;
    let key_id = KeyId(reader.array()?);
    let public_key = PublicKey(reader.array()?);
    let namespace_count = reader.u8()?;
    if usize::from(namespace_count) > crate::MAX_DELEGATION_SCOPES {
        return Err(DecodeError::TooManyRecords);
    }
    let mut allowed_namespaces = [BoundedText::default(); crate::MAX_DELEGATION_SCOPES];
    for item in allowed_namespaces
        .iter_mut()
        .take(usize::from(namespace_count))
    {
        *item = read_text(&mut reader)?;
    }
    let target_count = reader.u8()?;
    if usize::from(target_count) > crate::MAX_DELEGATION_TARGETS {
        return Err(DecodeError::TooManyRecords);
    }
    let mut allowed_targets = [BoundedText::default(); crate::MAX_DELEGATION_TARGETS];
    for item in allowed_targets.iter_mut().take(usize::from(target_count)) {
        *item = read_text(&mut reader)?;
    }
    let abi_count = reader.u8()?;
    if usize::from(abi_count) > crate::MAX_DELEGATION_ABIS {
        return Err(DecodeError::TooManyRecords);
    }
    let mut allowed_abis = [0; crate::MAX_DELEGATION_ABIS];
    for item in allowed_abis.iter_mut().take(usize::from(abi_count)) {
        *item = reader.u16()?;
    }
    let metadata = DelegationMetadata {
        header,
        developer_id,
        key_id,
        public_key,
        allowed_namespaces,
        namespace_count,
        allowed_targets,
        target_count,
        allowed_abis,
        abi_count,
        not_before: reader.u64()?,
        not_after: reader.u64()?,
    };
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    validate_delegation(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}
