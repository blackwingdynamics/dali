use super::{ByteSink, LinkState, LogLine, LogQueue, drain};

const LINE_CAPACITY: usize = 8;
const QUEUE_CAPACITY: usize = 2;
const USB_SERVICE_BUDGET: usize = 1;
const FINAL_SERVICE_BUDGET: usize = 4;
const FIRST_LOG: u8 = b'A';
const SECOND_LOG: u8 = b'B';
const THIRD_LOG: u8 = b'C';

struct TestSink {
    limit: usize,
    bytes: [u8; 16],
    length: usize,
}

enum LifecycleEvent {
    StorageStep,
    HostConfigured,
    HostDisconnected,
    Log(u8),
}

struct LifecycleSink {
    bytes: [u8; 16],
    length: usize,
    budget: usize,
}

impl LifecycleSink {
    fn service(
        &mut self,
        queue: &mut LogQueue<LINE_CAPACITY, QUEUE_CAPACITY>,
        link: LinkState,
        budget: usize,
    ) {
        self.budget = budget;
        let _ = drain(queue, link, self);
    }
}

impl ByteSink for LifecycleSink {
    fn write(&mut self, bytes: &[u8]) -> usize {
        let count = bytes.len().min(self.budget);
        self.bytes[self.length..self.length + count].copy_from_slice(&bytes[..count]);
        self.length += count;
        self.budget -= count;
        count
    }
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

#[test]
fn interleaves_blocking_storage_with_usb_lifecycle() {
    let events = [
        LifecycleEvent::Log(FIRST_LOG),
        LifecycleEvent::StorageStep,
        LifecycleEvent::Log(SECOND_LOG),
        LifecycleEvent::HostConfigured,
        LifecycleEvent::StorageStep,
        LifecycleEvent::HostDisconnected,
        LifecycleEvent::Log(THIRD_LOG),
        LifecycleEvent::StorageStep,
        LifecycleEvent::HostConfigured,
    ];
    let mut queue = LogQueue::<LINE_CAPACITY, QUEUE_CAPACITY>::new();
    let mut sink = LifecycleSink {
        bytes: [0; 16],
        length: 0,
        budget: 0,
    };
    let mut link = LinkState::Disconnected;

    for event in events {
        match event {
            LifecycleEvent::StorageStep => sink.service(
                &mut queue,
                link,
                if link.is_configured() {
                    USB_SERVICE_BUDGET
                } else {
                    0
                },
            ),
            LifecycleEvent::HostConfigured => {
                link = LinkState::Configured;
                sink.service(&mut queue, link, USB_SERVICE_BUDGET);
            }
            LifecycleEvent::HostDisconnected => link = LinkState::Disconnected,
            LifecycleEvent::Log(marker) => queue.push(line(marker, 1)),
        }
    }

    sink.service(&mut queue, link, FINAL_SERVICE_BUDGET);
    assert_eq!(
        &sink.bytes[..sink.length],
        &[FIRST_LOG, SECOND_LOG, THIRD_LOG]
    );
    assert_eq!(queue.copy_front(&mut [0; LINE_CAPACITY]), 0);
}
