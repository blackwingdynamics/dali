//! Strict parser for canonical signed metadata envelopes.

use super::{DecodeError, parse_signature_list};
use crate::SignedEnvelope;

const SIGNED_PREFIX: &[u8] = b"{\"signed\":";
const SIGNATURES_PREFIX: &[u8] = b",\"signatures\":";

/// Parses a canonical envelope and borrows its exact signed body bytes.
pub fn parse_signed_envelope(bytes: &[u8]) -> Result<SignedEnvelope<'_>, DecodeError> {
    let body_start = SIGNED_PREFIX.len();
    if !bytes.starts_with(SIGNED_PREFIX) || bytes.get(body_start) != Some(&b'{') {
        return Err(DecodeError::UnexpectedToken);
    }
    let body_end = find_object_end(bytes, body_start)?;
    let remainder = bytes.get(body_end..).ok_or(DecodeError::UnexpectedEnd)?;
    if !remainder.starts_with(SIGNATURES_PREFIX) || !remainder.ends_with(b"}") {
        return Err(DecodeError::UnexpectedToken);
    }
    let signature_start = SIGNATURES_PREFIX.len();
    let signature_end = remainder.len() - 1;
    let signatures = parse_signature_list(
        remainder
            .get(signature_start..signature_end)
            .ok_or(DecodeError::UnexpectedEnd)?,
    )?;
    Ok(SignedEnvelope {
        signed: bytes
            .get(body_start..body_end)
            .ok_or(DecodeError::UnexpectedEnd)?,
        signatures,
    })
}

fn find_object_end(bytes: &[u8], start: usize) -> Result<usize, DecodeError> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut offset = start;
    while let Some(&byte) = bytes.get(offset) {
        if in_string {
            match byte {
                b'\\' => return Err(DecodeError::InvalidString),
                b'"' => in_string = false,
                0..=0x1F => return Err(DecodeError::InvalidString),
                _ => {}
            }
        } else {
            match byte {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.checked_sub(1).ok_or(DecodeError::UnexpectedToken)?;
                    if depth == 0 {
                        return Ok(offset + 1);
                    }
                }
                _ => {}
            }
        }
        offset += 1;
    }
    Err(DecodeError::UnexpectedEnd)
}
