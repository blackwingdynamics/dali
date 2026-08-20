//! Canonical, bounded metadata encoders.

use crate::{
    KeyId, MAX_ROOT_BYTES, MAX_ROOT_KEYS, MAX_ROOT_ROLES, MetadataHeader, MetadataRole, PublicKey,
    RoleDefinition, RoleKey, SCHEMA_ID, validate_role,
};

const JSON_QUOTE: u8 = b'"';
const JSON_COMMA: u8 = b',';
const JSON_COLON: u8 = b':';
const JSON_LEFT_OBJECT: u8 = b'{';
const JSON_RIGHT_OBJECT: u8 = b'}';
const JSON_LEFT_ARRAY: u8 = b'[';
const JSON_RIGHT_ARRAY: u8 = b']';

/// Errors returned by bounded canonical metadata encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    /// The caller-owned output buffer cannot hold the complete signed body.
    BufferTooSmall,
    /// A record count exceeds the active metadata profile limit.
    TooManyRecords,
    /// Records are not in the canonical order required by the wire contract.
    NonCanonicalOrder,
    /// A supplied contract value is invalid.
    InvalidValue,
}

/// Encodes the canonical signed body of a root metadata document.
///
/// The output is the exact byte range that the root-role signer must sign. It
/// contains no envelope or signature fields. Input records must already be in
/// canonical order: keys by `key_id`, roles by role name, and role key IDs by
/// `key_id`.
pub fn encode_root_signed(
    output: &mut [u8],
    header: MetadataHeader,
    keys: &[RoleKey],
    roles: &[RoleDefinition],
) -> Result<usize, EncodeError> {
    if keys.len() > MAX_ROOT_KEYS || roles.len() > MAX_ROOT_ROLES {
        return Err(EncodeError::TooManyRecords);
    }
    if header.role != MetadataRole::Root || header.version == 0 {
        return Err(EncodeError::InvalidValue);
    }
    validate_order(keys, |left, right| left.key_id.0 <= right.key_id.0)?;
    validate_order(roles, |left, right| {
        left.role.as_str() <= right.role.as_str()
    })?;
    for role in roles {
        validate_role(*role).map_err(|_| EncodeError::InvalidValue)?;
        let active = usize::from(role.key_count);
        validate_order(&role.keys[..active], |left, right| left.0 <= right.0)?;
    }

    let mut writer = Writer::new(output, MAX_ROOT_BYTES);
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
        encode_key(&mut writer, *key)?;
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
        encode_role(&mut writer, *role)?;
    }
    writer.array_end()?;
    writer.comma()?;
    writer.field_name("schema")?;
    writer.string(SCHEMA_ID)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(header.version)?;
    writer.object_end()?;
    Ok(writer.len())
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

struct Writer<'a> {
    output: &'a mut [u8],
    length: usize,
    maximum: usize,
}

impl<'a> Writer<'a> {
    const fn new(output: &'a mut [u8], maximum: usize) -> Self {
        Self {
            output,
            length: 0,
            maximum,
        }
    }

    const fn len(&self) -> usize {
        self.length
    }

    fn push(&mut self, byte: u8) -> Result<(), EncodeError> {
        if self.length >= self.maximum {
            return Err(EncodeError::BufferTooSmall);
        }
        let destination = self
            .output
            .get_mut(self.length)
            .ok_or(EncodeError::BufferTooSmall)?;
        *destination = byte;
        self.length += 1;
        Ok(())
    }

