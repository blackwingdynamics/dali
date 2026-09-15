//! Bounded, transport-neutral artifact installation frames.

/// Installation protocol revision.
pub const PROTOCOL_VERSION: u8 = 1;
/// Maximum payload carried by one installation frame.
pub const MAX_PAYLOAD: usize = 512;
/// Fixed frame header length in bytes.
pub const HEADER_SIZE: usize = 16;
/// CRC32 trailer length in bytes.
pub const CRC_SIZE: usize = 4;
/// Maximum encoded frame length.
pub const MAX_FRAME_SIZE: usize = HEADER_SIZE + MAX_PAYLOAD + CRC_SIZE;
/// Installation frame magic used to distinguish frames from log bytes.
pub const FRAME_MAGIC: [u8; 4] = *b"DINS";
const CRC_POLYNOMIAL: u32 = 0xEDB8_8320;

/// Commands accepted by the installation state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    /// Erases the target slot and starts a replacement of the declared size.
    Begin,
    /// Writes one contiguous artifact chunk.
    Data,
    /// Requests complete candidate validation before publication.
    Validate,
    /// Publishes the validated candidate.
    Commit,
    /// Invalidates the current candidate.
    Abort,
}

impl Command {
    fn encode(self) -> u8 {
        match self {
            Self::Begin => 1,
            Self::Data => 2,
            Self::Validate => 3,
            Self::Commit => 4,
            Self::Abort => 5,
        }
    }

    fn decode(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Begin),
            2 => Some(Self::Data),
            3 => Some(Self::Validate),
            4 => Some(Self::Commit),
            5 => Some(Self::Abort),
            _ => None,
        }
    }
}

/// One decoded installation frame borrowing the caller's input buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frame<'a> {
    /// Installation command.
    pub command: Command,
    /// Byte offset of a data chunk within the candidate artifact.
    pub offset: u32,
    /// Complete artifact length declared by a `Begin` frame.
    pub total_length: u32,
    /// Command payload, bounded by [`MAX_PAYLOAD`].
    pub payload: &'a [u8],
}

/// Errors returned by frame encoding and decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    /// The encoded frame is too short, too long, or has trailing bytes.
    InvalidLength,
    /// The frame magic or protocol revision is unsupported.
    InvalidHeader,
    /// The command byte is unknown.
    InvalidCommand,
    /// The payload length exceeds the bounded frame capacity.
    InvalidPayload,
    /// The frame CRC does not match its encoded bytes.
    InvalidChecksum,
    /// The destination buffer cannot hold the encoded frame.
    BufferTooSmall,
}

impl<'a> Frame<'a> {
    /// Decodes exactly one complete frame and verifies its CRC32 trailer.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        if bytes.len() < HEADER_SIZE + CRC_SIZE || bytes.len() > MAX_FRAME_SIZE {
            return Err(FrameError::InvalidLength);
        }
        if bytes[..FRAME_MAGIC.len()] != FRAME_MAGIC || bytes[4] != PROTOCOL_VERSION {
            return Err(FrameError::InvalidHeader);
        }
        let command = Command::decode(bytes[5]).ok_or(FrameError::InvalidCommand)?;
        let payload_length = read_u16(&bytes[6..8]) as usize;
        if payload_length > MAX_PAYLOAD || bytes.len() != HEADER_SIZE + payload_length + CRC_SIZE {
            return Err(FrameError::InvalidPayload);
        }
        let checksum_offset = HEADER_SIZE + payload_length;
        if crc32(&bytes[..checksum_offset]) != read_u32(&bytes[checksum_offset..]) {
            return Err(FrameError::InvalidChecksum);
        }
        Ok(Self {
            command,
            offset: read_u32(&bytes[8..12]),
            total_length: read_u32(&bytes[12..16]),
            payload: &bytes[HEADER_SIZE..checksum_offset],
        })
    }

    /// Encodes one frame into caller-owned storage and returns its length.
    pub fn encode_into(&self, destination: &mut [u8]) -> Result<usize, FrameError> {
        if self.payload.len() > MAX_PAYLOAD {
            return Err(FrameError::InvalidPayload);
        }
        let length = HEADER_SIZE + self.payload.len() + CRC_SIZE;
        if destination.len() < length {
            return Err(FrameError::BufferTooSmall);
        }
        destination[..FRAME_MAGIC.len()].copy_from_slice(&FRAME_MAGIC);
        destination[4] = PROTOCOL_VERSION;
        destination[5] = self.command.encode();
        destination[6..8].copy_from_slice(&(self.payload.len() as u16).to_le_bytes());
        destination[8..12].copy_from_slice(&self.offset.to_le_bytes());
        destination[12..16].copy_from_slice(&self.total_length.to_le_bytes());
        destination[HEADER_SIZE..HEADER_SIZE + self.payload.len()].copy_from_slice(self.payload);
        let checksum_offset = HEADER_SIZE + self.payload.len();
        let checksum = crc32(&destination[..checksum_offset]);
        destination[checksum_offset..length].copy_from_slice(&checksum.to_le_bytes());
        Ok(length)
    }
}

