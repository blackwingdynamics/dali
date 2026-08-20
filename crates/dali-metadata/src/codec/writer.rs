use crate::{BoundedText, KeyId, PackageId, PublicKey, Sha256Digest};

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

pub(super) struct Writer<'a> {
    output: &'a mut [u8],
    length: usize,
    maximum: usize,
}

impl<'a> Writer<'a> {
    pub(super) const fn new(output: &'a mut [u8], maximum: usize) -> Self {
        Self {
            output,
            length: 0,
            maximum,
        }
    }

    pub(super) const fn len(&self) -> usize {
        self.length
    }

    pub(super) fn push(&mut self, byte: u8) -> Result<(), EncodeError> {
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

    pub(super) fn bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
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

    pub(super) fn string(&mut self, value: &str) -> Result<(), EncodeError> {
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

    pub(super) fn field_name(&mut self, name: &str) -> Result<(), EncodeError> {
        self.string(name)?;
        self.push(JSON_COLON)
    }

    pub(super) fn number(&mut self, value: u64) -> Result<(), EncodeError> {
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

    pub(super) fn text<const CAPACITY: usize>(
        &mut self,
        value: BoundedText<CAPACITY>,
    ) -> Result<(), EncodeError> {
        value
            .as_str()
            .ok_or(EncodeError::InvalidValue)
            .and_then(|value| self.string(value))
    }

    pub(super) fn hex<T: HexBytes>(&mut self, value: &T) -> Result<(), EncodeError> {
        self.push(JSON_QUOTE)?;
        for byte in value.as_bytes() {
            self.push(hex_digit(byte >> 4))?;
            self.push(hex_digit(byte & 0x0F))?;
        }
        self.push(JSON_QUOTE)
    }

    pub(super) fn object_start(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_LEFT_OBJECT)
    }

    pub(super) fn object_end(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_RIGHT_OBJECT)
    }

    pub(super) fn array_start(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_LEFT_ARRAY)
    }

    pub(super) fn array_end(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_RIGHT_ARRAY)
    }

    pub(super) fn comma(&mut self) -> Result<(), EncodeError> {
        self.push(JSON_COMMA)
    }
}

pub(super) trait HexBytes {
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

impl HexBytes for PackageId {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl HexBytes for Sha256Digest {
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
