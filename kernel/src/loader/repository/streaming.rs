//! Bounded Binary Metadata v2 target selection for repository streams.

use dali_metadata::{
    BINARY_ENVELOPE_HEADER_BYTES, BINARY_FORMAT_VERSION, BINARY_MAGIC, BinaryEnvelopeStreamParser,
    DecodeError, MetadataRole, PackageId, RoleDefinition, RoleKey, StreamingRoleVerifier,
    TargetPackage, parse_binary_target_record,
};

use crate::storage::repository::{RepositoryDocument, RepositoryStreamStorage};

/// Maximum target record retained while selecting one package.
pub const MAX_STREAMING_TARGET_RECORD_BYTES: usize = 512;
/// Minimum caller-owned chunk size recommended for F405 repository reads.
pub const STREAMING_METADATA_CHUNK_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    MetadataHeader,
    DelegationCount,
    DelegationLength,
    DelegationBytes,
    PackageCount,
    RecordLength,
    RecordBytes,
    Complete,
}

/// Errors produced while scanning one Binary Metadata v2 targets stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingTargetsError {
    /// The envelope header or role was invalid.
    InvalidEnvelope,
    /// A bounded field or record was truncated.
    UnexpectedEnd,
    /// A target record exceeded the bounded selection buffer.
    RecordTooLarge,
    /// The selected target record failed canonical decoding.
    InvalidRecord,
    /// The stream contained bytes outside its declared envelope.
    TrailingBytes,
}

/// Error boundary for selecting a target through a repository stream.
#[derive(Debug)]
pub enum StreamingTargetSelectionError<E> {
    /// The storage adapter failed while producing a chunk.
    Storage(E),
    /// The Binary Metadata v2 stream was malformed.
    Parse(StreamingTargetsError),
    /// The targets envelope failed its second-pass signature verification.
    Verification,
}

/// Selected target plus the authenticated serialized Targets document shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedBinaryTarget {
    /// Package authorization record selected by package identity.
    pub target: TargetPackage,
    /// Exact serialized Targets document length.
    pub length: u32,
    /// SHA-256 digest of the complete serialized Targets document.
    pub digest: dali_metadata::Sha256Digest,
    /// Signed body metadata retained for chain diagnostics.
    pub envelope: dali_metadata::StreamedEnvelope,
    /// Metadata version declared by the Targets body.
    pub version: u64,
}

/// Selects one target and authenticates the complete Binary v2 targets envelope.
///
/// The first pass parses only the selected record. The second pass replays the
/// exact envelope body through the bounded Ed25519/SHA-256 verifier.
pub fn select_verified_binary_target<S>(
    storage: &mut S,
    package_id: PackageId,
    role: RoleDefinition,
    keys: &[RoleKey],
    chunk: &mut [u8],
) -> Result<Option<VerifiedBinaryTarget>, StreamingTargetSelectionError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let mut selector = BinaryTargetsStreamParser::new(package_id);
    let mut envelope = BinaryEnvelopeStreamParser::new(MetadataRole::Targets);
    let mut digest = dali_crypto::Sha256Accumulator::new();
    let mut parse_error = None;
    let length = storage
        .stream_metadata(RepositoryDocument::Targets, chunk, |bytes| {
            digest.update(bytes);
            if parse_error.is_some() {
                return Ok(());
            }
            if let Err(error) = envelope.feed(bytes, |body| {
                selector.feed(body).map_err(|_| DecodeError::InvalidValue)
            }) {
                parse_error = Some(error);
            }
            Ok(())
        })
        .map_err(StreamingTargetSelectionError::Storage)?;
    if parse_error.is_some() {
        return Err(StreamingTargetSelectionError::Parse(
            StreamingTargetsError::InvalidEnvelope,
        ));
    }
    let envelope = envelope.finish().map_err(|_| {
        StreamingTargetSelectionError::Parse(StreamingTargetsError::InvalidEnvelope)
    })?;
    let version = selector.version;
    let target = selector
        .finish()
        .map_err(StreamingTargetSelectionError::Parse)?;
    let document_digest = dali_metadata::Sha256Digest(digest.finalize());
    let mut verifier = StreamingRoleVerifier::new(role, keys, envelope.signatures)
        .map_err(|_| StreamingTargetSelectionError::Verification)?;
    let mut replay = BinaryEnvelopeStreamParser::new(MetadataRole::Targets);
    let mut replay_digest = dali_crypto::Sha256Accumulator::new();
    let mut replay_error = false;
    storage
        .stream_metadata(RepositoryDocument::Targets, chunk, |bytes| {
            if replay_error {
                return Ok(());
            }
            replay_digest.update(bytes);
            if replay
                .feed(bytes, |body| {
                    verifier.update(body);
                    Ok::<(), DecodeError>(())
                })
                .is_err()
            {
                replay_error = true;
            }
            Ok(())
        })
        .map_err(StreamingTargetSelectionError::Storage)?;
    let replayed = replay
        .finish()
        .map_err(|_| StreamingTargetSelectionError::Verification)?;
    if replay_error
        || replayed.body_sha256 != envelope.body_sha256
        || dali_metadata::Sha256Digest(replay_digest.finalize()) != document_digest
        || verifier.finish().is_err()
    {
        return Err(StreamingTargetSelectionError::Verification);
    }
    Ok(target.map(|target| VerifiedBinaryTarget {
        target,
        length,
        digest: document_digest,
        envelope,
        version,
    }))
}

