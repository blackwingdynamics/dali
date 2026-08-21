//! Incremental Binary Metadata v2 envelope parsing.
//!
//! The parser validates the fixed envelope while bytes arrive and forwards
//! only the signed body chunks to the caller. It never retains the body.

use crate::{
    BINARY_ENVELOPE_HEADER_BYTES, BINARY_FORMAT_VERSION, BINARY_MAGIC, DecodeError, KeyId,
    MetadataRole, Signature, SignatureRecord, SignatureSet, validate_signature_set,
};

/// Errors returned by the incremental Binary Metadata v2 envelope parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingDecodeError {
    /// The fixed envelope header is malformed or uses another role.
    InvalidEnvelope,
    /// The input ended before the declared body or signatures were complete.
    UnexpectedEnd,
    /// More bytes arrived after the declared envelope length.
    TrailingBytes,
    /// A signature record or key ordering rule is invalid.
    InvalidSignatures,
    /// The caller's role-body parser rejected a body chunk.
    Body(DecodeError),
}

/// Result of a completed Binary Metadata v2 envelope stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamedEnvelope {
    /// Role encoded in the envelope.
    pub role: MetadataRole,
    /// Exact signed body length declared by the envelope.
    pub body_length: u32,
    /// Bounded signature set collected after the body.
    pub signatures: SignatureSet,
    /// SHA-256 digest of the exact streamed body bytes.
    pub body_sha256: crate::Sha256Digest,
}

/// Incrementally validates one Binary Metadata v2 envelope.
///
/// The body is not retained. A role-specific parser should be passed to
/// [`BinaryEnvelopeStreamParser::feed`] and must retain only the fields it
/// needs for its policy decision.
pub struct BinaryEnvelopeStreamParser {
    expected_role: MetadataRole,
    header: [u8; BINARY_ENVELOPE_HEADER_BYTES],
    header_length: usize,
    body_length: u32,
    body_seen: u32,
    total_length: u64,
    total_seen: u64,
    signature_count: usize,
    signature_bytes: usize,
    signature_buffer: [u8; crate::BINARY_SIGNATURE_RECORD_BYTES],
    signature_buffered: usize,
    signatures: [SignatureRecord; crate::MAX_SIGNATURES],
    body_digest: dali_crypto::Sha256Accumulator,
}

impl BinaryEnvelopeStreamParser {
    /// Creates a parser that accepts only `expected_role`.
    pub fn new(expected_role: MetadataRole) -> Self {
        Self {
            expected_role,
            header: [0; BINARY_ENVELOPE_HEADER_BYTES],
            header_length: 0,
            body_length: 0,
            body_seen: 0,
            total_length: 0,
            total_seen: 0,
            signature_count: 0,
            signature_bytes: 0,
            signature_buffer: [0; crate::BINARY_SIGNATURE_RECORD_BYTES],
            signature_buffered: 0,
            signatures: [SignatureRecord {
                key_id: KeyId([0; crate::KEY_ID_LENGTH]),
                signature: Signature([0; crate::SIGNATURE_LENGTH]),
            }; crate::MAX_SIGNATURES],
            body_digest: dali_crypto::Sha256Accumulator::new(),
        }
    }

    /// Feeds one transport chunk and forwards signed body bytes to `consumer`.
    pub fn feed<F>(&mut self, bytes: &[u8], mut consumer: F) -> Result<(), StreamingDecodeError>
    where
        F: FnMut(&[u8]) -> Result<(), DecodeError>,
    {
        let mut offset = 0;
        while offset < bytes.len() {
            if self.total_seen == self.total_length && self.header_length != 0 {
                return Err(StreamingDecodeError::TrailingBytes);
            }
            if self.header_length < BINARY_ENVELOPE_HEADER_BYTES {
                self.header[self.header_length] = bytes[offset];
                self.header_length += 1;
                self.total_seen += 1;
                offset += 1;
                if self.header_length == BINARY_ENVELOPE_HEADER_BYTES {
                    self.finish_header()?;
                }
                continue;
            }
            if self.body_seen < self.body_length {
                let remaining = usize::try_from(self.body_length - self.body_seen)
                    .map_err(|_| StreamingDecodeError::InvalidEnvelope)?;
                let count = remaining.min(bytes.len() - offset);
                consumer(&bytes[offset..offset + count]).map_err(StreamingDecodeError::Body)?;
                self.body_digest.update(&bytes[offset..offset + count]);
                self.body_seen += count as u32;
                self.total_seen += count as u64;
                offset += count;
                continue;
            }
            let count = self.signature_bytes.min(bytes.len() - offset);
            self.signature_buffer[self.signature_buffered..self.signature_buffered + count]
                .copy_from_slice(&bytes[offset..offset + count]);
            self.signature_buffered += count;
            self.signature_bytes -= count;
            self.total_seen += count as u64;
            offset += count;
            if self.signature_buffered == crate::BINARY_SIGNATURE_RECORD_BYTES {
                self.store_signature()?;
            }
        }
        Ok(())
    }

