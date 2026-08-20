use crate::{
    EncodeError, MAX_ROOT_KEYS, MAX_ROOT_ROLES, MetadataHeader, MetadataRole, RoleDefinition,
    RoleKey, SCHEMA_ID, validate_role, validate_role_references,
};

use super::writer::Writer;

/// Encodes the canonical signed body of a root metadata document.
pub fn encode_root_signed(
    output: &mut [u8],
    header: MetadataHeader,
    keys: &[RoleKey],
    roles: &[RoleDefinition],
) -> Result<usize, EncodeError> {
    validate_root_input(header, keys, roles)?;
    let mut writer = Writer::new(output, crate::MAX_ROOT_BYTES);
    encode_header(&mut writer, header, keys, roles)?;
    Ok(writer.len())
}

fn validate_root_input(
    header: MetadataHeader,
    keys: &[RoleKey],
    roles: &[RoleDefinition],
) -> Result<(), EncodeError> {
    if keys.len() > MAX_ROOT_KEYS || roles.len() > MAX_ROOT_ROLES {
        return Err(EncodeError::TooManyRecords);
    }
    if header.role != MetadataRole::Root || header.version == 0 {
        return Err(EncodeError::InvalidValue);
    }
    if keys.is_empty() || roles.is_empty() || !keys.iter().any(|key| key.role == MetadataRole::Root)
    {
        return Err(EncodeError::InvalidValue);
    }
    validate_order(keys, |left, right| left.key_id.0 < right.key_id.0)?;
    validate_order(roles, |left, right| {
        left.role.as_str() < right.role.as_str()
    })?;
    for role in roles {
        validate_role(*role).map_err(|_| EncodeError::InvalidValue)?;
        let active = usize::from(role.key_count);
        validate_order(&role.keys[..active], |left, right| left.0 < right.0)?;
    }
    validate_role_references(keys, roles).map_err(|_| EncodeError::InvalidValue)?;
    Ok(())
}

fn encode_header(
    writer: &mut Writer<'_>,
    header: MetadataHeader,
    keys: &[RoleKey],
    roles: &[RoleDefinition],
) -> Result<(), EncodeError> {
    writer.object_start()?;
    writer.field_name("expires")?;
    writer.number(header.expires)?;
    writer.comma()?;
    writer.field_name("keys")?;
    writer.array_start()?;
    for (index, key) in keys.iter().enumerate() {
        if index != 0 {
            writer.comma()?;
        }
        encode_key(writer, *key)?;
    }
    writer.array_end()?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(header.role.as_str())?;
    writer.comma()?;
    writer.field_name("roles")?;
    writer.array_start()?;
    for (index, role) in roles.iter().enumerate() {
        if index != 0 {
            writer.comma()?;
        }
        encode_role(writer, *role)?;
    }
    writer.array_end()?;
    writer.comma()?;
    writer.field_name("schema")?;
    writer.string(SCHEMA_ID)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(header.version)?;
    writer.object_end()
}

fn encode_key(writer: &mut Writer<'_>, key: RoleKey) -> Result<(), EncodeError> {
    if key.key_id.0 == [0; crate::KEY_ID_LENGTH]
        || key.public_key.0 == [0; crate::PUBLIC_KEY_LENGTH]
    {
        return Err(EncodeError::InvalidValue);
    }
    writer.object_start()?;
    writer.field_name("key_id")?;
    writer.hex(&key.key_id)?;
    writer.comma()?;
    writer.field_name("public_key")?;
    writer.hex(&key.public_key)?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(key.role.as_str())?;
    writer.object_end()
}

fn encode_role(writer: &mut Writer<'_>, role: RoleDefinition) -> Result<(), EncodeError> {
    writer.object_start()?;
    writer.field_name("key_ids")?;
    writer.array_start()?;
    for (index, key) in role
        .keys
        .iter()
        .take(usize::from(role.key_count))
        .enumerate()
    {
        if index != 0 {
            writer.comma()?;
        }
        writer.hex(key)?;
    }
    writer.array_end()?;
    writer.comma()?;
    writer.field_name("name")?;
    writer.string(role.role.as_str())?;
    writer.comma()?;
    writer.field_name("threshold")?;
    writer.number(u64::from(role.threshold))?;
    writer.object_end()
}

fn validate_order<T>(records: &[T], ordered: impl Fn(&T, &T) -> bool) -> Result<(), EncodeError> {
    if records.windows(2).all(|pair| ordered(&pair[0], &pair[1])) {
        Ok(())
    } else {
        Err(EncodeError::NonCanonicalOrder)
    }
}
