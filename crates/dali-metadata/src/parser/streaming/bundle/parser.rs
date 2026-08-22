//! Incremental Binary Metadata v2 bundle-manifest summary parser.

use super::queue::ByteQueue;
use crate::{BoundedText, MetadataHeader, MetadataRole, StreamingBodyError};
use core::mem::MaybeUninit;
const REQUIRED_KINDS: u8 = 0x7F;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Version,
    Expires,
    TargetLength,
    Target,
    FileCount,
    FileKind,
    FileIdLength,
    FileId,
    FileLength,
    FileDigest,
    Complete,
}

/// The bounded bundle fields required by boot generation admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BundleManifestSummary {
    /// Signed bundle generation and expiry fields.
    pub header: MetadataHeader,
    /// Target profile covered by the signed manifest.
    pub target_profile: BoundedText<{ crate::MAX_TARGET_PROFILE_BYTES }>,
    /// Number of canonical file references in the manifest.
    pub file_count: u16,
}
/// Incrementally parses and validates a bundle body without retaining files.
pub struct BinaryBundleBodyStreamParser {
    queue: ByteQueue,
    phase: Phase,
    version: u64,
    expires: u64,
    target_profile: BoundedText<{ crate::MAX_TARGET_PROFILE_BYTES }>,
    file_count: u16,
    file_index: usize,
    current_kind: u8,
    last_kind: u8,
    last_id: [u8; crate::MAX_BUNDLE_ID_BYTES],
    last_id_length: usize,
    text_length: usize,
    text: [u8; crate::MAX_BUNDLE_ID_BYTES],
    required_kinds: u8,
}
impl BinaryBundleBodyStreamParser {
    /// Creates an empty bundle summary parser.
    pub fn new() -> Self {
        Self {
            queue: ByteQueue::new(),
            phase: Phase::Version,
            version: 0,
            expires: 0,
            target_profile: BoundedText::default(),
            file_count: 0,
            file_index: 0,
            current_kind: 0,
            last_kind: 0,
            last_id: [0; crate::MAX_BUNDLE_ID_BYTES],
            last_id_length: 0,
            text_length: 0,
            text: [0; crate::MAX_BUNDLE_ID_BYTES],
            required_kinds: 0,
        }
    }

