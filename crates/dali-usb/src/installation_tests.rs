use super::*;

fn frame(command: Command, offset: u32, total_length: u32, payload: &[u8]) -> Frame<'_> {
    Frame {
        command,
        offset,
        total_length,
        payload,
    }
}

#[test]
fn frame_round_trip_preserves_bounded_fields() {
    let source = frame(Command::Data, 4, 8, b"data");
    let mut encoded = [0; MAX_FRAME_SIZE];
    let length = source.encode_into(&mut encoded).unwrap();
    assert_eq!(Frame::decode(&encoded[..length]).unwrap(), source);
}

#[test]
fn rejects_corrupted_frame() {
    let source = frame(Command::Begin, 0, 8, &[]);
    let mut encoded = [0; MAX_FRAME_SIZE];
    let length = source.encode_into(&mut encoded).unwrap();
    encoded[HEADER_SIZE] ^= 1;
    assert_eq!(
        Frame::decode(&encoded[..length]),
        Err(FrameError::InvalidChecksum)
    );
}

#[test]
fn session_requires_contiguous_complete_candidate() {
    let mut session = Session::new();
    assert_eq!(
        session.accept(frame(Command::Begin, 0, 8, &[])),
        Ok(Event::Begin { total_length: 8 })
    );
    assert_eq!(
        session.accept(frame(Command::Data, 0, 4, b"test")),
        Ok(Event::Data {
            offset: 0,
            payload_length: 4
        })
    );
    assert_eq!(
        session.accept(frame(Command::Validate, 0, 0, &[])),
        Err(SessionError::InvalidTransition)
    );
    session.accept(frame(Command::Data, 4, 4, b"done")).unwrap();
    assert_eq!(
        session.accept(frame(Command::Validate, 0, 0, &[])),
        Ok(Event::Validate)
    );
    assert_eq!(
        session.accept(frame(Command::Commit, 0, 0, &[])),
        Ok(Event::Commit)
    );
}
