//! Strict parser for canonical revocation metadata.

use super::{DecodeError, cursor::Cursor};
use crate::{
    BoundedText, KeyId, MAX_DEVELOPER_ID_BYTES, MAX_REVOCATION_REASON_BYTES, MAX_REVOCATIONS,
    MetadataHeader, MetadataRole, RevocationMetadata, RevocationRecord, SCHEMA_ID,
    validate_revocation_metadata,
};

/// Parses one canonical revocation signed body into fixed-capacity storage.
pub fn parse_revocation_signed(bytes: &[u8]) -> Result<RevocationMetadata, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    cursor.byte(b'{')?;
    cursor.field("expires")?;
    let expires = cursor.number()?;
    cursor.byte(b',')?;
    cursor.field("revocations")?;
    let (records, record_count) = parse_records(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Revocation.as_str() {
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
    let metadata = RevocationMetadata {
        header: MetadataHeader {
            role: MetadataRole::Revocation,
            version,
            expires,
        },
        records,
        record_count,
    };
    validate_revocation_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

fn parse_records(
    cursor: &mut Cursor<'_>,
) -> Result<([RevocationRecord; MAX_REVOCATIONS], u8), DecodeError> {
    let mut records = [RevocationRecord::default(); MAX_REVOCATIONS];
    cursor.byte(b'[')?;
    let mut count = 0_usize;
    if cursor.peek(b']') {
        cursor.byte(b']')?;
        return Ok((records, 0));
    }
    loop {
        if count == MAX_REVOCATIONS {
            return Err(DecodeError::TooManyRecords);
        }
        records[count] = parse_record(cursor)?;
        count += 1;
        if cursor.peek(b',') {
            cursor.byte(b',')?;
        } else {
            break;
        }
    }
    cursor.byte(b']')?;
    Ok((
        records,
        u8::try_from(count).map_err(|_| DecodeError::TooManyRecords)?,
    ))
}

fn parse_record(cursor: &mut Cursor<'_>) -> Result<RevocationRecord, DecodeError> {
    cursor.byte(b'{')?;
    cursor.field("developer_id")?;
    let developer_id = BoundedText::<MAX_DEVELOPER_ID_BYTES>::new(cursor.string()?)
        .map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("effective_version")?;
    let effective_version = cursor.number()?;
    cursor.byte(b',')?;
    cursor.field("issuer_key_id")?;
    let issuer_key_id = KeyId(cursor.hex::<{ crate::KEY_ID_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("key_id")?;
    let key_id = KeyId(cursor.hex::<{ crate::KEY_ID_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("reason")?;
    let reason = BoundedText::<MAX_REVOCATION_REASON_BYTES>::new(cursor.string()?)
        .map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b'}')?;
    Ok(RevocationRecord {
        developer_id,
        effective_version,
        issuer_key_id,
        key_id,
        reason,
    })
}
