//! Canonical Binary Metadata v2 envelope and bounded timestamp codec.

use crate::{
    DecodeError, EncodeError, MetadataHeader, MetadataRole, SignatureRecord, SignatureSet,
    TimestampMetadata, validate_signature_set, validate_timestamp_metadata,
};

/// Binary Metadata v2 magic bytes.
pub const BINARY_MAGIC: [u8; 4] = *b"DMB2";
/// Binary Metadata v2 format number.
pub const BINARY_FORMAT_VERSION: u8 = 2;
/// Fixed envelope header length before the signed body.
pub const BINARY_ENVELOPE_HEADER_BYTES: usize = 16;
/// Fixed size of one Ed25519 signature record.
pub const BINARY_SIGNATURE_RECORD_BYTES: usize = 80;

/// A borrowed Binary Metadata v2 envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryEnvelope<'a> {
    /// Role selected by the envelope.
    pub role: MetadataRole,
    /// Canonical signed body bytes.
    pub body: &'a [u8],
    /// Ed25519 signatures over `body`.
    pub signatures: SignatureSet,
}

/// Encodes one canonical Binary Metadata v2 envelope.
pub fn encode_binary_envelope(
    role: MetadataRole,
    body: &[u8],
    signatures: SignatureSet,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_signature_set(&signatures).map_err(|_| EncodeError::InvalidValue)?;
    let signature_count = usize::from(signatures.count);
    let signature_bytes = signature_count
        .checked_mul(BINARY_SIGNATURE_RECORD_BYTES)
        .ok_or(EncodeError::BufferTooSmall)?;
    let total = BINARY_ENVELOPE_HEADER_BYTES
        .checked_add(body.len())
        .and_then(|size| size.checked_add(signature_bytes))
        .ok_or(EncodeError::BufferTooSmall)?;
    if body.len() > u32::MAX as usize || output.len() < total {
        return Err(EncodeError::BufferTooSmall);
    }
    output[..4].copy_from_slice(&BINARY_MAGIC);
    output[4] = BINARY_FORMAT_VERSION;
    output[5] = role_number(role);
    output[6] = 0;
    output[7] = 0;
    write_u32(&mut output[8..12], body.len() as u32);
    output[12] = signatures.count;
    output[13..16].fill(0);
    output[16..16 + body.len()].copy_from_slice(body);
    let mut offset = 16 + body.len();
    for record in signatures.records[..signature_count].iter() {
        output[offset..offset + 16].copy_from_slice(&record.key_id.0);
        output[offset + 16..offset + 80].copy_from_slice(&record.signature.0);
        offset += BINARY_SIGNATURE_RECORD_BYTES;
    }
    Ok(total)
}

/// Parses and validates one canonical Binary Metadata v2 envelope.
pub fn parse_binary_envelope(bytes: &[u8]) -> Result<BinaryEnvelope<'_>, DecodeError> {
    if bytes.len() < BINARY_ENVELOPE_HEADER_BYTES {
        return Err(DecodeError::UnexpectedEnd);
    }
    if bytes[..4] != BINARY_MAGIC {
        return Err(DecodeError::InvalidMagic);
    }
    if bytes[4] != BINARY_FORMAT_VERSION {
        return Err(DecodeError::UnsupportedFormat);
    }
    if bytes[6] != 0 || bytes[7] != 0 || bytes[13..16].iter().any(|byte| *byte != 0) {
        return Err(DecodeError::InvalidValue);
    }
    let role = role_from_number(bytes[5]).ok_or(DecodeError::InvalidValue)?;
    let body_length = read_u32(&bytes[8..12]) as usize;
    let signature_count = usize::from(bytes[12]);
    if signature_count == 0 || signature_count > crate::MAX_SIGNATURES {
        return Err(DecodeError::TooManyRecords);
    }
    let signature_bytes = signature_count
        .checked_mul(BINARY_SIGNATURE_RECORD_BYTES)
        .ok_or(DecodeError::InvalidValue)?;
    let total = BINARY_ENVELOPE_HEADER_BYTES
        .checked_add(body_length)
        .and_then(|size| size.checked_add(signature_bytes))
        .ok_or(DecodeError::InvalidValue)?;
    if total != bytes.len() {
        return Err(DecodeError::TrailingBytes);
    }
    let body_end = BINARY_ENVELOPE_HEADER_BYTES + body_length;
    let mut records = [SignatureRecord::default(); crate::MAX_SIGNATURES];
    for (index, record) in records.iter_mut().take(signature_count).enumerate() {
        let start = body_end + index * BINARY_SIGNATURE_RECORD_BYTES;
        record.key_id.0.copy_from_slice(&bytes[start..start + 16]);
        record
            .signature
            .0
            .copy_from_slice(&bytes[start + 16..start + 80]);
    }
    let signatures = SignatureSet {
        records,
        count: signature_count as u8,
    };
    validate_signature_set(&signatures).map_err(|_| DecodeError::InvalidValue)?;
    Ok(BinaryEnvelope {
        role,
        body: &bytes[BINARY_ENVELOPE_HEADER_BYTES..body_end],
        signatures,
    })
}

