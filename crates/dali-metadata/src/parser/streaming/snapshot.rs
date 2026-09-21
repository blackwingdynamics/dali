//! Incremental Binary Metadata v2 snapshot-body parser.

use crate::{
    BoundedText, DelegationReference, MetadataHeader, MetadataRole, RevocationReference,
    Sha256Digest, SnapshotMetadata, TargetsReference, validate_snapshot_metadata,
};
use core::mem::MaybeUninit;

const QUEUE_BYTES: usize = 512;
const TEXT_BYTES: usize = crate::MAX_DELEGATION_ID_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    HeaderVersion,
    HeaderExpires,
    TargetsVersion,
    TargetsLength,
    TargetsDigest,
    RevocationsVersion,
    RevocationsLength,
    RevocationsDigest,
    DelegationCount,
    DelegationIdLength,
    DelegationId,
    DelegationVersion,
    DelegationLength,
    DelegationDigest,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Error {
    QueueFull,
    InvalidBody,
}

/// Incrementally parses a Binary v2 snapshot body without retaining the body.
pub struct BinarySnapshotBodyStreamParser<
    const CAPACITY: usize = { crate::MAX_SNAPSHOT_REFERENCES },
> {
    queue: ByteQueue,
    phase: Phase,
    version: u64,
    expires: u64,
    targets: TargetsReference,
    revocations: RevocationReference,
    delegations: [DelegationReference; CAPACITY],
    delegation_count: u8,
    delegation_index: usize,
    text_length: usize,
    text: [u8; TEXT_BYTES],
}

impl<const CAPACITY: usize> BinarySnapshotBodyStreamParser<CAPACITY> {
    /// Creates an empty snapshot body parser.
    pub const fn new() -> Self {
        Self {
            queue: ByteQueue::new(),
            phase: Phase::HeaderVersion,
            version: 0,
            expires: 0,
            targets: TargetsReference {
                version: 0,
                length: 0,
                sha256: Sha256Digest([0; crate::SHA256_LENGTH]),
            },
            revocations: RevocationReference {
                version: 0,
                length: 0,
                sha256: Sha256Digest([0; crate::SHA256_LENGTH]),
            },
            delegations: [DelegationReference::empty(); CAPACITY],
            delegation_count: 0,
            delegation_index: 0,
            text_length: 0,
            text: [0; TEXT_BYTES],
        }
    }

    /// Resets parser state in place without creating a stack-sized temporary.
    pub fn reset(&mut self) {
        self.queue.reset();
        self.phase = Phase::HeaderVersion;
        self.version = 0;
        self.expires = 0;
        self.targets = TargetsReference {
            version: 0,
            length: 0,
            sha256: Sha256Digest([0; crate::SHA256_LENGTH]),
        };
        self.revocations = RevocationReference {
            version: 0,
            length: 0,
            sha256: Sha256Digest([0; crate::SHA256_LENGTH]),
        };
        self.delegation_count = 0;
        self.delegation_index = 0;
        self.text_length = 0;
        self.text = [0; TEXT_BYTES];
    }

