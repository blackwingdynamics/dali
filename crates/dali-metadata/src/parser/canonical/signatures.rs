//! Strict parser for canonical metadata signature lists.

use super::{DecodeError, cursor::Cursor};
use crate::{
    KeyId, MAX_SIGNATURES, Signature, SignatureRecord, SignatureSet, validate_signature_set,
};

/// Parses one canonical signature array into fixed-capacity storage.
pub fn parse_signature_list(bytes: &[u8]) -> Result<SignatureSet, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    cursor.byte(b'[')?;
    let mut records = [SignatureRecord::default(); MAX_SIGNATURES];
    let mut count = 0_usize;
    if cursor.peek(b']') {
        cursor.byte(b']')?;
    } else {
        loop {
            if count == MAX_SIGNATURES {
                return Err(DecodeError::TooManyRecords);
            }
            records[count] = parse_signature(&mut cursor)?;
            count += 1;
            if cursor.peek(b',') {
                cursor.byte(b',')?;
            } else {
                break;
            }
        }
        cursor.byte(b']')?;
    }
    if !cursor.is_complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let set = SignatureSet {
        records,
        count: count as u8,
    };
    validate_signature_set(&set).map_err(|_| DecodeError::InvalidValue)?;
    Ok(set)
}

fn parse_signature(cursor: &mut Cursor<'_>) -> Result<SignatureRecord, DecodeError> {
    cursor.byte(b'{')?;
    cursor.field("key_id")?;
    let key_id = KeyId(cursor.hex::<{ crate::KEY_ID_LENGTH }>()?);
    cursor.byte(b',')?;
    cursor.field("signature")?;
    let signature = Signature(cursor.hex::<{ crate::SIGNATURE_LENGTH }>()?);
    cursor.byte(b'}')?;
    Ok(SignatureRecord { key_id, signature })
}
