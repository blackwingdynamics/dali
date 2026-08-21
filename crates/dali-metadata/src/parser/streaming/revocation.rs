//! Incremental Binary Metadata v2 revocation-body parser.

use crate::{
    BoundedText, KeyId, MetadataHeader, MetadataRole, RevocationMetadata, RevocationRecord,
    StreamingBodyError, validate_revocation_metadata,
};
use core::mem::MaybeUninit;

const QUEUE_BYTES: usize = 512;
const TEXT_BYTES: usize = crate::MAX_REVOCATION_REASON_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Version,
    Expires,
    Count,
    DeveloperLength,
    Developer,
    EffectiveVersion,
    Issuer,
    Key,
    ReasonLength,
    Reason,
    Complete,
}

/// Incrementally parses explicit revocation records without retaining the body.
pub struct BinaryRevocationBodyStreamParser<const CAPACITY: usize = { crate::MAX_REVOCATIONS }> {
    queue: ByteQueue,
    phase: Phase,
    version: u64,
    expires: u64,
    records: [RevocationRecord; CAPACITY],
    record_count: u8,
    record_index: usize,
    text_length: usize,
    text: [u8; TEXT_BYTES],
}

impl<const CAPACITY: usize> BinaryRevocationBodyStreamParser<CAPACITY> {
    /// Creates an empty revocation body parser.
    pub fn new() -> Self {
        Self {
            queue: ByteQueue::new(),
            phase: Phase::Version,
            version: 0,
            expires: 0,
            records: [RevocationRecord::default(); CAPACITY],
            record_count: 0,
            record_index: 0,
            text_length: 0,
            text: [0; TEXT_BYTES],
        }
    }

    /// Feeds one bounded transport fragment.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), StreamingBodyError> {
        for byte in bytes {
            self.queue.push(*byte).map_err(map_error)?;
            self.drive().map_err(map_error)?;
        }
        self.drive().map_err(map_error)
    }

    /// Completes parsing and validates all revocation records.
    pub fn finish(&mut self) -> Result<RevocationMetadata, StreamingBodyError> {
        let mut output = MaybeUninit::uninit();
        self.finish_into(&mut output)?;
        // SAFETY: finish_into initializes output before returning Ok.
        Ok(unsafe { output.assume_init() })
    }

    /// Completes parsing directly into caller-owned output storage.
    pub fn finish_into(
        &mut self,
        output: &mut MaybeUninit<RevocationMetadata>,
    ) -> Result<(), StreamingBodyError> {
        self.drive().map_err(map_error)?;
        if self.phase != Phase::Complete || !self.queue.is_empty() {
            return Err(StreamingBodyError::UnexpectedEnd);
        }
        let mut records = [RevocationRecord::default(); crate::MAX_REVOCATIONS];
        records[..usize::from(self.record_count)]
            .copy_from_slice(&self.records[..usize::from(self.record_count)]);
        let metadata = RevocationMetadata {
            header: MetadataHeader {
                role: MetadataRole::Revocation,
                version: self.version,
                expires: self.expires,
            },
            records,
            record_count: self.record_count,
        };
        validate_revocation_metadata(&metadata).map_err(|_| StreamingBodyError::InvalidBody)?;
        output.write(metadata);
        Ok(())
    }

    fn drive(&mut self) -> Result<(), Error> {
        loop {
            let progressed = match self.phase {
                Phase::Version => self.u64_field(|this, value| {
                    this.version = value;
                    this.phase = Phase::Expires
                }),
                Phase::Expires => self.u64_field(|this, value| {
                    this.expires = value;
                    this.phase = Phase::Count
                }),
                Phase::Count => self.count(),
                Phase::DeveloperLength | Phase::ReasonLength => self.text_length(),
                Phase::Developer => self.developer(),
                Phase::EffectiveVersion => self.u64_field(|this, value| {
                    this.records[this.record_index].effective_version = value;
                    this.phase = Phase::Issuer
                }),
                Phase::Issuer => self.key(true),
                Phase::Key => self.key(false),
                Phase::Reason => self.reason(),
                Phase::Complete => Ok(false),
            }?;
            if !progressed {
                return Ok(());
            }
        }
    }

    fn u64_field(&mut self, set: impl FnOnce(&mut Self, u64)) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<8>() else {
            return Ok(false);
        };
        set(self, u64::from_le_bytes(value));
        Ok(true)
    }
    fn count(&mut self) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<2>() else {
            return Ok(false);
        };
        if usize::from(u16::from_le_bytes(value)) > CAPACITY {
            return Err(Error::Invalid);
        }
        self.record_count = u8::try_from(u16::from_le_bytes(value)).map_err(|_| Error::Invalid)?;
        self.record_index = 0;
        self.phase = if self.record_count == 0 {
            Phase::Complete
        } else {
            Phase::DeveloperLength
        };
        Ok(true)
    }
    fn text_length(&mut self) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<2>() else {
            return Ok(false);
        };
        self.text_length = usize::from(u16::from_le_bytes(value));
        if self.text_length > TEXT_BYTES {
            return Err(Error::Invalid);
        }
        self.phase = match self.phase {
            Phase::DeveloperLength => Phase::Developer,
            Phase::ReasonLength => Phase::Reason,
            _ => return Err(Error::Invalid),
        };
        Ok(true)
    }
    fn developer(&mut self) -> Result<bool, Error> {
        let Some(value) = self.text_value::<{ crate::MAX_DEVELOPER_ID_BYTES }>()? else {
            return Ok(false);
        };
        self.records[self.record_index].developer_id = value;
        self.phase = Phase::EffectiveVersion;
        Ok(true)
    }
    fn key(&mut self, issuer: bool) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<{ crate::KEY_ID_LENGTH }>() else {
            return Ok(false);
        };
        if issuer {
            self.records[self.record_index].issuer_key_id = KeyId(value);
            self.phase = Phase::Key;
        } else {
            self.records[self.record_index].key_id = KeyId(value);
            self.phase = Phase::ReasonLength;
        }
        Ok(true)
    }
    fn reason(&mut self) -> Result<bool, Error> {
        let Some(value) = self.text_value::<{ crate::MAX_REVOCATION_REASON_BYTES }>()? else {
            return Ok(false);
        };
        self.records[self.record_index].reason = value;
        self.record_index += 1;
        self.phase = if self.record_index == usize::from(self.record_count) {
            Phase::Complete
        } else {
            Phase::DeveloperLength
        };
        Ok(true)
    }
    fn text_value<const N: usize>(&mut self) -> Result<Option<BoundedText<N>>, Error> {
        if !self.queue.take_into(self.text_length, &mut self.text) {
            return Ok(None);
        }
        let value =
            core::str::from_utf8(&self.text[..self.text_length]).map_err(|_| Error::Invalid)?;
        BoundedText::new(value)
            .map(Some)
            .map_err(|_| Error::Invalid)
    }
}

