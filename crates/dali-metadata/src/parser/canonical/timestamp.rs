//! Strict parser for canonical timestamp metadata.

use super::{DecodeError, cursor::Cursor};
use crate::{
    MetadataHeader, MetadataRole, SCHEMA_ID, TimestampMetadata, validate_timestamp_metadata,
};

/// Parses one canonical timestamp signed body into fixed-capacity storage.
pub fn parse_timestamp_signed(bytes: &[u8]) -> Result<TimestampMetadata, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    cursor.byte(b'{')?;
    cursor.field("expires")?;
    let expires = cursor.number()?;
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Timestamp.as_str() {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("schema")?;
    if cursor.string()? != SCHEMA_ID {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("snapshot")?;
    let (snapshot_version, snapshot_length, snapshot_sha256) = parse_snapshot(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("version")?;
    let version = cursor.number()?;
    cursor.byte(b'}')?;
    if !cursor.is_complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let metadata = TimestampMetadata {
        header: MetadataHeader {
            role: MetadataRole::Timestamp,
            version,
            expires,
        },
        snapshot_version,
        snapshot_length,
        snapshot_sha256,
    };
    validate_timestamp_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

fn parse_snapshot(cursor: &mut Cursor<'_>) -> Result<(u64, u32, crate::Sha256Digest), DecodeError> {
    cursor.byte(b'{')?;
    cursor.field("length")?;
    let length = u32::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("sha256")?;
    let sha256 = crate::Sha256Digest(cursor.hex::<{ crate::SHA256_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("version")?;
    let version = cursor.number()?;
    cursor.byte(b'}')?;
    Ok((version, length, sha256))
}
