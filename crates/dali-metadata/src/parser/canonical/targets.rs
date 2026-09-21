//! Strict parser for canonical targets metadata.

use super::{DecodeError, cursor::Cursor};
use crate::{
    BoundedText, CartridgeId, MAX_DELEGATION_ID_BYTES, MAX_DELEGATION_SCOPES, MAX_TARGET_RECORDS,
    MetadataHeader, MetadataRole, SCHEMA_ID, Sha256Digest, TargetCartridge, TargetsMetadata,
    validate_target_records,
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
    cursor.field("cartridges")?;
    let (cartridges, cartridge_count) = parse_cartridges(&mut cursor)?;
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
        &cartridges[..usize::from(cartridge_count)],
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
        cartridges,
        cartridge_count,
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

fn parse_cartridges(
    cursor: &mut Cursor<'_>,
) -> Result<([TargetCartridge; MAX_TARGET_RECORDS], u16), DecodeError> {
    cursor.byte(b'[')?;
    let mut cartridges = [TargetCartridge::default(); MAX_TARGET_RECORDS];
    let mut count = 0_usize;
    if cursor.peek(b']') {
        cursor.byte(b']')?;
        return Ok((cartridges, 0));
    }
    loop {
        if count == MAX_TARGET_RECORDS {
            return Err(DecodeError::TooManyRecords);
        }
        cartridges[count] = parse_cartridge(cursor)?;
        count += 1;
        if cursor.peek(b',') {
            cursor.byte(b',')?;
        } else {
            break;
        }
    }
    cursor.byte(b']')?;
    Ok((cartridges, count as u16))
}

fn parse_cartridge(cursor: &mut Cursor<'_>) -> Result<TargetCartridge, DecodeError> {
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
    cursor.field("cartridge_id")?;
    let cartridge_id = CartridgeId(cursor.hex::<{ crate::KEY_ID_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("cartridge_version")?;
    let cartridge_version = bounded(cursor.string()?)?;
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
    Ok(TargetCartridge {
        cartridge_id,
        namespace,
        developer_id,
        delegation_id,
        developer_key_id,
        target_profile,
        amrn_format,
        abi_version,
        cartridge_version,
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
