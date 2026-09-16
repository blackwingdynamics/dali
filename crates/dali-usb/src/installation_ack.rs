//! Bounded acknowledgements for the DINS installation lifecycle.

use crate::installation::{Command, Frame, FrameError, MAX_FRAME_SIZE};

/// Number of bytes in an acknowledgement payload.
pub const ACK_PAYLOAD_SIZE: usize = 7;
/// Acknowledgement status accepted by the installation host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AckStatus {
    /// The command completed and the lifecycle may continue.
    Accepted,
    /// The command was rejected by the protocol or backend.
    Rejected,
    /// The target requires its main-context validation pass.
    ValidationRequired,
    /// The validated candidate is now published.
    Committed,
    /// The candidate was invalidated.
    Aborted,
}

impl AckStatus {
    const fn encode(self) -> u8 {
        match self {
            Self::Accepted => 1,
            Self::Rejected => 2,
            Self::ValidationRequired => 3,
            Self::Committed => 4,
            Self::Aborted => 5,
        }
    }

    fn decode(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Accepted),
            2 => Some(Self::Rejected),
            3 => Some(Self::ValidationRequired),
            4 => Some(Self::Committed),
            5 => Some(Self::Aborted),
            _ => None,
        }
    }
}

/// One bounded response to exactly one DINS command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Acknowledgement {
    /// Command completed or rejected by the target.
    pub command: Command,
    /// Result of processing the command.
    pub status: AckStatus,
    /// Optional bounded protocol/backend detail code.
    pub detail: u16,
    /// Next sequential artifact offset accepted by the target.
    pub next_offset: u32,
}

/// Errors returned while encoding or decoding acknowledgements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AckError {
    /// The underlying frame was malformed.
    Frame(FrameError),
    /// The frame payload is not the fixed acknowledgement size.
    InvalidPayload,
    /// The status byte is not part of the acknowledgement contract.
    InvalidStatus,
}

impl Acknowledgement {
    /// Encodes the acknowledgement using the existing DINS frame contract.
    pub fn encode_into(&self, destination: &mut [u8]) -> Result<usize, FrameError> {
        let payload = self.payload();
        Frame {
            command: self.command,
            offset: self.next_offset,
            total_length: 0,
            payload: &payload,
        }
        .encode_into(destination)
    }

    /// Decodes one complete acknowledgement and verifies its frame CRC.
    pub fn decode(bytes: &[u8]) -> Result<Self, AckError> {
        let frame = Frame::decode(bytes).map_err(AckError::Frame)?;
        if frame.payload.len() != ACK_PAYLOAD_SIZE {
            return Err(AckError::InvalidPayload);
        }
        let status = AckStatus::decode(frame.payload[0]).ok_or(AckError::InvalidStatus)?;
        Ok(Self {
            command: frame.command,
            status,
            detail: u16::from_le_bytes([frame.payload[1], frame.payload[2]]),
            next_offset: frame.offset,
        })
    }

    fn payload(self) -> [u8; ACK_PAYLOAD_SIZE] {
        let mut payload = [0; ACK_PAYLOAD_SIZE];
        payload[0] = self.status.encode();
        payload[1..3].copy_from_slice(&self.detail.to_le_bytes());
        payload[3..].copy_from_slice(&self.next_offset.to_le_bytes());
        payload
    }
}

/// Returns the largest buffer required for one acknowledgement frame.
pub const fn max_encoded_acknowledgement_size() -> usize {
    MAX_FRAME_SIZE
}

#[cfg(test)]
#[path = "installation_ack_tests.rs"]
mod tests;
