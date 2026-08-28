//! Strict parser for canonical offline bundle manifests.

use super::{DecodeError, cursor::Cursor};
use crate::{
    BoundedText, BundleFile, BundleFileKind, BundleMetadata, MAX_BUNDLE_FILES, MAX_BUNDLE_ID_BYTES,
    MAX_TARGET_PROFILE_BYTES, MetadataHeader, MetadataRole, SCHEMA_ID, Sha256Digest,
    validate_bundle_metadata,
};

/// Parses one canonical offline bundle manifest signed body.
pub fn parse_bundle_signed(bytes: &[u8]) -> Result<BundleMetadata, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    cursor.byte(b'{')?;
    cursor.field("files")?;
    let (files, file_count) = parse_files(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Bundle.as_str() {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("schema")?;
    if cursor.string()? != SCHEMA_ID {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("target_profile")?;
    let target_profile = BoundedText::<MAX_TARGET_PROFILE_BYTES>::new(cursor.string()?)
        .map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("version")?;
    let version = cursor.number()?;
    cursor.byte(b'}')?;
    if !cursor.is_complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let metadata = BundleMetadata {
        header: MetadataHeader {
            role: MetadataRole::Bundle,
            version,
            expires: 0,
        },
        target_profile,
        files,
        file_count,
    };
    validate_bundle_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

fn parse_files(
    cursor: &mut Cursor<'_>,
) -> Result<([BundleFile; MAX_BUNDLE_FILES], u16), DecodeError> {
    let mut files = [BundleFile::default(); MAX_BUNDLE_FILES];
    cursor.byte(b'[')?;
    let mut count = 0_usize;
    if cursor.peek(b']') {
        cursor.byte(b']')?;
        return Ok((files, 0));
    }
    loop {
        if count == MAX_BUNDLE_FILES {
            return Err(DecodeError::TooManyRecords);
        }
        files[count] = parse_file(cursor)?;
        count += 1;
        if cursor.peek(b',') {
            cursor.byte(b',')?;
        } else {
            break;
        }
    }
    cursor.byte(b']')?;
    Ok((
        files,
        u16::try_from(count).map_err(|_| DecodeError::TooManyRecords)?,
    ))
}

fn parse_file(cursor: &mut Cursor<'_>) -> Result<BundleFile, DecodeError> {
    cursor.byte(b'{')?;
    cursor.field("id")?;
    let id = BoundedText::<MAX_BUNDLE_ID_BYTES>::new(cursor.string()?)
        .map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("kind")?;
    let kind = parse_kind(cursor.string()?)?;
    cursor.byte(b',')?;
    cursor.field("length")?;
    let length = u32::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("sha256")?;
    let sha256 = Sha256Digest(cursor.hex::<{ crate::SHA256_LENGTH }>()?);
    cursor.byte(b'}')?;
    Ok(BundleFile {
        kind,
        id,
        length,
        sha256,
    })
}

fn parse_kind(value: &str) -> Result<BundleFileKind, DecodeError> {
    match value {
        "root" => Ok(BundleFileKind::Root),
        "timestamp" => Ok(BundleFileKind::Timestamp),
        "snapshot" => Ok(BundleFileKind::Snapshot),
        "targets" => Ok(BundleFileKind::Targets),
        "revocation" => Ok(BundleFileKind::Revocation),
        "delegation" => Ok(BundleFileKind::Delegation),
        "cartridge" => Ok(BundleFileKind::Cartridge),
        _ => Err(DecodeError::InvalidValue),
    }
}
