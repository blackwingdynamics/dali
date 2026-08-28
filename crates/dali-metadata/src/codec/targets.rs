use crate::{
    BoundedText, EncodeError, MAX_DELEGATION_ID_BYTES, MAX_TARGET_RECORDS, MAX_TARGETS_BYTES,
    MetadataHeader, MetadataRole, SCHEMA_ID, TargetCartridge, validate_target_records,
};

use super::writer::Writer;

/// Encodes the canonical signed body of targets metadata.
pub fn encode_targets_signed(
    output: &mut [u8],
    header: MetadataHeader,
    delegations: &[BoundedText<MAX_DELEGATION_ID_BYTES>],
    cartridges: &[TargetCartridge],
) -> Result<usize, EncodeError> {
    if header.role != MetadataRole::Targets || header.version == 0 {
        return Err(EncodeError::InvalidValue);
    }
    if delegations.len() > crate::MAX_DELEGATION_SCOPES || cartridges.len() > MAX_TARGET_RECORDS {
        return Err(EncodeError::TooManyRecords);
    }
    validate_target_records(delegations, cartridges).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = Writer::new(output, MAX_TARGETS_BYTES);
    writer.object_start()?;
    writer.field_name("delegations")?;
    writer.array_start()?;
    for (index, delegation) in delegations.iter().enumerate() {
        if index != 0 {
            writer.comma()?;
        }
        writer.text(*delegation)?;
    }
    writer.array_end()?;
    writer.comma()?;
    writer.field_name("expires")?;
    writer.number(header.expires)?;
    writer.comma()?;
    writer.field_name("cartridges")?;
    writer.array_start()?;
    for (index, cartridge) in cartridges.iter().enumerate() {
        if index != 0 {
            writer.comma()?;
        }
        encode_cartridge(&mut writer, *cartridge)?;
    }
    writer.array_end()?;
    writer.comma()?;
    writer.field_name("role")?;
    writer.string(header.role.as_str())?;
    writer.comma()?;
    writer.field_name("schema")?;
    writer.string(SCHEMA_ID)?;
    writer.comma()?;
    writer.field_name("version")?;
    writer.number(header.version)?;
    writer.object_end()?;
    Ok(writer.len())
}

fn encode_cartridge(
    writer: &mut Writer<'_>,
    cartridge: TargetCartridge,
) -> Result<(), EncodeError> {
    writer.object_start()?;
    writer.field_name("abi_version")?;
    writer.number(u64::from(cartridge.abi_version))?;
    writer.comma()?;
    writer.field_name("amrn_format")?;
    writer.number(u64::from(cartridge.amrn_format))?;
    writer.comma()?;
    writer.field_name("developer_id")?;
    writer.text(cartridge.developer_id)?;
    writer.comma()?;
    writer.field_name("developer_key_id")?;
    writer.hex(&cartridge.developer_key_id)?;
    writer.comma()?;
    writer.field_name("delegation_id")?;
    writer.text(cartridge.delegation_id)?;
    writer.comma()?;
    writer.field_name("length")?;
    writer.number(u64::from(cartridge.length))?;
    writer.comma()?;
    writer.field_name("minimum_kernel_version")?;
    writer.text(cartridge.minimum_kernel_version)?;
    writer.comma()?;
    writer.field_name("namespace")?;
    writer.text(cartridge.namespace)?;
    writer.comma()?;
    writer.field_name("cartridge_id")?;
    writer.hex(&cartridge.cartridge_id)?;
    writer.comma()?;
    writer.field_name("cartridge_version")?;
    writer.text(cartridge.cartridge_version)?;
    writer.comma()?;
    writer.field_name("required_services")?;
    writer.number(u64::from(cartridge.required_services))?;
    writer.comma()?;
    writer.field_name("sha256")?;
    writer.hex(&cartridge.sha256)?;
    writer.comma()?;
    writer.field_name("slot_id")?;
    writer.number(u64::from(cartridge.slot_id))?;
    writer.comma()?;
    writer.field_name("target_profile")?;
    writer.text(cartridge.target_profile)?;
    writer.object_end()
}
