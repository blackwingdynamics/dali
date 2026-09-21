#![no_std]

//! Fixed-capacity USB delivery primitives shared by the kernel and host tests.

pub mod installation;
pub mod installation_ack;
pub mod installation_queue;

use core::fmt::{self, Arguments, Write};

/// Host-visible USB CDC link state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkState {
    /// No configured USB host is currently available.
    Disconnected,
    /// The host has configured the USB device and can receive CDC data.
    Configured,
}

/// Result of attempting to move accepted bytes to the transport endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlushStatus {
    /// All bytes accepted by the sink were handed to the endpoint.
    Complete,
    /// The sink still owns accepted bytes and needs another service attempt.
    Pending,
    /// The sink reported a transport failure.
    Failed,
}

/// Result of one bounded queue service operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrainReport {
    /// Number of records discarded because the queue overflowed.
    pub dropped: u32,
    /// Result of flushing the sink after queue service.
    pub flush: FlushStatus,
}

/// A byte sink used by the bounded delivery engine.
pub trait ByteSink {
    /// Attempts to write bytes and returns the number accepted.
    fn write(&mut self, bytes: &[u8]) -> usize;

    /// Flushes bytes accepted by the sink toward its transport endpoint.
    fn flush(&mut self) -> FlushStatus;
}

impl LinkState {
    /// Returns the state represented by the latest USB device configuration.
    pub const fn from_configured(configured: bool) -> Self {
        if configured {
            Self::Configured
        } else {
            Self::Disconnected
        }
    }

    /// Returns whether the configured CDC terminal is open on the host.
    pub const fn from_configured_and_open(configured: bool, terminal_open: bool) -> Self {
        Self::from_configured(configured && terminal_open)
    }

    /// Returns whether the link can accept CDC output.
    pub const fn is_configured(self) -> bool {
        matches!(self, Self::Configured)
    }
}

/// A fixed-size formatted log record.
#[derive(Clone, Copy)]
pub struct LogLine<const LINE_CAPACITY: usize> {
    bytes: [u8; LINE_CAPACITY],
    length: usize,
}

impl<const LINE_CAPACITY: usize> LogLine<LINE_CAPACITY> {
    /// Formats arguments into a fixed-size record.
    pub fn format(arguments: Arguments<'_>) -> Result<Self, fmt::Error> {
        let mut line = Self {
            bytes: [0; LINE_CAPACITY],
            length: 0,
        };
        write!(LineWriter { line: &mut line }, "{}", arguments)?;
        Ok(line)
    }

    /// Creates a record when the input fits in the fixed record buffer.
    pub const fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > LINE_CAPACITY {
            return None;
        }
        let mut record = Self {
            bytes: [0; LINE_CAPACITY],
            length: bytes.len(),
        };
        let mut index = 0;
        while index < bytes.len() {
            record.bytes[index] = bytes[index];
            index += 1;
        }
        Some(record)
    }

    /// Copies the remaining record bytes into the destination buffer.
    pub fn copy_from(&self, offset: usize, destination: &mut [u8]) -> usize {
        let remaining = self.length.saturating_sub(offset);
        let count = remaining.min(destination.len());
        destination[..count].copy_from_slice(&self.bytes[offset..offset + count]);
        count
    }

    /// Returns the number of bytes in the record.
    pub const fn length(self) -> usize {
        self.length
    }
}

struct LineWriter<'a, const LINE_CAPACITY: usize> {
    line: &'a mut LogLine<LINE_CAPACITY>,
}

impl<const LINE_CAPACITY: usize> Write for LineWriter<'_, LINE_CAPACITY> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self
            .line
            .length
            .checked_add(value.len())
            .ok_or(fmt::Error)?;
        if end > LINE_CAPACITY {
            return Err(fmt::Error);
        }
        self.line.bytes[self.line.length..end].copy_from_slice(value.as_bytes());
        self.line.length = end;
        Ok(())
    }
}

/// A bounded FIFO queue that retains the newest records after overflow.
pub struct LogQueue<const LINE_CAPACITY: usize, const QUEUE_CAPACITY: usize> {
    lines: [LogLine<LINE_CAPACITY>; QUEUE_CAPACITY],
    head: usize,
    length: usize,
    offset: usize,
    dropped: u32,
}

impl<const LINE_CAPACITY: usize, const QUEUE_CAPACITY: usize>
    LogQueue<LINE_CAPACITY, QUEUE_CAPACITY>
{
    /// Creates an empty queue.
    pub const fn new() -> Self {
        assert!(LINE_CAPACITY > 0);
        assert!(QUEUE_CAPACITY > 0);
        Self {
            lines: [LogLine {
                bytes: [0; LINE_CAPACITY],
                length: 0,
            }; QUEUE_CAPACITY],
            head: 0,
            length: 0,
            offset: 0,
            dropped: 0,
        }
    }

    /// Adds a record, dropping the oldest record when the queue is full.
    pub fn push(&mut self, line: LogLine<LINE_CAPACITY>) {
        let index = (self.head + self.length) % QUEUE_CAPACITY;
        self.lines[index] = line;
        if self.length == QUEUE_CAPACITY {
            self.head = (self.head + 1) % QUEUE_CAPACITY;
            self.offset = 0;
            self.dropped = self.dropped.saturating_add(1);
        } else {
            self.length += 1;
        }
    }

    /// Copies the unsent bytes at the front of the queue.
    pub fn copy_front(&self, destination: &mut [u8]) -> usize {
        if self.length == 0 {
            return 0;
        }
        self.lines[self.head].copy_from(self.offset, destination)
    }

    /// Advances the front by the number of bytes accepted by the transport.
    pub fn advance(&mut self, count: usize) {
        if self.length == 0 {
            return;
        }
        self.offset += count;
        if self.offset >= self.lines[self.head].length() {
            self.head = (self.head + 1) % QUEUE_CAPACITY;
            self.length -= 1;
            self.offset = 0;
        }
    }

    /// Returns and clears the number of records dropped by overflow.
    pub fn take_dropped(&mut self) -> u32 {
        let dropped = self.dropped;
        self.dropped = 0;
        dropped
    }
}

impl<const LINE_CAPACITY: usize, const QUEUE_CAPACITY: usize> Default
    for LogQueue<LINE_CAPACITY, QUEUE_CAPACITY>
{
    fn default() -> Self {
        Self::new()
    }
}

/// Drains queued records without discarding bytes rejected by the sink.
pub fn drain<const LINE_CAPACITY: usize, const QUEUE_CAPACITY: usize, S: ByteSink + ?Sized>(
    queue: &mut LogQueue<LINE_CAPACITY, QUEUE_CAPACITY>,
    link: LinkState,
    sink: &mut S,
) -> DrainReport {
    if !link.is_configured() {
        return DrainReport {
            dropped: 0,
            flush: FlushStatus::Complete,
        };
    }

    let mut chunk = [0; LINE_CAPACITY];
    loop {
        let count = queue.copy_front(&mut chunk);
        if count == 0 {
            break;
        }
        let written = sink.write(&chunk[..count]).min(count);
        if written == 0 {
            return DrainReport {
                dropped: 0,
                flush: sink.flush(),
            };
        }
        queue.advance(written);
    }
    DrainReport {
        dropped: queue.take_dropped(),
        flush: sink.flush(),
    }
}

#[cfg(test)]
mod tests;
