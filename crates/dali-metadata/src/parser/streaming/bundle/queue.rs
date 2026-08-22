use super::parser::ParserError;

const QUEUE_BYTES: usize = 512;

pub(super) struct ByteQueue {
    bytes: [u8; QUEUE_BYTES],
    length: usize,
}

impl ByteQueue {
    pub(super) const fn new() -> Self {
        Self {
            bytes: [0; QUEUE_BYTES],
            length: 0,
        }
    }

    pub(super) fn push(&mut self, byte: u8) -> Result<(), ParserError> {
        if self.length == self.bytes.len() {
            return Err(ParserError::QueueFull);
        }
        self.bytes[self.length] = byte;
        self.length += 1;
        Ok(())
    }

    pub(super) fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        if self.length < N {
            return None;
        }
        let mut output = [0; N];
        output.copy_from_slice(&self.bytes[..N]);
        self.bytes.copy_within(N..self.length, 0);
        self.length -= N;
        Some(output)
    }

    pub(super) fn take_into(&mut self, length: usize, output: &mut [u8]) -> bool {
        if length > output.len() || self.length < length {
            return false;
        }
        output[..length].copy_from_slice(&self.bytes[..length]);
        self.bytes.copy_within(length..self.length, 0);
        self.length -= length;
        true
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.length == 0
    }
}
