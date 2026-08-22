//! DALI-CMT.BIN record types and wire-format boundaries.

use super::CommitJournalState;

mod codec;
mod recovery;
#[cfg(test)]
mod tests;

pub use recovery::recover;

const MAGIC: [u8; 4] = *b"DLCT";
const FORMAT_VERSION: u8 = 1;
const ACTIVE_SLOT_A: u8 = 0;
const ACTIVE_SLOT_B: u8 = 1;
const STATE_OFFSET: usize = 0x05;
const SLOT_OFFSET: usize = 0x06;
const SEQUENCE_OFFSET: usize = 0x08;
const BUNDLE_VERSION_OFFSET: usize = 0x10;
const BUNDLE_LENGTH_OFFSET: usize = 0x18;
const BUNDLE_DIGEST_OFFSET: usize = 0x20;
const CRC_OFFSET: usize = 0x40;
const RESERVED_OFFSET: usize = 0x44;

/// Maximum candidate size accepted by the durable repository contract.
pub const MAX_BUNDLE_BYTES: u32 = 128 * 1024;

/// Logical slot selected by a valid commit journal record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalSlot {
    /// First durable payload slot.
    A,
    /// Second durable payload slot.
    B,
}

impl JournalSlot {
    fn encode(self) -> u8 {
        match self {
            Self::A => ACTIVE_SLOT_A,
            Self::B => ACTIVE_SLOT_B,
        }
    }

    fn decode(value: u8) -> Result<Self, JournalError> {
        match value {
            ACTIVE_SLOT_A => Ok(Self::A),
            ACTIVE_SLOT_B => Ok(Self::B),
            _ => Err(JournalError::InvalidSlot),
        }
    }
}

/// Result of examining both fixed journal records after a reboot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryDecision {
    /// A committed generation is the durable active state.
    Committed(CommitJournalRecord),
    /// No committed record survived; any prepared state must be discarded.
    DiscardPrepared,
}

/// One fixed-size DALI-CMT.BIN record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitJournalRecord {
    /// Prepared or committed transition state.
    pub state: CommitJournalState,
    /// Payload slot referenced by the record.
    pub active_slot: JournalSlot,
    /// Monotonically increasing journal sequence.
    pub sequence: u64,
    /// Repository bundle generation.
    pub bundle_version: u64,
    /// Exact bundle length in bytes.
    pub bundle_length: u32,
    /// SHA-256 digest supplied by the authenticated repository metadata.
    pub bundle_digest: [u8; 32],
}

/// Errors returned by the bounded journal codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalError {
    /// The output/input length is not exactly one record.
    InvalidLength,
    /// The record magic is not `DLCT`.
    InvalidMagic,
    /// The record format version is unsupported.
    UnsupportedVersion,
    /// The state byte is not defined by the frozen contract.
    InvalidState,
    /// The active slot byte is invalid.
    InvalidSlot,
    /// Reserved bytes are not zero.
    NonZeroReserved,
    /// The bundle length exceeds the bounded contract.
    InvalidBundleLength,
    /// The journal sequence is zero and cannot establish ordering.
    InvalidSequence,
    /// The bundle version is zero and cannot establish anti-rollback state.
    InvalidBundleVersion,
    /// The bundle digest is empty and cannot identify authenticated contents.
    InvalidBundleDigest,
    /// The stored CRC32 does not match the record body.
    InvalidCrc,
    /// Neither journal record contains a committed generation.
    NoCommittedRecord,
}
