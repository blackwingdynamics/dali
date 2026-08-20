//! Cursor and bounded collection parsing.

use core::str;

use super::{
    DecodeError,
    root::{
        hex_value, parse_role, validate_key_order, validate_role_key_order, validate_role_order,
    },
};
use crate::{
    KEY_ID_LENGTH, KeyId, MAX_ROLE_KEYS, MAX_ROOT_KEYS, MAX_ROOT_ROLES, MetadataRole,
    PUBLIC_KEY_LENGTH, PublicKey, RoleDefinition, RoleKey, validate_role,
};

pub(super) struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub(super) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(super) fn byte(&mut self, expected: u8) -> Result<(), DecodeError> {
        if self.bytes.get(self.offset) == Some(&expected) {
            self.offset += 1;
            Ok(())
        } else if self.offset == self.bytes.len() {
            Err(DecodeError::UnexpectedEnd)
        } else {
            Err(DecodeError::UnexpectedToken)
        }
    }

    pub(super) fn field(&mut self, expected: &str) -> Result<(), DecodeError> {
        if self.string()? != expected {
            return Err(DecodeError::InvalidFieldOrder);
        }
        self.byte(b':')
    }

    pub(super) fn string(&mut self) -> Result<&'a str, DecodeError> {
        self.byte(b'"')?;
        let start = self.offset;
        while let Some(&byte) = self.bytes.get(self.offset) {
            match byte {
                b'"' => {
                    let value = str::from_utf8(&self.bytes[start..self.offset])
                        .map_err(|_| DecodeError::InvalidString)?;
                    self.offset += 1;
                    return Ok(value);
                }
                b'\\' | 0..=0x1F => return Err(DecodeError::InvalidString),
                _ => self.offset += 1,
            }
        }
        Err(DecodeError::UnexpectedEnd)
    }

    pub(super) fn number(&mut self) -> Result<u64, DecodeError> {
        if self.bytes.get(self.offset) == Some(&b'0') {
            self.offset += 1;
            if self.bytes.get(self.offset).is_some_and(u8::is_ascii_digit) {
                return Err(DecodeError::InvalidNumber);
            }
            return Ok(0);
        }
        if !self.bytes.get(self.offset).is_some_and(u8::is_ascii_digit) {
            return Err(DecodeError::InvalidNumber);
        }
        let mut value = 0_u64;
        while let Some(&byte) = self.bytes.get(self.offset) {
            if !byte.is_ascii_digit() {
                break;
            }
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(u64::from(byte - b'0')))
                .ok_or(DecodeError::InvalidNumber)?;
            self.offset += 1;
        }
        Ok(value)
    }

    pub(super) fn keys(&mut self) -> Result<([RoleKey; MAX_ROOT_KEYS], u8), DecodeError> {
        self.byte(b'[')?;
        let empty = RoleKey {
            role: MetadataRole::Root,
            key_id: KeyId([0; KEY_ID_LENGTH]),
            public_key: PublicKey([0; PUBLIC_KEY_LENGTH]),
        };
        let mut keys = [empty; MAX_ROOT_KEYS];
        let mut count = 0_usize;
        if self.bytes.get(self.offset) == Some(&b']') {
            self.offset += 1;
            return Ok((keys, 0));
        }
        loop {
            if count == MAX_ROOT_KEYS {
                return Err(DecodeError::TooManyRecords);
            }
            keys[count] = self.key()?;
            count += 1;
            if self.bytes.get(self.offset) == Some(&b',') {
                self.offset += 1;
            } else {
                break;
            }
        }
        self.byte(b']')?;
        validate_role_key_order(&keys[..count])?;
        Ok((keys, count as u8))
    }

    fn key(&mut self) -> Result<RoleKey, DecodeError> {
        self.byte(b'{')?;
        self.field("key_id")?;
        let key_id = KeyId(self.hex::<KEY_ID_LENGTH>()?);
        self.byte(b',')?;
        self.field("public_key")?;
        let public_key = PublicKey(self.hex::<PUBLIC_KEY_LENGTH>()?);
        self.byte(b',')?;
        self.field("role")?;
        let role = parse_role(self.string()?)?;
        self.byte(b'}')?;
        if key_id.0 == [0; KEY_ID_LENGTH] || public_key.0 == [0; PUBLIC_KEY_LENGTH] {
            return Err(DecodeError::InvalidValue);
        }
        Ok(RoleKey {
            role,
            key_id,
            public_key,
        })
    }

    pub(super) fn roles(&mut self) -> Result<([RoleDefinition; MAX_ROOT_ROLES], u8), DecodeError> {
        self.byte(b'[')?;
        let empty = RoleDefinition {
            role: MetadataRole::Root,
            keys: [KeyId([0; KEY_ID_LENGTH]); MAX_ROLE_KEYS],
            key_count: 0,
            threshold: 0,
        };
        let mut roles = [empty; MAX_ROOT_ROLES];
        let mut count = 0_usize;
        if self.bytes.get(self.offset) == Some(&b']') {
            self.offset += 1;
            return Ok((roles, 0));
        }
        loop {
            if count == MAX_ROOT_ROLES {
                return Err(DecodeError::TooManyRecords);
            }
            roles[count] = self.role()?;
            count += 1;
            if self.bytes.get(self.offset) == Some(&b',') {
                self.offset += 1;
            } else {
                break;
            }
        }
        self.byte(b']')?;
        validate_role_order(&roles[..count])?;
        Ok((roles, count as u8))
    }

    fn role(&mut self) -> Result<RoleDefinition, DecodeError> {
        self.byte(b'{')?;
        self.field("key_ids")?;
        let (keys, key_count) = self.key_ids()?;
        self.byte(b',')?;
        self.field("name")?;
        let role = parse_role(self.string()?)?;
        self.byte(b',')?;
        self.field("threshold")?;
        let threshold = self.number()?;
        self.byte(b'}')?;
        let threshold = u8::try_from(threshold).map_err(|_| DecodeError::InvalidValue)?;
        let role = RoleDefinition {
            role,
            keys,
            key_count,
            threshold,
        };
        validate_role(role).map_err(|_| DecodeError::InvalidValue)?;
        Ok(role)
    }

    fn key_ids(&mut self) -> Result<([KeyId; MAX_ROLE_KEYS], u8), DecodeError> {
        self.byte(b'[')?;
        let mut keys = [KeyId([0; KEY_ID_LENGTH]); MAX_ROLE_KEYS];
        let mut count = 0_usize;
        if self.bytes.get(self.offset) == Some(&b']') {
            self.offset += 1;
            return Ok((keys, 0));
        }
        loop {
            if count == MAX_ROLE_KEYS {
                return Err(DecodeError::TooManyRecords);
            }
            keys[count] = KeyId(self.hex::<KEY_ID_LENGTH>()?);
            count += 1;
            if self.bytes.get(self.offset) == Some(&b',') {
                self.offset += 1;
            } else {
                break;
            }
        }
        self.byte(b']')?;
        validate_key_order(&keys[..count])?;
        Ok((keys, count as u8))
    }

    fn hex<const LENGTH: usize>(&mut self) -> Result<[u8; LENGTH], DecodeError> {
        let value = self.string()?;
        if value.len() != LENGTH * 2 {
            return Err(DecodeError::InvalidHex);
        }
        let mut output = [0; LENGTH];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            output[index] = (hex_value(pair[0])? << 4) | hex_value(pair[1])?;
        }
        Ok(output)
    }

    pub(super) const fn is_complete(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
