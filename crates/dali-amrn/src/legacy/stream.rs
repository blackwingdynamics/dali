use super::{CRC32_INITIAL, CRC32_POLYNOMIAL, Header, ParseError, validate_header};

/// Metadata produced after a complete payload validation pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedPayload {
    /// The validated fixed header fields.
    pub header: Header,
    /// The calculated native entry address before the Thumb bit is applied.
    pub entry_address: u32,
}

/// Incrementally validates a payload against a decoded AMRN header.
pub struct PayloadValidator {
    header: Header,
    bytes_seen: usize,
    checksum: Crc32,
}

impl PayloadValidator {
    /// Creates a validator for a structurally valid AMRN header.
    pub fn new(header: Header) -> Result<Self, ParseError> {
        validate_header(header)?;
        Ok(Self {
            header,
            bytes_seen: 0,
            checksum: Crc32::new(),
        })
    }

    /// Adds the next contiguous payload chunk to the validation pass.
    pub fn update(&mut self, chunk: &[u8]) -> Result<(), ParseError> {
        let next_size = self
            .bytes_seen
            .checked_add(chunk.len())
            .ok_or(ParseError::PayloadOutsideCartridge)?;
        if next_size > self.header.payload_size as usize {
            return Err(ParseError::PayloadOutsideCartridge);
        }
        self.checksum.update(chunk);
        self.bytes_seen = next_size;
        Ok(())
    }

    /// Completes validation and returns safe-to-copy payload metadata.
    pub fn finish(self) -> Result<ValidatedPayload, ParseError> {
        let expected_size = self.header.payload_size as usize;
        if self.bytes_seen != expected_size {
            return Err(ParseError::PayloadOutsideCartridge);
        }
        if self.checksum.finish() != self.header.crc32 {
            return Err(ParseError::CrcMismatch);
        }
        let entry_address = self
            .header
            .load_address
            .checked_add(self.header.execution_offset)
            .ok_or(ParseError::EntryAddressOverflow)?;
        Ok(ValidatedPayload {
            header: self.header,
            entry_address,
        })
    }
}

/// Incremental CRC32 calculator shared by bounded cartridge readers.
pub struct Crc32(u32);

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Crc32 {
    /// Creates a calculator with the AMRN initial state.
    pub const fn new() -> Self {
        Self(CRC32_INITIAL)
    }

    /// Adds the next contiguous byte range.
    pub fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u32::from(*byte);
            for _ in 0..u8::BITS {
                self.0 = if self.0 & 1 == 1 {
                    (self.0 >> 1) ^ CRC32_POLYNOMIAL
                } else {
                    self.0 >> 1
                };
            }
        }
    }

    /// Returns the finalized checksum.
    pub fn finish(self) -> u32 {
        !self.0
    }
}

pub(crate) fn checksum(bytes: &[u8]) -> u32 {
    let mut checksum = Crc32::new();
    checksum.update(bytes);
    checksum.finish()
}

pub(crate) fn checksum_parts(first: &[u8], second: &[u8]) -> u32 {
    let mut checksum = Crc32::new();
    checksum.update(first);
    checksum.update(second);
    checksum.finish()
}
