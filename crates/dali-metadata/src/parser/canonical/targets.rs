//! Strict parser for canonical targets metadata.

use super::{DecodeError, cursor::Cursor};
use crate::{
    BoundedText, MAX_DELEGATION_ID_BYTES, MAX_DELEGATION_SCOPES, MAX_TARGET_RECORDS,
    MetadataHeader, MetadataRole, PackageId, SCHEMA_ID, Sha256Digest, TargetPackage,
    TargetsMetadata, validate_target_records,
};

/// Parses one canonical targets signed body into fixed-capacity storage.
pub fn parse_targets_signed(bytes: &[u8]) -> Result<TargetsMetadata, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    cursor.byte(b'{')?;
    cursor.field("delegations")?;
    let (delegations, delegation_count) = parse_delegations(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("expires")?;
    let expires = cursor.number()?;
    cursor.byte(b',')?;
    cursor.field("packages")?;
    let (packages, package_count) = parse_packages(&mut cursor)?;
    cursor.byte(b',')?;
    cursor.field("role")?;
    if cursor.string()? != MetadataRole::Targets.as_str() {
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
    if !cursor.is_complete() || version == 0 {
        return Err(if cursor.is_complete() {
            DecodeError::InvalidValue
        } else {
            DecodeError::TrailingBytes
        });
    }
    validate_target_records(
        &delegations[..usize::from(delegation_count)],
        &packages[..usize::from(package_count)],
    )
    .map_err(|_| DecodeError::InvalidValue)?;
    Ok(TargetsMetadata {
        header: MetadataHeader {
            role: MetadataRole::Targets,
            version,
            expires,
        },
        delegations,
        delegation_count,
        packages,
        package_count,
    })
}

fn parse_delegations(
    cursor: &mut Cursor<'_>,
) -> Result<
    (
        [BoundedText<MAX_DELEGATION_ID_BYTES>; MAX_DELEGATION_SCOPES],
        u8,
    ),
    DecodeError,
> {
    cursor.byte(b'[')?;
    let mut delegations = [BoundedText::default(); MAX_DELEGATION_SCOPES];
    let mut count = 0_usize;
    if cursor.peek(b']') {
        cursor.byte(b']')?;
        return Ok((delegations, 0));
    }
    loop {
        if count == MAX_DELEGATION_SCOPES {
            return Err(DecodeError::TooManyRecords);
        }
        delegations[count] = bounded(cursor.string()?)?;
        count += 1;
        if cursor.peek(b',') {
            cursor.byte(b',')?;
        } else {
            break;
        }
    }
    cursor.byte(b']')?;
    Ok((delegations, count as u8))
}

fn parse_packages(
    cursor: &mut Cursor<'_>,
) -> Result<([TargetPackage; MAX_TARGET_RECORDS], u16), DecodeError> {
    cursor.byte(b'[')?;
    let mut packages = [TargetPackage::default(); MAX_TARGET_RECORDS];
    let mut count = 0_usize;
    if cursor.peek(b']') {
        cursor.byte(b']')?;
        return Ok((packages, 0));
    }
    loop {
        if count == MAX_TARGET_RECORDS {
            return Err(DecodeError::TooManyRecords);
        }
        packages[count] = parse_package(cursor)?;
        count += 1;
        if cursor.peek(b',') {
            cursor.byte(b',')?;
        } else {
            break;
        }
    }
    cursor.byte(b']')?;
    Ok((packages, count as u16))
}

fn parse_package(cursor: &mut Cursor<'_>) -> Result<TargetPackage, DecodeError> {
    cursor.byte(b'{')?;
    cursor.field("abi_version")?;
    let abi_version = u16::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("amrn_format")?;
    let amrn_format = u16::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("developer_id")?;
    let developer_id = bounded(cursor.string()?)?;
    cursor.byte(b',')?;
    cursor.field("developer_key_id")?;
    let developer_key_id = crate::KeyId(cursor.hex::<{ crate::KEY_ID_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("delegation_id")?;
    let delegation_id = bounded(cursor.string()?)?;
    cursor.byte(b',')?;
    cursor.field("length")?;
    let length = u32::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("minimum_kernel_version")?;
    let minimum_kernel_version = bounded(cursor.string()?)?;
    cursor.byte(b',')?;
    cursor.field("namespace")?;
    let namespace = bounded(cursor.string()?)?;
    cursor.byte(b',')?;
    cursor.field("package_id")?;
    let package_id = PackageId(cursor.hex::<{ crate::KEY_ID_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("package_version")?;
    let package_version = bounded(cursor.string()?)?;
    cursor.byte(b',')?;
    cursor.field("required_services")?;
    let required_services =
        u32::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("sha256")?;
    let sha256 = Sha256Digest(cursor.hex::<{ crate::SHA256_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("slot_id")?;
    let slot_id = u8::try_from(cursor.number()?).map_err(|_| DecodeError::InvalidValue)?;
    cursor.byte(b',')?;
    cursor.field("target_profile")?;
    let target_profile = bounded(cursor.string()?)?;
    cursor.byte(b'}')?;
    Ok(TargetPackage {
        package_id,
        namespace,
        developer_id,
        delegation_id,
        developer_key_id,
        target_profile,
        amrn_format,
        abi_version,
        package_version,
        minimum_kernel_version,
        length,
        sha256,
        required_services,
        slot_id,
    })
}

fn bounded<const CAPACITY: usize>(value: &str) -> Result<BoundedText<CAPACITY>, DecodeError> {
    BoundedText::new(value).map_err(|_| DecodeError::InvalidValue)
}
