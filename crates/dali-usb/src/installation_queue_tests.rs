use super::*;
use crate::installation::{Command, Frame, MAX_FRAME_SIZE};

fn encoded_frame(command: Command, payload: &[u8]) -> ([u8; MAX_FRAME_SIZE], usize) {
    let frame = Frame {
        command,
        offset: 0,
        total_length: payload.len() as u32,
        payload,
    };
    let mut bytes = [0; MAX_FRAME_SIZE];
    let length = frame.encode_into(&mut bytes).unwrap();
    (bytes, length)
}

#[test]
fn assembles_frames_across_transport_chunks() {
    let (bytes, length) = encoded_frame(Command::Begin, &[]);
    let mut queue = InstallationReceiveQueue::<2>::new();
    let split = 3;
    assert_eq!(queue.push_bytes(&bytes[..split]).queued, 0);
    assert_eq!(queue.push_bytes(&bytes[split..length]).queued, 1);
    let mut output = [0; MAX_FRAME_SIZE];
    assert_eq!(queue.pop_frame(&mut output), Some(length));
    assert_eq!(&output[..length], &bytes[..length]);
}

#[test]
fn drops_frames_when_fifo_is_full() {
    let (bytes, length) = encoded_frame(Command::Abort, &[]);
    let mut queue = InstallationReceiveQueue::<1>::new();
    assert_eq!(queue.push_bytes(&bytes[..length]).queued, 1);
    assert_eq!(queue.push_bytes(&bytes[..length]).dropped, 1);
    assert_eq!(queue.take_dropped(), 1);
}

#[test]
fn rejects_invalid_header_and_recovers_for_next_frame() {
    let (valid, length) = encoded_frame(Command::Abort, &[]);
    let mut invalid = valid;
    invalid[0] = 0;
    let mut queue = InstallationReceiveQueue::<2>::new();
    let report = queue.push_bytes(&invalid[..length]);
    assert_eq!(report.queued, 0);
    assert!(!report.malformed);
    assert_eq!(queue.push_bytes(&valid[..length]).queued, 1);
}