impl<const CAPACITY: usize> Default for BinaryRevocationBodyStreamParser<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Error {
    QueueFull,
    Invalid,
}
fn map_error(error: Error) -> StreamingBodyError {
    match error {
        Error::QueueFull => StreamingBodyError::BodyTooLarge,
        Error::Invalid => StreamingBodyError::InvalidBody,
    }
}

struct ByteQueue {
    bytes: [u8; QUEUE_BYTES],
    length: usize,
}
impl ByteQueue {
    const fn new() -> Self {
        Self {
            bytes: [0; QUEUE_BYTES],
            length: 0,
        }
    }
    fn push(&mut self, byte: u8) -> Result<(), Error> {
        if self.length == QUEUE_BYTES {
            return Err(Error::QueueFull);
        }
        self.bytes[self.length] = byte;
        self.length += 1;
        Ok(())
    }
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        if self.length < N {
            return None;
        }
        let mut out = [0; N];
        out.copy_from_slice(&self.bytes[..N]);
        self.bytes.copy_within(N..self.length, 0);
        self.length -= N;
        Some(out)
    }
    fn take_into(&mut self, length: usize, out: &mut [u8]) -> bool {
        if length > out.len() || self.length < length {
            return false;
        }
        out[..length].copy_from_slice(&self.bytes[..length]);
        self.bytes.copy_within(length..self.length, 0);
        self.length -= length;
        true
    }
    const fn is_empty(&self) -> bool {
        self.length == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_fragmented_revocation_body() {
        let developer = BoundedText::new("developer").expect("developer fits");
        let reason = BoundedText::new("compromised").expect("reason fits");
        let record = RevocationRecord {
            developer_id: developer,
            effective_version: 2,
            issuer_key_id: KeyId([1; crate::KEY_ID_LENGTH]),
            key_id: KeyId([2; crate::KEY_ID_LENGTH]),
            reason,
        };
        let metadata = RevocationMetadata {
            header: MetadataHeader {
                role: MetadataRole::Revocation,
                version: 1,
                expires: 0,
            },
            records: [record; crate::MAX_REVOCATIONS],
            record_count: 1,
        };
        let mut body = [0; crate::MAX_REVOCATION_BYTES];
        let length =
            crate::encode_binary_revocation_body(metadata, &mut body).expect("revocation encodes");
        let mut parser = BinaryRevocationBodyStreamParser::<{ crate::MAX_REVOCATIONS }>::new();
        for chunk in body[..length].chunks(4) {
            parser.feed(chunk).expect("fragment parses")
        }
        let parsed = parser.finish().expect("revocation finishes");
        assert_eq!(parsed.record_count, 1);
        assert_eq!(parsed.records[0], record);
    }
}
