//! Root metadata document parsing.

use super::{DecodeError, cursor::Cursor};
use crate::{MetadataHeader, MetadataRole, RootMetadata, SCHEMA_ID, validate_role_references};

/// Parses one canonical root signed body into fixed-capacity storage.
pub fn parse_root_signed(bytes: &[u8]) -> Result<RootMetadata, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    cursor.byte(b'{')?;
    cursor.field("expires")?;
    let expires = cursor.number()?;
    cursor.byte(b',')?;
    cursor.field("keys")?;
    let (keys, key_count) = cursor.keys()?;
    if key_count == 0
        || !keys[..usize::from(key_count)]
            .iter()
            .any(|key| key.role == MetadataRole::Root)
    {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Root.as_str() {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("roles")?;
    let (roles, role_count) = cursor.roles()?;
    if role_count == 0 {
        return Err(DecodeError::InvalidValue);
    }
    validate_role_references(
        &keys[..usize::from(key_count)],
        &roles[..usize::from(role_count)],
    )
    .map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("schema")?;
    if cursor.string()? != SCHEMA_ID {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("version")?;
    let version = cursor.number()?;
    cursor.byte(b'}')?;
    if !cursor.is_complete() {
        return Err(DecodeError::TrailingBytes);
    }
    if version == 0 {
        return Err(DecodeError::InvalidValue);
    }
    Ok(RootMetadata {
        header: MetadataHeader {
            role: MetadataRole::Root,
            version,
            expires,
        },
        keys,
        key_count,
        roles,
        role_count,
    })
}

pub(super) fn parse_role(value: &str) -> Result<MetadataRole, DecodeError> {
    match value {
        "root" => Ok(MetadataRole::Root),
        "timestamp" => Ok(MetadataRole::Timestamp),
        "snapshot" => Ok(MetadataRole::Snapshot),
        "targets" => Ok(MetadataRole::Targets),
        "delegation" => Ok(MetadataRole::Delegation),
        "bundle" => Ok(MetadataRole::Bundle),
        _ => Err(DecodeError::InvalidValue),
    }
}

pub(super) fn hex_value(value: u8) -> Result<u8, DecodeError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(DecodeError::InvalidHex),
    }
}

pub(super) fn validate_key_order(keys: &[crate::KeyId]) -> Result<(), DecodeError> {
    if keys.windows(2).all(|pair| pair[0].0 < pair[1].0) {
        Ok(())
    } else {
        Err(DecodeError::InvalidFieldOrder)
    }
}

pub(super) fn validate_role_key_order(keys: &[crate::RoleKey]) -> Result<(), DecodeError> {
    if keys
        .windows(2)
        .all(|pair| pair[0].key_id.0 < pair[1].key_id.0)
    {
        Ok(())
    } else {
        Err(DecodeError::InvalidFieldOrder)
    }
}

pub(super) fn validate_role_order(roles: &[crate::RoleDefinition]) -> Result<(), DecodeError> {
    if roles
        .windows(2)
        .all(|pair| pair[0].role.as_str() < pair[1].role.as_str())
    {
        Ok(())
    } else {
        Err(DecodeError::InvalidFieldOrder)
    }
}