    fn bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        if self.length > self.maximum
            || self.maximum - self.length < bytes.len()
            || self.output.len().saturating_sub(self.length) < bytes.len()
        {
            return Err(EncodeError::BufferTooSmall);
        }
        let end = self.length + bytes.len();
        self.output[self.length..end].copy_from_slice(bytes);
        self.length = end;
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), EncodeError> {
        if value
            .bytes()
            .any(|byte| byte < 0x20 || byte == b'"' || byte == b'\\')
        {
            return Err(EncodeError::InvalidValue);
        }
        self.push(JSON_QUOTE)?;
        self.bytes(value.as_bytes())?;
        self.push(JSON_QUOTE)
    }

    fn field_name(&mut self, name: &str) -> Result<(), EncodeError> {
        self.string(name)?;
        self.push(JSON_COLON)
    }

    fn number(&mut self, value: u64) -> Result<(), EncodeError> {
        let mut buffer = [0; 20];
        let mut index = buffer.len();
        let mut remaining = value;
        loop {
            index -= 1;
            buffer[index] = b'0' + (remaining % 10) as u8;
            remaining /= 10;
            if remaining == 0 {
                break;
            }
        }
        self.bytes(&buffer[index..])
    }

    fn hex<T: HexBytes>(&mut self, value: &T) -> Result<(), EncodeError> {
        self.push(JSON_QUOTE)?;
        for byte in value.as_bytes() {
            self.push(hex_digit(byte >> 4))?;
            self.push(hex_digit(byte & 0x0F))?;
        }
        self.push(JSON_QUOTE)
    }

    fn object_start(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_LEFT_OBJECT)
    }

    fn object_end(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_RIGHT_OBJECT)
    }

    fn array_start(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_LEFT_ARRAY)
    }

    fn array_end(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_RIGHT_ARRAY)
    }

    fn comma(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_COMMA)
    }
}

trait HexBytes {
    fn as_bytes(&self) -> &[u8];
}

impl HexBytes for KeyId {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl HexBytes for PublicKey {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'a' + value - 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_ROLE_KEYS;

    const KEY: RoleKey = RoleKey {
        role: MetadataRole::Targets,
        key_id: KeyId([1; crate::KEY_ID_LENGTH]),
        public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
    };

    fn root_role(key_id: KeyId) -> RoleDefinition {
        let mut keys = [KeyId([0; crate::KEY_ID_LENGTH]); MAX_ROLE_KEYS];
        keys[0] = key_id;
        RoleDefinition {
            role: MetadataRole::Targets,
            keys,
            key_count: 1,
            threshold: 1,
        }
    }

    #[test]
    fn encodes_the_canonical_root_signed_body() {
        let mut output = [0; MAX_ROOT_BYTES];
        let length = encode_root_signed(
            &mut output,
            MetadataHeader {
                role: MetadataRole::Root,
                version: 1,
                expires: 0,
            },
            &[KEY],
            &[root_role(KEY.key_id)],
        )
        .expect("root body should encode");
        assert_eq!(
            &output[..length],
            br#"{"expires":0,"keys":[{"key_id":"01010101010101010101010101010101","public_key":"0202020202020202020202020202020202020202020202020202020202020202","role":"targets"}],"role":"root","roles":[{"key_ids":["01010101010101010101010101010101"],"name":"targets","threshold":1}],"schema":"dali.metadata.v1","version":1}"#
        );
    }

    #[test]
    fn rejects_non_canonical_key_order() {
        let mut output = [0; MAX_ROOT_BYTES];
        let first = KEY;
        let second = RoleKey {
            key_id: KeyId([0; crate::KEY_ID_LENGTH]),
            ..KEY
        };
        assert_eq!(
            encode_root_signed(
                &mut output,
                MetadataHeader {
                    role: MetadataRole::Root,
                    version: 1,
                    expires: 0,
                },
                &[first, second],
                &[],
            ),
            Err(EncodeError::NonCanonicalOrder)
        );
    }

    #[test]
    fn rejects_an_output_buffer_that_is_too_small() {
        let mut output = [0; 8];
        assert_eq!(
            encode_root_signed(
                &mut output,
                MetadataHeader {
                    role: MetadataRole::Root,
                    version: 1,
                    expires: 0,
                },
                &[KEY],
                &[root_role(KEY.key_id)],
            ),
            Err(EncodeError::BufferTooSmall)
        );
    }
}
