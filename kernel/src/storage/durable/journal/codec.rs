//! Encoding, decoding, and field validation for one journal record.

use super::super::{COMMIT_JOURNAL_RECORD_BYTES, CommitJournalState};
use super::{
    BUNDLE_DIGEST_OFFSET, BUNDLE_LENGTH_OFFSET, BUNDLE_VERSION_OFFSET, CRC_OFFSET,
    CommitJournalRecord, FORMAT_VERSION, JournalError, JournalSlot, MAGIC, RESERVED_OFFSET,
    SEQUENCE_OFFSET, SLOT_OFFSET, STATE_OFFSET,
};

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

/// Performs the `validate_fields` operation for this subsystem.
fn validate_fields(record: &CommitJournalRecord) -> Result<(), JournalError> {
    if record.sequence == 0 {
        return Err(JournalError::InvalidSequence);
    }
    if record.bundle_version == 0 {
        return Err(JournalError::InvalidBundleVersion);
    }
    if record.bundle_length == 0 || record.bundle_length > super::MAX_BUNDLE_BYTES {
        return Err(JournalError::InvalidBundleLength);
    }
    if record.bundle_digest == [0; 32] {
        return Err(JournalError::InvalidBundleDigest);
    }
    Ok(())
}

/// Performs the `decode_state` operation for this subsystem.
fn decode_state(value: u8) -> Result<CommitJournalState, JournalError> {
    match value {
        1 => Ok(CommitJournalState::Prepared),
        2 => Ok(CommitJournalState::Committed),
        _ => Err(JournalError::InvalidState),
    }
}

/// Performs the `write_u32` operation for this subsystem.
fn write_u32(output: &mut [u8], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// Performs the `write_u64` operation for this subsystem.
fn write_u64(output: &mut [u8], offset: usize, value: u64) {
    output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

/// Performs the `read_u32` operation for this subsystem.
fn read_u32(input: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(input[offset..offset + 4].try_into().unwrap_or([0; 4]))
}

/// Performs the `read_u64` operation for this subsystem.
fn read_u64(input: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(input[offset..offset + 8].try_into().unwrap_or([0; 8]))
}

/// Performs the `crc32` operation for this subsystem.
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