    /// Feeds transport fragments into the bounded state machine.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), crate::StreamingBodyError> {
        for byte in bytes {
            self.queue.push(*byte).map_err(map_error)?;
            self.drive().map_err(map_error)?;
        }
        self.drive().map_err(map_error)
    }

    /// Completes parsing and validates the typed snapshot contract.
    pub fn finish(&mut self) -> Result<SnapshotMetadata, crate::StreamingBodyError> {
        let mut output = MaybeUninit::uninit();
        self.finish_into(&mut output)?;
        // SAFETY: finish_into initializes output before returning Ok.
        Ok(unsafe { output.assume_init() })
    }

    /// Completes parsing directly into caller-owned output storage.
    pub fn finish_into(
        &mut self,
        output: &mut MaybeUninit<SnapshotMetadata>,
    ) -> Result<(), crate::StreamingBodyError> {
        self.drive().map_err(map_error)?;
        if self.phase != Phase::Complete || !self.queue.is_empty() {
            return Err(crate::StreamingBodyError::UnexpectedEnd);
        }
        let output_ptr = output.as_mut_ptr();
        // SAFETY: every field of the output is initialized before validation,
        // and the parser owns the source references for the duration of copy.
        unsafe {
            (*output_ptr).header = MetadataHeader {
                role: MetadataRole::Snapshot,
                version: self.version,
                expires: self.expires,
            };
            (*output_ptr).targets = self.targets;
            (*output_ptr).revocations = self.revocations;
            for index in 0..crate::MAX_SNAPSHOT_REFERENCES {
                (*output_ptr).delegations[index] = if index < usize::from(self.delegation_count) {
                    self.delegations[index]
                } else {
                    DelegationReference::empty()
                };
            }
            (*output_ptr).delegation_count = self.delegation_count;
            validate_snapshot_metadata(&*output_ptr)
                .map_err(|_| crate::StreamingBodyError::InvalidBody)?;
        }
        Ok(())
    }

    fn drive(&mut self) -> Result<(), Error> {
        loop {
            let progressed = match self.phase {
                Phase::HeaderVersion => self.assign_u64(0),
                Phase::HeaderExpires => self.assign_u64(1),
                Phase::TargetsVersion => self.assign_u64(2),
                Phase::TargetsLength => self.assign_u32(0),
                Phase::TargetsDigest => self.assign_digest(0),
                Phase::RevocationsVersion => self.assign_u64(3),
                Phase::RevocationsLength => self.assign_u32(1),
                Phase::RevocationsDigest => self.assign_digest(1),
                Phase::DelegationCount => match self.queue.take::<1>() {
                    None => Ok(false),
                    Some(value) => {
                        let value = value[0];
                        if usize::from(value) > CAPACITY {
                            return Err(Error::InvalidBody);
                        }
                        self.delegation_count = value;
                        self.phase = if value == 0 {
                            Phase::Complete
                        } else {
                            Phase::DelegationIdLength
                        };
                        Ok(true)
                    }
                },
                Phase::DelegationIdLength => match self.queue.take::<2>() {
                    None => Ok(false),
                    Some(value) => {
                        self.text_length = usize::from(u16::from_le_bytes(value));
                        if self.text_length > TEXT_BYTES {
                            return Err(Error::InvalidBody);
                        }
                        self.phase = Phase::DelegationId;
                        Ok(true)
                    }
                },
                Phase::DelegationId => {
                    if !self.queue.take_into(self.text_length, &mut self.text) {
                        Ok(false)
                    } else {
                        let id = core::str::from_utf8(&self.text[..self.text_length])
                            .ok()
                            .and_then(|value| BoundedText::new(value).ok())
                            .ok_or(Error::InvalidBody)?;
                        self.delegations[self.delegation_index].id = id;
                        self.phase = Phase::DelegationVersion;
                        Ok(true)
                    }
                }
                Phase::DelegationVersion => self.assign_u64(4),
                Phase::DelegationLength => self.assign_u32(2),
                Phase::DelegationDigest => self.assign_digest(2),
                Phase::Complete => Ok(false),
            }?;
            if !progressed {
                return Ok(());
            }
        }
    }

    fn assign_u32(&mut self, _field: u8) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<4>() else {
            return Ok(false);
        };
        let value = u32::from_le_bytes(value);
        match self.phase {
            Phase::TargetsLength => self.targets.length = value,
            Phase::RevocationsLength => self.revocations.length = value,
            Phase::DelegationLength => self.delegations[self.delegation_index].length = value,
            _ => return Err(Error::InvalidBody),
        }
        self.advance_fixed_phase();
        Ok(true)
    }

    fn assign_u64(&mut self, _field: u8) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<8>() else {
            return Ok(false);
        };
        let value = u64::from_le_bytes(value);
        match self.phase {
            Phase::HeaderVersion => self.version = value,
            Phase::HeaderExpires => self.expires = value,
            Phase::TargetsVersion => self.targets.version = value,
            Phase::RevocationsVersion => self.revocations.version = value,
            Phase::DelegationVersion => self.delegations[self.delegation_index].version = value,
            _ => return Err(Error::InvalidBody),
        }
        self.advance_fixed_phase();
        Ok(true)
    }

    fn assign_digest(&mut self, _field: u8) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<{ crate::SHA256_LENGTH }>() else {
            return Ok(false);
        };
        let value = Sha256Digest(value);
        match self.phase {
            Phase::TargetsDigest => self.targets.sha256 = value,
            Phase::RevocationsDigest => self.revocations.sha256 = value,
            Phase::DelegationDigest => self.delegations[self.delegation_index].sha256 = value,
            _ => return Err(Error::InvalidBody),
        }
        self.advance_fixed_phase();
        Ok(true)
    }

    fn advance_fixed_phase(&mut self) {
        self.phase = match self.phase {
            Phase::HeaderVersion => Phase::HeaderExpires,
            Phase::HeaderExpires => Phase::TargetsVersion,
            Phase::TargetsVersion => Phase::TargetsLength,
            Phase::TargetsLength => Phase::TargetsDigest,
            Phase::TargetsDigest => Phase::RevocationsVersion,
            Phase::RevocationsVersion => Phase::RevocationsLength,
            Phase::RevocationsLength => Phase::RevocationsDigest,
            Phase::RevocationsDigest => Phase::DelegationCount,
            Phase::DelegationVersion => Phase::DelegationLength,
            Phase::DelegationLength => Phase::DelegationDigest,
            Phase::DelegationDigest => {
                self.delegation_index += 1;
                if self.delegation_index == usize::from(self.delegation_count) {
                    Phase::Complete
                } else {
                    Phase::DelegationIdLength
                }
            }
            phase => phase,
        };
    }
}