/// Lifecycle states for one bounded installation session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    /// No candidate is being received.
    Idle,
    /// Data chunks are expected at the declared sequential offset.
    Receiving { total_length: u32, next_offset: u32 },
    /// All bytes were received and validation was requested.
    Validated { total_length: u32 },
}

/// Events emitted after a valid frame advances the installation lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    /// The target slot may now be erased and a candidate received.
    Begin { total_length: u32 },
    /// The target may program this contiguous chunk.
    Data { offset: u32, payload_length: u16 },
    /// The target must run AMRN validation before accepting commit.
    Validate,
    /// The validated candidate may be published.
    Commit,
    /// The target must invalidate the candidate.
    Abort,
}

/// Protocol state machine independent from USB, Flash, and logging backends.
pub struct Session {
    state: State,
}

impl Session {
    /// Creates an idle installation session.
    pub const fn new() -> Self {
        Self { state: State::Idle }
    }

    /// Returns the current lifecycle state.
    pub const fn state(&self) -> State {
        self.state
    }

    /// Resets the session after a backend failure.
    pub fn reset(&mut self) {
        self.state = State::Idle;
    }

    /// Applies one decoded frame and emits the corresponding bounded action.
    pub fn accept(&mut self, frame: Frame<'_>) -> Result<Event, SessionError> {
        match (self.state, frame.command) {
            (State::Idle, Command::Begin) if valid_total(frame.total_length) => {
                self.state = State::Receiving {
                    total_length: frame.total_length,
                    next_offset: 0,
                };
                Ok(Event::Begin {
                    total_length: frame.total_length,
                })
            }
            (
                State::Receiving {
                    total_length,
                    next_offset,
                },
                Command::Data,
            ) if frame.offset == next_offset
                && !frame.payload.is_empty()
                && frame
                    .offset
                    .checked_add(frame.payload.len() as u32)
                    .is_some_and(|end| end <= total_length) =>
            {
                let payload_length = frame.payload.len() as u16;
                self.state = State::Receiving {
                    total_length,
                    next_offset: next_offset + payload_length as u32,
                };
                Ok(Event::Data {
                    offset: frame.offset,
                    payload_length,
                })
            }
            (
                State::Receiving {
                    total_length,
                    next_offset,
                },
                Command::Validate,
            ) if next_offset == total_length => {
                self.state = State::Validated { total_length };
                Ok(Event::Validate)
            }
            (State::Validated { .. }, Command::Commit) => {
                self.state = State::Idle;
                Ok(Event::Commit)
            }
            (_, Command::Abort) => {
                self.state = State::Idle;
                Ok(Event::Abort)
            }
            _ => Err(SessionError::InvalidTransition),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors returned when a valid frame violates the installation lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionError {
    /// The command is not valid in the current state or has an invalid range.
    InvalidTransition,
}

fn valid_total(length: u32) -> bool {
    length > 0
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut checksum = u32::MAX;
    for byte in bytes {
        checksum ^= u32::from(*byte);
        for _ in 0..8 {
            checksum = if checksum & 1 == 1 {
                (checksum >> 1) ^ CRC_POLYNOMIAL
            } else {
                checksum >> 1
            };
        }
    }
    !checksum
}

#[cfg(test)]
#[path = "installation_tests.rs"]
mod tests;
