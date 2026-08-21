//! Strict parser for canonical snapshot metadata.

use super::{DecodeError, cursor::Cursor};
use crate::{
    DelegationReference, MAX_SNAPSHOT_REFERENCES, MetadataHeader, MetadataRole,
    RevocationReference, SCHEMA_ID, SnapshotMetadata, TargetsReference, validate_snapshot_metadata,
};

/// Parses one canonical snapshot signed body into fixed-capacity storage.
pub fn parse_snapshot_signed(bytes: &[u8]) -> Result<SnapshotMetadata, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    cursor.byte(b'{')?;
    cursor.field("expires")?;
    let expires = cursor.number()?;
    cursor.byte(b',')?;
    cursor.field("metadata")?;
    let (targets, revocations, delegations, delegation_count) = parse_references(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Snapshot.as_str() {
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
    let metadata = SnapshotMetadata {
        header: MetadataHeader {
            role: MetadataRole::Snapshot,
            version,
            expires,
        },
        targets,
        revocations,
        delegations,
        delegation_count,
    };
    validate_snapshot_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

fn parse_references(
    cursor: &mut Cursor<'_>,
) -> Result<
    (
        TargetsReference,
        RevocationReference,
        [DelegationReference; MAX_SNAPSHOT_REFERENCES],
        u8,
    ),
    DecodeError,
> {
    cursor.byte(b'[')?;
    let mut delegations = [DelegationReference::default(); MAX_SNAPSHOT_REFERENCES];
    if cursor.peek(b']') {
        return Err(DecodeError::InvalidValue);
    }
    let targets = parse_targets_reference(cursor)?;
    let revocations = parse_revocation_reference(cursor)?;
    let mut count = 0_usize;
    while cursor.peek(b',') {
        cursor.byte(b',')?;
        if count == MAX_SNAPSHOT_REFERENCES {
            return Err(DecodeError::TooManyRecords);
        }
        delegations[count] = parse_delegation_reference(cursor)?;
        count += 1;
    }
    cursor.byte(b']')?;
    Ok((targets, revocations, delegations, count as u8))
}

fn parse_revocation_reference(cursor: &mut Cursor<'_>) -> Result<RevocationReference, DecodeError> {
    cursor.byte(b',')?;
    cursor.byte(b'{')?;
    cursor.field("length")?;
    let length = u32::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Revocation.as_str() {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("sha256")?;
    let sha256 = crate::Sha256Digest(cursor.hex::<{ crate::SHA256_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("version")?;
    let version = cursor.number()?;
    cursor.byte(b'}')?;
    Ok(RevocationReference {
        version,
        length,
        sha256,
    })
}

fn parse_targets_reference(cursor: &mut Cursor<'_>) -> Result<TargetsReference, DecodeError> {
    cursor.byte(b'{')?;
    cursor.field("length")?;
    let length = u32::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Targets.as_str() {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("sha256")?;
    let sha256 = crate::Sha256Digest(cursor.hex::<{ crate::SHA256_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("version")?;
    let version = cursor.number()?;
    cursor.byte(b'}')?;
    Ok(TargetsReference {
        version,
        length,
        sha256,
    })
}

fn parse_delegation_reference(cursor: &mut Cursor<'_>) -> Result<DelegationReference, DecodeError> {
    cursor.byte(b'{')?;
    cursor.field("id")?;
    let id = crate::BoundedText::new(cursor.string()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("length")?;
    let length = u32::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Delegation.as_str() {
        return Err(DecodeError::InvalidValue);
    }
    cursor.byte(b',')?;
    cursor.field("sha256")?;
    let sha256 = crate::Sha256Digest(cursor.hex::<{ crate::SHA256_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("version")?;
    let version = cursor.number()?;
    cursor.byte(b'}')?;
    Ok(DelegationReference {
        id,
        version,
        length,
        sha256,
    })
}
