use crate::{EncodeError, MAX_SIGNATURES, SignatureRecord, SignatureSet, validate_signature_set};

use super::writer::Writer;

/// Encodes a canonical metadata signature array.
pub fn encode_signature_list(output: &mut [u8], set: SignatureSet) -> Result<usize, EncodeError> {
    validate_signature_set(&set).map_err(|_| EncodeError::InvalidValue)?;
    if usize::from(set.count) > MAX_SIGNATURES {
        return Err(EncodeError::TooManyRecords);
    }
    let mut writer = Writer::new(output, crate::MAX_ROOT_BYTES);
    writer.array_start()?;
    for (index, record) in set.records[..usize::from(set.count)].iter().enumerate() {
        if index != 0 {
            writer.comma()?;
        }
        encode_signature(&mut writer, *record)?;
    }
    writer.array_end()?;
    Ok(writer.len())
}

fn encode_signature(writer: &mut Writer<'_>, record: SignatureRecord) -> Result<(), EncodeError> {
    writer.object_start()?;
    writer.field_name("key_id")?;
    writer.hex(&record.key_id)?;
    writer.comma()?;
    writer.field_name("signature")?;
    writer.hex(&record.signature)?;
    writer.object_end()
}