/// Selects one target record without retaining the complete targets document.
pub fn select_binary_target<S>(
    storage: &mut S,
    package_id: PackageId,
    chunk: &mut [u8],
) -> Result<Option<TargetPackage>, StreamingTargetSelectionError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let mut parser = BinaryTargetsStreamParser::new(package_id);
    let mut parse_error = None;
    storage
        .stream_metadata(RepositoryDocument::Targets, chunk, |bytes| {
            if parse_error.is_none()
                && let Err(error) = parser.feed(bytes)
            {
                parse_error = Some(error);
            }
            Ok(())
        })
        .map_err(StreamingTargetSelectionError::Storage)?;
    if let Some(error) = parse_error {
        return Err(StreamingTargetSelectionError::Parse(error));
    }
    parser
        .finish()
        .map_err(StreamingTargetSelectionError::Parse)
}

struct BinaryTargetsStreamParser {
    wanted: PackageId,
    envelope: [u8; BINARY_ENVELOPE_HEADER_BYTES],
    envelope_length: usize,
    body_length: usize,
    body_seen: usize,
    total_length: usize,
    total_seen: usize,
    phase: Phase,
    version: u64,
    field: [u8; 16],
    field_length: usize,
    field_need: usize,
    delegations_left: u16,
    delegation_bytes_left: usize,
    packages_left: u32,
    record_length: usize,
    record_seen: usize,
    candidate_id: [u8; 16],
    candidate_length: usize,
    selected: bool,
    record: [u8; MAX_STREAMING_TARGET_RECORD_BYTES],
    record_buffered: usize,
    selected_target: Option<TargetPackage>,
}

impl BinaryTargetsStreamParser {
    fn new(wanted: PackageId) -> Self {
        Self {
            wanted,
            envelope: [0; BINARY_ENVELOPE_HEADER_BYTES],
            envelope_length: 0,
            body_length: 0,
            body_seen: 0,
            total_length: 0,
            total_seen: 0,
            phase: Phase::MetadataHeader,
            version: 0,
            field: [0; 16],
            field_length: 0,
            field_need: 16,
            delegations_left: 0,
            delegation_bytes_left: 0,
            packages_left: 0,
            record_length: 0,
            record_seen: 0,
            candidate_id: [0; 16],
            candidate_length: 0,
            selected: false,
            record: [0; MAX_STREAMING_TARGET_RECORD_BYTES],
            record_buffered: 0,
            selected_target: None,
        }
    }

    fn feed(&mut self, bytes: &[u8]) -> Result<(), StreamingTargetsError> {
        for byte in bytes {
            self.total_seen = self
                .total_seen
                .checked_add(1)
                .ok_or(StreamingTargetsError::TrailingBytes)?;
            if self.total_seen > self.total_length && self.total_length != 0 {
                return Err(StreamingTargetsError::TrailingBytes);
            }
            if self.envelope_length < BINARY_ENVELOPE_HEADER_BYTES {
                self.envelope[self.envelope_length] = *byte;
                self.envelope_length += 1;
                if self.envelope_length == BINARY_ENVELOPE_HEADER_BYTES {
                    self.finish_envelope_header()?;
                }
            } else if self.body_seen < self.body_length {
                self.body_seen += 1;
                self.consume_body_byte(*byte)?;
            }
        }
        Ok(())
    }

    fn finish_envelope_header(&mut self) -> Result<(), StreamingTargetsError> {
        if self.envelope[..4] != BINARY_MAGIC
            || self.envelope[4] != BINARY_FORMAT_VERSION
            || self.envelope[5] != role_number(MetadataRole::Targets)
            || self.envelope[6] != 0
            || self.envelope[7] != 0
            || self.envelope[13..16].iter().any(|byte| *byte != 0)
        {
            return Err(StreamingTargetsError::InvalidEnvelope);
        }
        let signatures = usize::from(self.envelope[12]);
        if signatures == 0 || signatures > dali_metadata::MAX_SIGNATURES {
            return Err(StreamingTargetsError::InvalidEnvelope);
        }
        self.body_length = read_u32(&self.envelope[8..12]) as usize;
        self.total_length = BINARY_ENVELOPE_HEADER_BYTES
            .checked_add(self.body_length)
            .and_then(|length| {
                length.checked_add(signatures * dali_metadata::BINARY_SIGNATURE_RECORD_BYTES)
            })
            .ok_or(StreamingTargetsError::InvalidEnvelope)?;
        Ok(())
    }

