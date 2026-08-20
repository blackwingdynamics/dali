use crate::{EncodeError, MAX_ENVELOPE_BYTES, SignatureSet, validate_signature_set};

use super::writer::Writer;

/// Encodes a canonical signed metadata envelope.
pub fn encode_signed_envelope(
    output: &mut [u8],
    signed: &[u8],
    signatures: SignatureSet,
) -> Result<usize, EncodeError> {
    if signed.len() < 2 || signed[0] != b'{' || signed[signed.len() - 1] != b'}' {
        return Err(EncodeError::InvalidValue);
    }
    validate_signature_set(&signatures).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = Writer::new(output, MAX_ENVELOPE_BYTES);
    writer.object_start()?;
    writer.field_name("signed")?;
    writer.bytes(signed)?;
    writer.comma()?;
    writer.field_name("signatures")?;
    writer.array_start()?;
    for (index, record) in signatures.records[..usize::from(signatures.count)]
        .iter()
        .enumerate()
    {
        if index != 0 {
            writer.comma()?;
        }
        writer.object_start()?;
        writer.field_name("key_id")?;
        writer.hex(&record.key_id)?;
        writer.comma()?;
        writer.field_name("signature")?;
        writer.hex(&record.signature)?;
        writer.object_end()?;
    }
    writer.array_end()?;
    writer.object_end()?;
    Ok(writer.len())
}
