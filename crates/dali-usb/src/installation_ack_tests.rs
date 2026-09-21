use super::*;
use crate::installation::HEADER_SIZE;

fn acknowledgement() -> Acknowledgement {
    Acknowledgement {
        command: Command::Data,
        status: AckStatus::Accepted,
        detail: 0,
        next_offset: 512,
    }
}

#[test]
fn acknowledgement_round_trip_preserves_contract_fields() {
    let source = acknowledgement();
    let mut encoded = [0; MAX_FRAME_SIZE];
    let length = source.encode_into(&mut encoded).unwrap();
    assert_eq!(Acknowledgement::decode(&encoded[..length]), Ok(source));
}

#[test]
fn rejects_wrong_payload_size() {
    let source = Frame {
        command: Command::Commit,
        offset: 0,
        total_length: 0,
        payload: &[],
    };
    let mut encoded = [0; MAX_FRAME_SIZE];
    let length = source.encode_into(&mut encoded).unwrap();
    assert_eq!(
        Acknowledgement::decode(&encoded[..length]),
        Err(AckError::InvalidPayload)
    );
}

#[test]
fn rejects_unknown_status() {
    let source = acknowledgement();
    let mut encoded = [0; MAX_FRAME_SIZE];
    let length = source.encode_into(&mut encoded).unwrap();
    encoded[16] = 0xFF;
    let checksum_offset = HEADER_SIZE + ACK_PAYLOAD_SIZE;
    let checksum = crate::installation::crc32(&encoded[..checksum_offset]);
    encoded[checksum_offset..length].copy_from_slice(&checksum.to_le_bytes());
    assert_eq!(
        Acknowledgement::decode(&encoded[..length]),
        Err(AckError::InvalidStatus)
    );
}