    /// Completes the stream and validates declared lengths and signatures.
    pub fn finish(self) -> Result<StreamedEnvelope, StreamingDecodeError> {
        if self.header_length != BINARY_ENVELOPE_HEADER_BYTES
            || self.body_seen != self.body_length
            || self.signature_bytes != 0
            || self.signature_buffered != 0
            || self.total_seen != self.total_length
        {
            return Err(StreamingDecodeError::UnexpectedEnd);
        }
        let signatures = SignatureSet {
            records: self.signatures,
            count: self.signature_count as u8,
        };
        validate_signature_set(&signatures).map_err(|_| StreamingDecodeError::InvalidSignatures)?;
        Ok(StreamedEnvelope {
            role: self.expected_role,
            body_length: self.body_length,
            signatures,
            body_sha256: crate::Sha256Digest(self.body_digest.finalize()),
        })
    }

    fn finish_header(&mut self) -> Result<(), StreamingDecodeError> {
        if self.header[..4] != BINARY_MAGIC
            || self.header[4] != BINARY_FORMAT_VERSION
            || self.header[5] != role_number(self.expected_role)
            || self.header[6] != 0
            || self.header[7] != 0
            || self.header[13..16].iter().any(|byte| *byte != 0)
        {
            return Err(StreamingDecodeError::InvalidEnvelope);
        }
        self.body_length = u32::from_le_bytes([
            self.header[8],
            self.header[9],
            self.header[10],
            self.header[11],
        ]);
        self.signature_count = usize::from(self.header[12]);
        if self.signature_count == 0 || self.signature_count > crate::MAX_SIGNATURES {
            return Err(StreamingDecodeError::InvalidEnvelope);
        }
        self.signature_bytes = self
            .signature_count
            .checked_mul(crate::BINARY_SIGNATURE_RECORD_BYTES)
            .ok_or(StreamingDecodeError::InvalidEnvelope)?;
        self.total_length = BINARY_ENVELOPE_HEADER_BYTES as u64
            + u64::from(self.body_length)
            + self.signature_bytes as u64;
        Ok(())
    }

    fn store_signature(&mut self) -> Result<(), StreamingDecodeError> {
        let index = self
            .signature_count
            .checked_sub(self.signature_bytes / crate::BINARY_SIGNATURE_RECORD_BYTES + 1)
            .ok_or(StreamingDecodeError::InvalidSignatures)?;
        let record = self
            .signatures
            .get_mut(index)
            .ok_or(StreamingDecodeError::InvalidSignatures)?;
        record
            .key_id
            .0
            .copy_from_slice(&self.signature_buffer[..crate::KEY_ID_LENGTH]);
        record.signature.0.copy_from_slice(
            &self.signature_buffer[crate::KEY_ID_LENGTH..crate::BINARY_SIGNATURE_RECORD_BYTES],
        );
        self.signature_buffered = 0;
        Ok(())
    }
}

/// Streaming envelope parser for the root role.
pub type BinaryRootStreamParser = BinaryEnvelopeStreamParser;
/// Streaming envelope parser for the timestamp role.
pub type BinaryTimestampStreamParser = BinaryEnvelopeStreamParser;
/// Streaming envelope parser for the snapshot role.
pub type BinarySnapshotStreamParser = BinaryEnvelopeStreamParser;
/// Streaming envelope parser for the targets role.
pub type BinaryTargetsEnvelopeStreamParser = BinaryEnvelopeStreamParser;
/// Streaming envelope parser for the delegation role.
pub type BinaryDelegationStreamParser = BinaryEnvelopeStreamParser;
/// Streaming envelope parser for the revocation role.
pub type BinaryRevocationStreamParser = BinaryEnvelopeStreamParser;

const fn role_number(role: MetadataRole) -> u8 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Signature, encode_binary_envelope};

    fn envelope(role: MetadataRole) -> ([u8; 256], usize) {
        let signatures = SignatureSet {
            records: [SignatureRecord {
                key_id: crate::KeyId([1; crate::KEY_ID_LENGTH]),
                signature: Signature([2; crate::SIGNATURE_LENGTH]),
            }; crate::MAX_SIGNATURES],
            count: 1,
        };
        let mut output = [0; 256];
        let length = encode_binary_envelope(role, b"streamed-body", signatures, &mut output)
            .expect("test envelope fits");
        (output, length)
    }

    #[test]
    fn parses_every_supported_repository_role_in_small_chunks() {
        for role in [
            MetadataRole::Root,
            MetadataRole::Timestamp,
            MetadataRole::Snapshot,
            MetadataRole::Targets,
            MetadataRole::Delegation,
            MetadataRole::Revocation,
        ] {
            let (bytes, length) = envelope(role);
            let mut parser = BinaryEnvelopeStreamParser::new(role);
            let mut body = [0; 32];
            let mut body_length = 0;
            for chunk in bytes[..length].chunks(3) {
                parser
                    .feed(chunk, |part| {
                        body[body_length..body_length + part.len()].copy_from_slice(part);
                        body_length += part.len();
                        Ok(())
                    })
                    .expect("stream should parse");
            }
            let result = parser.finish().expect("stream should finish");
            assert_eq!(result.role, role);
            assert_eq!(&body[..body_length], b"streamed-body");
            let mut digest = dali_crypto::Sha256Accumulator::new();
            digest.update(b"streamed-body");
            assert_eq!(result.body_sha256.0, digest.finalize());
        }
    }

    #[test]
    fn rejects_wrong_role_without_consuming_body() {
        let (bytes, length) = envelope(MetadataRole::Root);
        let mut parser = BinaryEnvelopeStreamParser::new(MetadataRole::Targets);
        assert_eq!(
            parser.feed(&bytes[..length], |_| Ok(())),
            Err(StreamingDecodeError::InvalidEnvelope)
        );
    }
}