    fn consume_body_byte(&mut self, byte: u8) -> Result<(), StreamingTargetsError> {
        match self.phase {
            Phase::MetadataHeader => {
                self.read_field(byte, Phase::MetadataHeader);
                if self.field_length == self.field_need {
                    self.version = read_u64(&self.field[..8]);
                    self.reset_field(2, Phase::DelegationCount);
                }
                Ok(())
            }
            Phase::DelegationCount => {
                self.read_field(byte, Phase::DelegationCount);
                if self.field_length == self.field_need {
                    self.delegations_left = read_u16(&self.field[..2]);
                    self.reset_field(
                        2,
                        if self.delegations_left == 0 {
                            Phase::PackageCount
                        } else {
                            Phase::DelegationLength
                        },
                    );
                }
                Ok(())
            }
            Phase::DelegationLength => {
                self.read_field(byte, Phase::DelegationLength);
                if self.field_length == self.field_need {
                    self.delegation_bytes_left = usize::from(read_u16(&self.field[..2]));
                    self.delegations_left -= 1;
                    if self.delegation_bytes_left == 0 {
                        self.reset_field(
                            2,
                            if self.delegations_left == 0 {
                                Phase::PackageCount
                            } else {
                                Phase::DelegationLength
                            },
                        );
                    } else {
                        self.phase = Phase::DelegationBytes;
                        self.field_length = 0;
                    }
                }
                Ok(())
            }
            Phase::DelegationBytes => {
                self.delegation_bytes_left -= 1;
                if self.delegation_bytes_left == 0 {
                    self.reset_field(
                        2,
                        if self.delegations_left == 0 {
                            Phase::PackageCount
                        } else {
                            Phase::DelegationLength
                        },
                    );
                }
                Ok(())
            }
            Phase::PackageCount => {
                self.read_field(byte, Phase::PackageCount);
                if self.field_length == self.field_need {
                    self.packages_left = read_u32(&self.field[..4]);
                    self.phase = if self.packages_left == 0 {
                        Phase::Complete
                    } else {
                        Phase::RecordLength
                    };
                    self.field_length = 0;
                }
                Ok(())
            }
            Phase::RecordLength => {
                self.read_field(byte, Phase::RecordLength);
                if self.field_length == self.field_need {
                    self.record_length = usize::from(read_u16(&self.field[..2]));
                    self.record_seen = 0;
                    self.candidate_length = 0;
                    self.record_buffered = 0;
                    self.selected = false;
                    self.phase = Phase::RecordBytes;
                    self.field_length = 0;
                }
                Ok(())
            }
            Phase::RecordBytes => {
                self.consume_record_byte(byte)?;
                if self.record_seen == self.record_length {
                    self.finish_record()?;
                }
                Ok(())
            }
            Phase::Complete => Err(StreamingTargetsError::TrailingBytes),
        }
    }

    fn read_field(&mut self, byte: u8, _next: Phase) {
        self.field[self.field_length] = byte;
        self.field_length += 1;
    }

    fn reset_field(&mut self, need: usize, phase: Phase) {
        self.field_length = 0;
        self.field_need = need;
        self.phase = phase;
    }

    fn consume_record_byte(&mut self, byte: u8) -> Result<(), StreamingTargetsError> {
        if self.candidate_length < self.candidate_id.len() {
            self.candidate_id[self.candidate_length] = byte;
            self.candidate_length += 1;
            if self.candidate_length == self.candidate_id.len() {
                self.selected = self.candidate_id == self.wanted.0;
                if self.selected {
                    self.record[..self.candidate_id.len()].copy_from_slice(&self.candidate_id);
                    self.record_buffered = self.candidate_id.len();
                }
            }
        } else if self.selected {
            if self.record_buffered == self.record.len() {
                return Err(StreamingTargetsError::RecordTooLarge);
            }
            self.record[self.record_buffered] = byte;
            self.record_buffered += 1;
        }
        self.record_seen += 1;
        Ok(())
    }

    fn finish_record(&mut self) -> Result<(), StreamingTargetsError> {
        if self.selected {
            let target = parse_binary_target_record(&self.record[..self.record_buffered])
                .map_err(|_| StreamingTargetsError::InvalidRecord)?;
            self.selected_target = Some(target);
        }
        self.packages_left -= 1;
        self.phase = if self.packages_left == 0 {
            Phase::Complete
        } else {
            Phase::RecordLength
        };
        Ok(())
    }

    fn finish(self) -> Result<Option<TargetPackage>, StreamingTargetsError> {
        if self.envelope_length != BINARY_ENVELOPE_HEADER_BYTES
            || self.body_seen != self.body_length
            || self.total_seen != self.total_length
            || self.phase != Phase::Complete
        {
            return Err(StreamingTargetsError::UnexpectedEnd);
        }
        Ok(self.selected_target)
    }
}

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

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}
