use super::{ByteSink, LinkState, LogLine, LogQueue, drain};

const LINE_CAPACITY: usize = 8;
const QUEUE_CAPACITY: usize = 2;

struct TestSink {
    limit: usize,
    bytes: [u8; 16],
    length: usize,
}

impl ByteSink for TestSink {
    fn write(&mut self, bytes: &[u8]) -> usize {
        let count = bytes.len().min(self.limit);
        self.bytes[self.length..self.length + count].copy_from_slice(&bytes[..count]);
        self.length += count;
        count
    }
}

fn line(marker: u8, length: usize) -> LogLine<LINE_CAPACITY> {
    let mut bytes = [0; LINE_CAPACITY];
    bytes[0] = marker;
    match LogLine::from_bytes(&bytes[..length]) {
        Some(line) => line,
        None => LogLine {
            bytes: [0; LINE_CAPACITY],
            length: 0,
        },
    }
}

#[test]
fn advances_only_accepted_bytes() {
    let mut queue = LogQueue::<LINE_CAPACITY, QUEUE_CAPACITY>::new();
    queue.push(line(0x41, 3));
    let mut chunk = [0; 2];

    assert_eq!(queue.copy_front(&mut chunk), 2);
    queue.advance(2);
    assert_eq!(queue.copy_front(&mut chunk), 1);
    queue.advance(1);
    assert_eq!(queue.copy_front(&mut chunk), 0);
}

#[test]
fn preserves_fifo_order() {
    let mut queue = LogQueue::<LINE_CAPACITY, QUEUE_CAPACITY>::new();
    queue.push(line(0x41, 1));
    queue.push(line(0x42, 1));
    let mut chunk = [0; 1];

    assert_eq!(queue.copy_front(&mut chunk), 1);
    assert_eq!(chunk[0], 0x41);
    queue.advance(1);
    assert_eq!(queue.copy_front(&mut chunk), 1);
    assert_eq!(chunk[0], 0x42);
}

#[test]
fn reports_overflow_and_keeps_newest_records() {
    let mut queue = LogQueue::<LINE_CAPACITY, QUEUE_CAPACITY>::new();
    queue.push(line(0x41, 1));
    queue.push(line(0x42, 1));
    queue.push(line(0x43, 1));
    let mut chunk = [0; 1];

    assert_eq!(queue.copy_front(&mut chunk), 1);
    assert_eq!(chunk[0], 0x42);
    assert_eq!(queue.take_dropped(), 1);
    assert_eq!(queue.take_dropped(), 0);
}

#[test]
fn models_disconnect_and_reconnect() {
    let disconnected = LinkState::from_configured(false);
    assert_eq!(disconnected, LinkState::Disconnected);
    assert!(!disconnected.is_configured());

    let configured = LinkState::from_configured(true);
    assert_eq!(configured, LinkState::Configured);
    assert!(configured.is_configured());
}

#[test]
fn retains_records_across_disconnect_and_partial_writes() {
    let mut queue = LogQueue::<LINE_CAPACITY, QUEUE_CAPACITY>::new();
    queue.push(line(0x41, 3));
    let mut sink = TestSink {
        limit: 2,
        bytes: [0; 16],
        length: 0,
    };

    assert_eq!(drain(&mut queue, LinkState::Disconnected, &mut sink), 0);
    assert_eq!(sink.length, 0);
    assert_eq!(drain(&mut queue, LinkState::Configured, &mut sink), 0);
    assert_eq!(sink.length, 3);
    assert_eq!(&sink.bytes[..3], &[0x41, 0, 0]);
}
