//! Bounded receive-side assembly and queuing for installation frames.

use crate::installation::{CRC_SIZE, FRAME_MAGIC, HEADER_SIZE, MAX_FRAME_SIZE, MAX_PAYLOAD};

/// Maximum number of complete frames retained by one receive queue.
pub const DEFAULT_QUEUE_CAPACITY: usize = 2;

/// Result of accepting one transport byte slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceiveReport {
    /// Number of complete frames queued by this call.
    pub queued: usize,
    /// Number of complete frames dropped because the queue was full.
    pub dropped: usize,
    /// Whether malformed framing was discarded during this call.
    pub malformed: bool,
}

/// Fixed-memory receive assembler and FIFO for complete installation frames.
pub struct InstallationReceiveQueue<const QUEUE_CAPACITY: usize> {
    frames: [[u8; MAX_FRAME_SIZE]; QUEUE_CAPACITY],
    lengths: [usize; QUEUE_CAPACITY],
    head: usize,
    count: usize,
    assembling: [u8; MAX_FRAME_SIZE],
    assembling_length: usize,
    expected_length: Option<usize>,
    dropped: u32,
}

impl<const QUEUE_CAPACITY: usize> InstallationReceiveQueue<QUEUE_CAPACITY> {
    /// Creates an empty queue with a compile-time bounded frame capacity.
    pub const fn new() -> Self {
        assert!(QUEUE_CAPACITY > 0);
        Self {
            frames: [[0; MAX_FRAME_SIZE]; QUEUE_CAPACITY],
            lengths: [0; QUEUE_CAPACITY],
            head: 0,
            count: 0,
            assembling: [0; MAX_FRAME_SIZE],
            assembling_length: 0,
            expected_length: None,
            dropped: 0,
        }
    }

    /// Accepts arbitrary transport chunking and queues complete frames.
    pub fn push_bytes(&mut self, bytes: &[u8]) -> ReceiveReport {
        let mut report = ReceiveReport {
            queued: 0,
            dropped: 0,
            malformed: false,
        };
        for byte in bytes {
            if self.assembling_length == 0 && *byte != FRAME_MAGIC[0] {
                continue;
            }
            if self.assembling_length == MAX_FRAME_SIZE {
                self.reset_assembly();
                report.malformed = true;
            }
            self.assembling[self.assembling_length] = *byte;
            self.assembling_length += 1;
            if self.assembling_length == HEADER_SIZE {
                if !self.valid_header() {
                    self.reset_assembly();
                    report.malformed = true;
                    continue;
                }
                let payload_length =
                    u16::from_le_bytes([self.assembling[6], self.assembling[7]]) as usize;
                self.expected_length = Some(HEADER_SIZE + payload_length + CRC_SIZE);
            }
            if self.expected_length == Some(self.assembling_length) {
                if self.enqueue_current() {
                    report.queued += 1;
                } else {
                    report.dropped += 1;
                }
                self.reset_assembly();
            }
        }
        report
    }

    /// Copies and removes the oldest complete frame into caller storage.
    pub fn pop_frame(&mut self, destination: &mut [u8; MAX_FRAME_SIZE]) -> Option<usize> {
        if self.count == 0 {
            return None;
        }
        let length = self.lengths[self.head];
        destination[..length].copy_from_slice(&self.frames[self.head][..length]);
        self.head = (self.head + 1) % QUEUE_CAPACITY;
        self.count -= 1;
        Some(length)
    }

    /// Returns and clears the cumulative overflow count.
    pub fn take_dropped(&mut self) -> u32 {
        let dropped = self.dropped;
        self.dropped = 0;
        dropped
    }

    fn valid_header(&self) -> bool {
        self.assembling[..FRAME_MAGIC.len()] == FRAME_MAGIC
            && self.assembling[4] == crate::installation::PROTOCOL_VERSION
            && (u16::from_le_bytes([self.assembling[6], self.assembling[7]]) as usize)
                <= MAX_PAYLOAD
    }

    fn enqueue_current(&mut self) -> bool {
        if self.count == QUEUE_CAPACITY {
            self.dropped = self.dropped.saturating_add(1);
            return false;
        }
        let index = (self.head + self.count) % QUEUE_CAPACITY;
        self.frames[index][..self.assembling_length]
            .copy_from_slice(&self.assembling[..self.assembling_length]);
        self.lengths[index] = self.assembling_length;
        self.count += 1;
        true
    }

    fn reset_assembly(&mut self) {
        self.assembling_length = 0;
        self.expected_length = None;
    }
}

impl<const QUEUE_CAPACITY: usize> Default for InstallationReceiveQueue<QUEUE_CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "installation_queue_tests.rs"]
mod tests;