impl<const CAPACITY: usize> Default for BinarySnapshotBodyStreamParser<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

fn map_error(error: Error) -> crate::StreamingBodyError {
    match error {
        Error::QueueFull => crate::StreamingBodyError::BodyTooLarge,
        Error::InvalidBody => crate::StreamingBodyError::InvalidBody,
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

    fn reset(&mut self) {
        self.length = 0;
    }

    fn push(&mut self, byte: u8) -> Result<(), Error> {
        if self.length == self.bytes.len() {
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
        let mut output = [0; N];
        output.copy_from_slice(&self.bytes[..N]);
        self.bytes.copy_within(N..self.length, 0);
        self.length -= N;
        Some(output)
    }

    fn take_into(&mut self, length: usize, output: &mut [u8]) -> bool {
        if length > output.len() || self.length < length {
            return false;
        }
        output[..length].copy_from_slice(&self.bytes[..length]);
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

    fn digest() -> Sha256Digest {
        Sha256Digest([7; crate::SHA256_LENGTH])
    }

    #[test]
    fn parses_a_fragmented_snapshot_without_retaining_the_body() {
        let snapshot = SnapshotMetadata {
            header: MetadataHeader {
                role: MetadataRole::Snapshot,
                version: 1,
                expires: 0,
            },
            targets: TargetsReference {
                version: 1,
                length: 64,
                sha256: digest(),
            },
            revocations: RevocationReference {
                version: 1,
                length: 64,
                sha256: digest(),
            },
            delegations: [DelegationReference {
                id: BoundedText::new("developer").expect("test text fits"),
                version: 1,
                length: 64,
                sha256: digest(),
            }; crate::MAX_SNAPSHOT_REFERENCES],
            delegation_count: 1,
        };
        let mut body = [0; crate::MAX_SNAPSHOT_BYTES];
        let length = crate::encode_binary_snapshot_body(snapshot, &mut body)
            .expect("snapshot body should encode");
        let mut parser =
            BinarySnapshotBodyStreamParser::<{ crate::MAX_SNAPSHOT_REFERENCES }>::new();
        for chunk in body[..length].chunks(3) {
            parser.feed(chunk).expect("fragment should parse");
        }
        let parsed = parser.finish().expect("snapshot should parse");
        assert_eq!(parsed.header, snapshot.header);
        assert_eq!(parsed.targets, snapshot.targets);
        assert_eq!(parsed.revocations, snapshot.revocations);
        assert_eq!(parsed.delegation_count, snapshot.delegation_count);
        assert_eq!(parsed.delegations[0], snapshot.delegations[0]);
    }
}
