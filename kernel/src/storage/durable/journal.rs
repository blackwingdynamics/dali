//! DALI-CMT.BIN record codec.

use super::{COMMIT_JOURNAL_RECORD_BYTES, CommitJournalState};

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
    /// The stored CRC32 does not match the record body.
    InvalidCrc,
}

impl CommitJournalRecord {
    /// Encodes one record into the exact 104-byte wire representation.
    pub fn encode(&self, output: &mut [u8]) -> Result<(), JournalError> {
        if output.len() != COMMIT_JOURNAL_RECORD_BYTES {
            return Err(JournalError::InvalidLength);
        }
        validate_fields(self)?;
        output.fill(0);
        output[..MAGIC.len()].copy_from_slice(&MAGIC);
        output[0x04] = FORMAT_VERSION;
        output[STATE_OFFSET] = self.state as u8;
        output[SLOT_OFFSET] = self.active_slot.encode();
        write_u64(output, SEQUENCE_OFFSET, self.sequence);
        write_u64(output, BUNDLE_VERSION_OFFSET, self.bundle_version);
        write_u32(output, BUNDLE_LENGTH_OFFSET, self.bundle_length);
        output[BUNDLE_DIGEST_OFFSET..CRC_OFFSET].copy_from_slice(&self.bundle_digest);
        write_u32(output, CRC_OFFSET, crc32(&output[..CRC_OFFSET]));
        Ok(())
    }

    /// Decodes and validates one exact 104-byte wire record.
    pub fn decode(input: &[u8]) -> Result<Self, JournalError> {
        if input.len() != COMMIT_JOURNAL_RECORD_BYTES {
            return Err(JournalError::InvalidLength);
        }
        if input[..MAGIC.len()] != MAGIC {
            return Err(JournalError::InvalidMagic);
        }
        if input[0x04] != FORMAT_VERSION {
            return Err(JournalError::UnsupportedVersion);
        }
        if input[RESERVED_OFFSET..].iter().any(|byte| *byte != 0) {
            return Err(JournalError::NonZeroReserved);
        }
        if read_u32(input, CRC_OFFSET) != crc32(&input[..CRC_OFFSET]) {
            return Err(JournalError::InvalidCrc);
        }
        let record = Self {
            state: decode_state(input[STATE_OFFSET])?,
            active_slot: JournalSlot::decode(input[SLOT_OFFSET])?,
            sequence: read_u64(input, SEQUENCE_OFFSET),
            bundle_version: read_u64(input, BUNDLE_VERSION_OFFSET),
            bundle_length: read_u32(input, BUNDLE_LENGTH_OFFSET),
            bundle_digest: input[BUNDLE_DIGEST_OFFSET..CRC_OFFSET]
                .try_into()
                .map_err(|_| JournalError::InvalidLength)?,
        };
        validate_fields(&record)?;
        Ok(record)
    }
}

fn validate_fields(record: &CommitJournalRecord) -> Result<(), JournalError> {
    if record.bundle_length > MAX_BUNDLE_BYTES {
        return Err(JournalError::InvalidBundleLength);
    }
    Ok(())
}

fn decode_state(value: u8) -> Result<CommitJournalState, JournalError> {
    match value {
        1 => Ok(CommitJournalState::Prepared),
        2 => Ok(CommitJournalState::Committed),
        _ => Err(JournalError::InvalidState),
    }
}

fn write_u32(output: &mut [u8], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(output: &mut [u8], offset: usize, value: u64) {
    output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(input: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(input[offset..offset + 4].try_into().unwrap_or([0; 4]))
}

fn read_u64(input: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(input[offset..offset + 8].try_into().unwrap_or([0; 8]))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB88320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> CommitJournalRecord {
        CommitJournalRecord {
            state: CommitJournalState::Committed,
            active_slot: JournalSlot::B,
            sequence: 7,
            bundle_version: 3,
            bundle_length: 512,
            bundle_digest: [0xA5; 32],
        }
    }

    #[test]
    fn round_trips_frozen_record() {
        let mut bytes = [0; COMMIT_JOURNAL_RECORD_BYTES];
        record().encode(&mut bytes).expect("record encodes");
        assert_eq!(CommitJournalRecord::decode(&bytes), Ok(record()));
    }

    #[test]
    fn rejects_tampered_body_and_reserved_bytes() {
        let mut bytes = [0; COMMIT_JOURNAL_RECORD_BYTES];
        record().encode(&mut bytes).expect("record encodes");
        bytes[0x10] ^= 1;
        assert_eq!(
            CommitJournalRecord::decode(&bytes),
            Err(JournalError::InvalidCrc)
        );
        record().encode(&mut bytes).expect("record encodes");
        bytes[RESERVED_OFFSET] = 1;
        assert_eq!(
            CommitJournalRecord::decode(&bytes),
            Err(JournalError::NonZeroReserved)
        );
    }

    #[test]
    fn rejects_oversized_bundle() {
        let mut bytes = [0; COMMIT_JOURNAL_RECORD_BYTES];
        let mut invalid = record();
        invalid.bundle_length = MAX_BUNDLE_BYTES + 1;
        assert_eq!(
            invalid.encode(&mut bytes),
            Err(JournalError::InvalidBundleLength)
        );
    }
}
