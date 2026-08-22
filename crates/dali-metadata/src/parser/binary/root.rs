//! Binary Metadata v2 root role body codec.

use super::{header, read_header};
use crate::codec::binary::{BodyReader, BodyWriter};
use crate::{
    DecodeError, EncodeError, KeyId, MetadataRole, RoleDefinition, RoleKey,
    validate_role_references,
};

/// Encodes the canonical Binary Metadata v2 root body.
pub fn encode_binary_root_body(
    metadata: crate::RootMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    if metadata.header.role != MetadataRole::Root
        || metadata.key_count == 0
        || usize::from(metadata.key_count) > crate::MAX_ROOT_KEYS
        || metadata.role_count == 0
        || usize::from(metadata.role_count) > crate::MAX_ROOT_ROLES
    {
        return Err(EncodeError::InvalidValue);
    }
    validate_role_references(
        &metadata.keys[..usize::from(metadata.key_count)],
        &metadata.roles[..usize::from(metadata.role_count)],
    )
    .map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    writer.u8(metadata.key_count)?;
    for key in metadata.keys.iter().take(usize::from(metadata.key_count)) {
        writer.bytes(&key.key_id.0)?;
        writer.u8(crate::codec::binary::role_number(key.role))?;
        writer.bytes(&key.public_key.0)?;
    }
    writer.u8(metadata.role_count)?;
    for role in metadata.roles.iter().take(usize::from(metadata.role_count)) {
        writer.u8(crate::codec::binary::role_number(role.role))?;
        writer.u8(role.threshold)?;
        writer.u8(role.key_count)?;
        for key in role.keys.iter().take(usize::from(role.key_count)) {
            writer.bytes(&key.0)?;
        }
    }
    Ok(writer.position())
}

/// Parses the canonical Binary Metadata v2 root body.
pub fn parse_binary_root_body(bytes: &[u8]) -> Result<crate::RootMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Root)?;
    let key_count = reader.u8()?;
    if key_count == 0 || usize::from(key_count) > crate::MAX_ROOT_KEYS {
        return Err(DecodeError::TooManyRecords);
    }
    let mut keys = [RoleKey {
        role: MetadataRole::Root,
        key_id: KeyId([0; crate::KEY_ID_LENGTH]),
        public_key: crate::PublicKey([0; crate::PUBLIC_KEY_LENGTH]),
    }; crate::MAX_ROOT_KEYS];
    for key in keys.iter_mut().take(usize::from(key_count)) {
        key.key_id = KeyId(reader.array()?);
        key.role = crate::codec::binary::role_from_number(reader.u8()?)
            .ok_or(DecodeError::InvalidValue)?;
        key.public_key = crate::PublicKey(reader.array()?);
    }
    let role_count = reader.u8()?;
    if role_count == 0 || usize::from(role_count) > crate::MAX_ROOT_ROLES {
        return Err(DecodeError::TooManyRecords);
    }
    let empty_role = RoleDefinition {
        role: MetadataRole::Root,
        keys: [KeyId([0; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS],
        key_count: 0,
        threshold: 0,
    };
    let mut roles = [empty_role; crate::MAX_ROOT_ROLES];
    for role in roles.iter_mut().take(usize::from(role_count)) {
        role.role = crate::codec::binary::role_from_number(reader.u8()?)
            .ok_or(DecodeError::InvalidValue)?;
        role.threshold = reader.u8()?;
        role.key_count = reader.u8()?;
        if role.key_count == 0 || usize::from(role.key_count) > crate::MAX_ROLE_KEYS {
            return Err(DecodeError::InvalidValue);
        }
        for key in role.keys.iter_mut().take(usize::from(role.key_count)) {
            *key = KeyId(reader.array()?);
        }
    }
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    validate_role_references(
        &keys[..usize::from(key_count)],
        &roles[..usize::from(role_count)],
    )
    .map_err(|_| DecodeError::InvalidValue)?;
    Ok(crate::RootMetadata {
        header,
        keys,
        key_count,
        roles,
        role_count,
    })
}
