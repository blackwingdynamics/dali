//! Strict parser for canonical developer delegations.

use super::{DecodeError, cursor::Cursor};
use crate::{
    BoundedText, DelegationMetadata, KeyId, MAX_DELEGATION_ABIS, MAX_DELEGATION_SCOPES,
    MAX_DELEGATION_TARGETS, MAX_DEVELOPER_ID_BYTES, MAX_NAMESPACE_BYTES, MAX_TARGET_PROFILE_BYTES,
    MetadataHeader, MetadataRole, PUBLIC_KEY_LENGTH, PublicKey, SCHEMA_ID, validate_delegation,
};

/// Parses one canonical developer-delegation signed body.
pub fn parse_delegation_signed(bytes: &[u8]) -> Result<DelegationMetadata, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    cursor.byte(b'{')?;
    cursor.field("allowed_abis")?;
    let (allowed_abis, abi_count) = parse_abis(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("allowed_namespaces")?;
    let (allowed_namespaces, namespace_count) =
        parse_texts::<MAX_NAMESPACE_BYTES, MAX_DELEGATION_SCOPES>(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("allowed_targets")?;
    let (allowed_targets, target_count) =
        parse_texts::<MAX_TARGET_PROFILE_BYTES, MAX_DELEGATION_TARGETS>(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("developer_id")?;
    let developer_id = bounded::<MAX_DEVELOPER_ID_BYTES>(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("expires")?;
    let expires = cursor.number()?;
    cursor.byte(b',')?;
    cursor.field("key_id")?;
    let key_id = KeyId(cursor.hex::<{ crate::KEY_ID_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("not_after")?;
    let not_after = cursor.number()?;
    cursor.byte(b',')?;
    cursor.field("not_before")?;
    let not_before = cursor.number()?;
    cursor.byte(b',')?;
    cursor.field("public_key")?;
    let public_key = PublicKey(cursor.hex::<PUBLIC_KEY_LENGTH>()?);
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Delegation.as_str() {
        return Err(DecodeError::InvalidValue);
    }
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
    let metadata = DelegationMetadata {
        header: MetadataHeader {
            role: MetadataRole::Delegation,
            version,
            expires,
        },
        developer_id,
        key_id,
        public_key,
        allowed_namespaces,
        namespace_count,
        allowed_targets,
        target_count,
        allowed_abis,
        abi_count,
        not_before,
        not_after,
    };
    validate_delegation(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

fn parse_abis(cursor: &mut Cursor<'_>) -> Result<([u16; MAX_DELEGATION_ABIS], u8), DecodeError> {
    let mut values = [0; MAX_DELEGATION_ABIS];
    let count = parse_array(cursor, &mut values, |cursor| {
        u16::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)
    })?;
    Ok((values, count))
}

fn parse_texts<const CAPACITY: usize, const MAX: usize>(
    cursor: &mut Cursor<'_>,
) -> Result<([BoundedText<CAPACITY>; MAX], u8), DecodeError> {
    let mut values = [BoundedText::default(); MAX];
    let count = parse_array(cursor, &mut values, bounded::<CAPACITY>)?;
    Ok((values, count))
}

fn parse_array<T, const MAX: usize>(
    cursor: &mut Cursor<'_>,
    values: &mut [T; MAX],
    mut parse: impl FnMut(&mut Cursor<'_>) -> Result<T, DecodeError>,
) -> Result<u8, DecodeError> {
    cursor.byte(b'[')?;
    let mut count = 0;
    if cursor.peek(b']') {
        cursor.byte(b']')?;
        return Ok(count);
    }
    loop {
        if usize::from(count) == MAX {
            return Err(DecodeError::TooManyRecords);
        }
        values[usize::from(count)] = parse(cursor)?;
        count += 1;
        if cursor.peek(b',') {
            cursor.byte(b',')?;
        } else {
            break;
        }
    }
    cursor.byte(b']')?;
    Ok(count)
}

fn bounded<const CAPACITY: usize>(
    cursor: &mut Cursor<'_>,
) -> Result<BoundedText<CAPACITY>, DecodeError> {
    BoundedText::new(cursor.string()?).map_err(|_| DecodeError::InvalidValue)
}