/// Encodes the canonical Binary Metadata v2 timestamp body.
pub fn encode_binary_timestamp_body(
    metadata: TimestampMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_timestamp_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    if output.len() < 60 {
        return Err(EncodeError::BufferTooSmall);
    }
    let mut writer = BodyWriter::new(output);
    writer.u64(metadata.header.version)?;
    writer.u64(metadata.header.expires)?;
    writer.u64(metadata.snapshot_version)?;
    writer.u32(metadata.snapshot_length)?;
    writer.bytes(&metadata.snapshot_sha256.0)?;
    Ok(writer.position())
}

/// Parses the canonical Binary Metadata v2 timestamp body.
pub fn parse_binary_timestamp_body(bytes: &[u8]) -> Result<TimestampMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let metadata = TimestampMetadata {
        header: MetadataHeader {
            role: MetadataRole::Timestamp,
            version: reader.u64()?,
            expires: reader.u64()?,
        },
        snapshot_version: reader.u64()?,
        snapshot_length: reader.u32()?,
        snapshot_sha256: crate::Sha256Digest(reader.array()?),
    };
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    validate_timestamp_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

pub(crate) struct BodyWriter<'a> {
    output: &'a mut [u8],
    position: usize,
}

impl BodyWriter<'_> {
    pub(crate) const fn new(output: &mut [u8]) -> BodyWriter<'_> {
        BodyWriter {
            output,
            position: 0,
        }
    }

    pub(crate) fn bytes(&mut self, value: &[u8]) -> Result<(), EncodeError> {
        let end = self
            .position
            .checked_add(value.len())
            .ok_or(EncodeError::BufferTooSmall)?;
        if end > self.output.len() {
            return Err(EncodeError::BufferTooSmall);
        }
        self.output[self.position..end].copy_from_slice(value);
        self.position = end;
        Ok(())
    }

    pub(crate) fn u8(&mut self, value: u8) -> Result<(), EncodeError> {
        self.bytes(&[value])
    }

    pub(crate) fn u16(&mut self, value: u16) -> Result<(), EncodeError> {
        self.bytes(&value.to_le_bytes())
    }

    pub(crate) fn u32(&mut self, value: u32) -> Result<(), EncodeError> {
        self.bytes(&value.to_le_bytes())
    }

    pub(crate) fn u64(&mut self, value: u64) -> Result<(), EncodeError> {
        self.bytes(&value.to_le_bytes())
    }

    pub(crate) const fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn patch_u16(&mut self, position: usize, value: u16) -> Result<(), EncodeError> {
        let end = position.checked_add(2).ok_or(EncodeError::BufferTooSmall)?;
        let destination = self
            .output
            .get_mut(position..end)
            .ok_or(EncodeError::BufferTooSmall)?;
        destination.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}

