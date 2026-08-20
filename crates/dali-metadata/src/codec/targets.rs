use crate::{
    BoundedText, EncodeError, MAX_DELEGATION_ID_BYTES, MAX_TARGET_RECORDS, MAX_TARGETS_BYTES,
    MetadataHeader, MetadataRole, SCHEMA_ID, TargetPackage, validate_target_records,
};

use super::writer::Writer;

/// Encodes the canonical signed body of targets metadata.
pub fn encode_targets_signed(
    output: &mut [u8],
    header: MetadataHeader,
    delegations: &[BoundedText<MAX_DELEGATION_ID_BYTES>],
    packages: &[TargetPackage],
) -> Result<usize, EncodeError> {
    if header.role != MetadataRole::Targets || header.version == 0 {
        return Err(EncodeError::InvalidValue);
    }
    if delegations.len() > crate::MAX_DELEGATION_SCOPES || packages.len() > MAX_TARGET_RECORDS {
        return Err(EncodeError::TooManyRecords);
    }
    validate_target_records(delegations, packages).map_err(|_| EncodeError::InvalidValue)?;
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
    writer.field_name("packages")?;
    writer.array_start()?;
    for (index, package) in packages.iter().enumerate() {
        if index != 0 {
            writer.comma()?;
        }
        encode_package(&mut writer, *package)?;
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

fn encode_package(writer: &mut Writer<'_>, package: TargetPackage) -> Result<(), EncodeError> {
    writer.object_start()?;
    writer.field_name("abi_version")?;
    writer.number(u64::from(package.abi_version))?;
    writer.comma()?;
    writer.field_name("amrn_format")?;
    writer.number(u64::from(package.amrn_format))?;
    writer.comma()?;
    writer.field_name("developer_id")?;
    writer.text(package.developer_id)?;
    writer.comma()?;
    writer.field_name("developer_key_id")?;
    writer.hex(&package.developer_key_id)?;
    writer.comma()?;
    writer.field_name("delegation_id")?;
    writer.text(package.delegation_id)?;
    writer.comma()?;
    writer.field_name("length")?;
    writer.number(u64::from(package.length))?;
    writer.comma()?;
    writer.field_name("minimum_kernel_version")?;
    writer.text(package.minimum_kernel_version)?;
    writer.comma()?;
    writer.field_name("namespace")?;
    writer.text(package.namespace)?;
    writer.comma()?;
    writer.field_name("package_id")?;
    writer.hex(&package.package_id)?;
    writer.comma()?;
    writer.field_name("package_version")?;
    writer.text(package.package_version)?;
    writer.comma()?;
    writer.field_name("required_services")?;
    writer.number(u64::from(package.required_services))?;
    writer.comma()?;
    writer.field_name("sha256")?;
    writer.hex(&package.sha256)?;
    writer.comma()?;
    writer.field_name("slot_id")?;
    writer.number(u64::from(package.slot_id))?;
    writer.comma()?;
    writer.field_name("target_profile")?;
    writer.text(package.target_profile)?;
    writer.object_end()
}