    /// Feeds one bounded body fragment.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), StreamingBodyError> {
        for byte in bytes {
            self.queue.push(*byte).map_err(map_error)?;
            self.drive().map_err(map_error)?;
        }
        self.drive().map_err(map_error)
    }

    /// Completes the parser and returns the retained signed summary.
    pub fn finish(&mut self) -> Result<BundleManifestSummary, StreamingBodyError> {
        let mut output = MaybeUninit::uninit();
        self.finish_into(&mut output)?;
        // SAFETY: finish_into initializes output before returning Ok.
        Ok(unsafe { output.assume_init() })
    }

    /// Completes directly into caller-owned output storage.
    pub fn finish_into(
        &mut self,
        output: &mut MaybeUninit<BundleManifestSummary>,
    ) -> Result<(), StreamingBodyError> {
        self.drive().map_err(map_error)?;
        if self.phase != Phase::Complete
            || !self.queue.is_empty()
            || self.required_kinds != REQUIRED_KINDS
        {
            return Err(StreamingBodyError::UnexpectedEnd);
        }
        output.write(BundleManifestSummary {
            header: MetadataHeader {
                role: MetadataRole::Bundle,
                version: self.version,
                expires: self.expires,
            },
            target_profile: self.target_profile,
            file_count: self.file_count,
        });
        Ok(())
    }

    fn drive(&mut self) -> Result<(), ParserError> {
        loop {
            let progressed = match self.phase {
                Phase::Version => self.u64_field(|this, value| {
                    this.version = value;
                    this.phase = Phase::Expires;
                }),
                Phase::Expires => self.u64_field(|this, value| {
                    this.expires = value;
                    this.phase = Phase::TargetLength;
                }),
                Phase::TargetLength => self.text_length(false),
                Phase::Target => self.target(),
                Phase::FileCount => self.file_count(),
                Phase::FileKind => self.file_kind(),
                Phase::FileIdLength => self.text_length(true),
                Phase::FileId => self.file_id(),
                Phase::FileLength => self.file_length(),
                Phase::FileDigest => self.file_digest(),
                Phase::Complete => Ok(false),
            }?;
            if !progressed {
                return Ok(());
            }
        }
    }

    fn u64_field(&mut self, set: impl FnOnce(&mut Self, u64)) -> Result<bool, ParserError> {
        let Some(value) = self.queue.take::<8>() else {
            return Ok(false);
        };
        set(self, u64::from_le_bytes(value));
        Ok(true)
    }

    fn text_length(&mut self, file_id: bool) -> Result<bool, ParserError> {
        let Some(value) = self.queue.take::<2>() else {
            return Ok(false);
        };
        self.text_length = usize::from(u16::from_le_bytes(value));
        if self.text_length
            > if file_id {
                crate::MAX_BUNDLE_ID_BYTES
            } else {
                crate::MAX_TARGET_PROFILE_BYTES
            }
        {
            return Err(ParserError::Invalid);
        }
        self.phase = if file_id {
            Phase::FileId
        } else {
            Phase::Target
        };
        Ok(true)
    }

    fn target(&mut self) -> Result<bool, ParserError> {
        if !self.queue.take_into(self.text_length, &mut self.text) {
            return Ok(false);
        }
        let value = core::str::from_utf8(&self.text[..self.text_length])
            .ok()
            .and_then(|value| BoundedText::new(value).ok())
            .ok_or(ParserError::Invalid)?;
        crate::validation::validate_target_profile(value.as_str().ok_or(ParserError::Invalid)?)
            .map_err(|_| ParserError::Invalid)?;
        self.target_profile = value;
        self.phase = Phase::FileCount;
        Ok(true)
    }

    fn file_count(&mut self) -> Result<bool, ParserError> {
        let Some(value) = self.queue.take::<2>() else {
            return Ok(false);
        };
        self.file_count = u16::from_le_bytes(value);
        if self.file_count == 0 || usize::from(self.file_count) > crate::MAX_BUNDLE_FILES {
            return Err(ParserError::Invalid);
        }
        self.phase = Phase::FileKind;
        Ok(true)
    }

    fn file_kind(&mut self) -> Result<bool, ParserError> {
        let Some(value) = self.queue.take::<1>() else {
            return Ok(false);
        };
        let order = kind_order(value[0]).ok_or(ParserError::Invalid)?;
        if order < self.last_kind {
            return Err(ParserError::Invalid);
        }
        self.current_kind = value[0];
        self.required_kinds |= 1 << (value[0] - 1);
        self.phase = Phase::FileIdLength;
        Ok(true)
    }

    fn file_id(&mut self) -> Result<bool, ParserError> {
        if !self.queue.take_into(self.text_length, &mut self.text) {
            return Ok(false);
        }
        if self.text_length == 0
            || (kind_order(self.current_kind) == Some(self.last_kind)
                && self.text[..self.text_length] <= self.last_id[..self.last_id_length])
        {
            return Err(ParserError::Invalid);
        }
        if let Some(expected) = expected_id(self.current_kind)
            && &self.text[..self.text_length] != expected
        {
            return Err(ParserError::Invalid);
        }
        self.last_kind = kind_order(self.current_kind).ok_or(ParserError::Invalid)?;
        self.last_id_length = self.text_length;
        self.last_id[..self.text_length].copy_from_slice(&self.text[..self.text_length]);
        self.phase = Phase::FileLength;
        Ok(true)
    }

    fn file_length(&mut self) -> Result<bool, ParserError> {
        let Some(value) = self.queue.take::<4>() else {
            return Ok(false);
        };
        if u32::from_le_bytes(value) == 0 {
            return Err(ParserError::Invalid);
        }
        self.phase = Phase::FileDigest;
        Ok(true)
    }

    fn file_digest(&mut self) -> Result<bool, ParserError> {
        let Some(value) = self.queue.take::<{ crate::SHA256_LENGTH }>() else {
            return Ok(false);
        };
        if value == [0; crate::SHA256_LENGTH] {
            return Err(ParserError::Invalid);
        }
        self.file_index += 1;
        self.phase = if self.file_index == usize::from(self.file_count) {
            Phase::Complete
        } else {
            Phase::FileKind
        };
        Ok(true)
    }
}
impl Default for BinaryBundleBodyStreamParser {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ParserError {
    QueueFull,
    Invalid,
}

fn map_error(error: ParserError) -> StreamingBodyError {
    match error {
        ParserError::QueueFull => StreamingBodyError::BodyTooLarge,
        ParserError::Invalid => StreamingBodyError::InvalidBody,
    }
}

fn expected_id(kind: u8) -> Option<&'static [u8]> {
    match kind {
        1 => Some(b"root"),
        2 => Some(b"timestamp"),
        3 => Some(b"snapshot"),
        4 => Some(b"targets"),
        6 => Some(b"revocation"),
        _ => None,
    }
}

fn kind_order(kind: u8) -> Option<u8> {
    match kind {
        1 => Some(0),
        2 => Some(1),
        3 => Some(2),
        4 => Some(3),
        6 => Some(4),
        5 => Some(5),
        7 => Some(6),
        _ => None,
    }
}