pub(crate) struct BodyReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> BodyReader<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let end = self
            .position
            .checked_add(N)
            .ok_or(DecodeError::UnexpectedEnd)?;
        let source = self
            .bytes
            .get(self.position..end)
            .ok_or(DecodeError::UnexpectedEnd)?;
        let mut output = [0; N];
        output.copy_from_slice(source);
        self.position = end;
        Ok(output)
    }

    pub(crate) fn array_slice(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(DecodeError::UnexpectedEnd)?;
        let source = self
            .bytes
            .get(self.position..end)
            .ok_or(DecodeError::UnexpectedEnd)?;
        self.position = end;
        Ok(source)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub(crate) const fn complete(&self) -> bool {
        self.position == self.bytes.len()
    }
}

fn write_u32(output: &mut [u8], value: u32) {
    output.copy_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub(crate) const fn role_number(role: MetadataRole) -> u8 {
    match role {
        MetadataRole::Root => 1,
        MetadataRole::Timestamp => 2,
        MetadataRole::Snapshot => 3,
        MetadataRole::Targets => 4,
        MetadataRole::Delegation => 5,
        MetadataRole::Revocation => 6,
        MetadataRole::Recovery => 7,
        MetadataRole::Bundle => 8,
    }
}

pub(crate) const fn role_from_number(value: u8) -> Option<MetadataRole> {
    match value {
        1 => Some(MetadataRole::Root),
        2 => Some(MetadataRole::Timestamp),
        3 => Some(MetadataRole::Snapshot),
        4 => Some(MetadataRole::Targets),
        5 => Some(MetadataRole::Delegation),
        6 => Some(MetadataRole::Revocation),
        7 => Some(MetadataRole::Recovery),
        8 => Some(MetadataRole::Bundle),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Sha256Digest, Signature};

    #[test]
    fn round_trips_binary_envelope() {
        let signatures = SignatureSet {
            records: [
                SignatureRecord {
                    key_id: crate::KeyId([1; crate::KEY_ID_LENGTH]),
                    signature: Signature([2; crate::SIGNATURE_LENGTH]),
                },
                SignatureRecord::default(),
                SignatureRecord::default(),
                SignatureRecord::default(),
                SignatureRecord::default(),
                SignatureRecord::default(),
                SignatureRecord::default(),
                SignatureRecord::default(),
            ],
            count: 1,
        };
        let mut output = [0; 128];
        let length =
            encode_binary_envelope(MetadataRole::Timestamp, b"body", signatures, &mut output)
                .expect("binary envelope should encode");
        let parsed =
            parse_binary_envelope(&output[..length]).expect("binary envelope should parse");
        assert_eq!(parsed.role, MetadataRole::Timestamp);
        assert_eq!(parsed.body, b"body");
        assert_eq!(parsed.signatures, signatures);
    }

    #[test]
    fn rejects_binary_envelope_trailing_bytes() {
        let signatures = SignatureSet {
            records: [SignatureRecord {
                key_id: crate::KeyId([1; crate::KEY_ID_LENGTH]),
                signature: Signature([2; crate::SIGNATURE_LENGTH]),
            }; crate::MAX_SIGNATURES],
            count: 1,
        };
        let mut output = [0; 128];
        let length = encode_binary_envelope(MetadataRole::Root, b"body", signatures, &mut output)
            .expect("binary envelope should encode");
        output[length] = 0;
        assert_eq!(
            parse_binary_envelope(&output[..=length]),
            Err(DecodeError::TrailingBytes)
        );
    }

    #[test]
    fn round_trips_binary_timestamp_body() {
        let metadata = TimestampMetadata {
            header: MetadataHeader {
                role: MetadataRole::Timestamp,
                version: 3,
                expires: 99,
            },
            snapshot_version: 4,
            snapshot_length: 128,
            snapshot_sha256: Sha256Digest([7; crate::SHA256_LENGTH]),
        };
        let mut output = [0; 60];
        let length = encode_binary_timestamp_body(metadata, &mut output)
            .expect("timestamp body should encode");
        assert_eq!(parse_binary_timestamp_body(&output[..length]), Ok(metadata));
    }
}
