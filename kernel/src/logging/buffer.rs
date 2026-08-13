//! Fixed-capacity USB log queue used while the CDC device is unavailable.

use core::fmt::{self, Arguments, Write};

use super::Level;

pub(crate) const LOG_LINE_CAPACITY: usize = 128;
pub(crate) const LOG_QUEUE_CAPACITY: usize = 32;

#[derive(Clone, Copy)]
pub(crate) struct LogLine {
    bytes: [u8; LOG_LINE_CAPACITY],
    length: usize,
}

impl LogLine {
    pub(crate) fn format(
        level: Level,
        subsystem: &'static str,
        arguments: Arguments<'_>,
    ) -> Result<Self, fmt::Error> {
        let mut line = Self {
            bytes: [0; LOG_LINE_CAPACITY],
            length: 0,
        };
        write!(
            LineWriter { line: &mut line },
            "[{}][{}] {}\r\n",
            level.label(),
            subsystem,
            arguments
        )?;
        Ok(line)
    }

    pub(crate) fn copy_from(&self, offset: usize, destination: &mut [u8]) -> usize {
        let remaining = self.length.saturating_sub(offset);
        let count = remaining.min(destination.len());
        destination[..count].copy_from_slice(&self.bytes[offset..offset + count]);
        count
    }

    pub(crate) const fn length(self) -> usize {
        self.length
    }
}

struct LineWriter<'a> {
    line: &'a mut LogLine,
}

impl Write for LineWriter<'_> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self
            .line
            .length
            .checked_add(value.len())
            .ok_or(fmt::Error)?;
        if end > LOG_LINE_CAPACITY {
            return Err(fmt::Error);
        }
        self.line.bytes[self.line.length..end].copy_from_slice(value.as_bytes());
        self.line.length = end;
        Ok(())
    }
}

pub(crate) struct LogQueue {
    lines: [LogLine; LOG_QUEUE_CAPACITY],
    head: usize,
    length: usize,
    offset: usize,
    dropped: u32,
}

impl LogQueue {
    pub(crate) const fn new() -> Self {
        const EMPTY_LINE: LogLine = LogLine {
            bytes: [0; LOG_LINE_CAPACITY],
            length: 0,
        };
        Self {
            lines: [EMPTY_LINE; LOG_QUEUE_CAPACITY],
            head: 0,
            length: 0,
            offset: 0,
            dropped: 0,
        }
    }

    pub(crate) fn push(&mut self, line: LogLine) {
        let index = (self.head + self.length) % LOG_QUEUE_CAPACITY;
        self.lines[index] = line;
        if self.length == LOG_QUEUE_CAPACITY {
            self.head = (self.head + 1) % LOG_QUEUE_CAPACITY;
            self.offset = 0;
            self.dropped = self.dropped.saturating_add(1);
        } else {
            self.length += 1;
        }
    }

    pub(crate) fn copy_front(&self, destination: &mut [u8]) -> usize {
        if self.length == 0 {
            return 0;
        }
        self.lines[self.head].copy_from(self.offset, destination)
    }

    pub(crate) fn advance(&mut self, count: usize) {
        if self.length == 0 {
            return;
        }
        self.offset += count;
        if self.offset >= self.lines[self.head].length() {
            self.head = (self.head + 1) % LOG_QUEUE_CAPACITY;
            self.length -= 1;
            self.offset = 0;
        }
    }

    pub(crate) fn take_dropped(&mut self) -> u32 {
        let dropped = self.dropped;
        self.dropped = 0;
        dropped
    }
}
