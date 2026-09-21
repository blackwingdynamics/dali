use super::{MetadataError, validate_developer_id, validate_namespace, validate_reference};
use crate::{
    KEY_ID_LENGTH, MAX_DELEGATION_SCOPES, MAX_TARGET_RECORDS, MetadataRole, TargetsMetadata,
};

pub fn validate_targets_metadata(metadata: &TargetsMetadata) -> Result<(), MetadataError> {
    if metadata.header.role != MetadataRole::Targets
        || metadata.header.version == 0
        || usize::from(metadata.delegation_count) > MAX_DELEGATION_SCOPES
        || usize::from(metadata.cartridge_count) > MAX_TARGET_RECORDS
    {
        return Err(MetadataError::InvalidCartridgeRecord);
    }
    validate_target_records(
        &metadata.delegations[..usize::from(metadata.delegation_count)],
        &metadata.cartridges[..usize::from(metadata.cartridge_count)],
    )
}

pub fn validate_target_records(
    delegations: &[crate::BoundedText<{ crate::MAX_DELEGATION_ID_BYTES }>],
    cartridges: &[crate::TargetCartridge],
) -> Result<(), MetadataError> {
    if delegations.len() > MAX_DELEGATION_SCOPES || cartridges.len() > MAX_TARGET_RECORDS {
        return Err(MetadataError::InvalidCartridgeRecord);
    }
    for (index, delegation) in delegations.iter().enumerate() {
        if delegation.as_str().is_none()
            || delegations[..index]
                .iter()
                .any(|candidate| candidate == delegation)
        {
            return Err(MetadataError::InvalidCartridgeRecord);
        }
    }
    for (index, cartridge) in cartridges.iter().enumerate() {
        if cartridge.cartridge_id.0 == [0; KEY_ID_LENGTH]
            || cartridge.developer_key_id.0 == [0; KEY_ID_LENGTH]
            || cartridge.sha256.0 == [0; crate::SHA256_LENGTH]
            || cartridge.length == 0
            || cartridge.amrn_format == 0
            || cartridge.abi_version == 0
            || cartridge.namespace.as_str().is_none()
            || cartridge.developer_id.as_str().is_none()
            || cartridge.delegation_id.as_str().is_none()
            || cartridge.target_profile.as_str().is_none()
            || cartridge.cartridge_version.as_str().is_none()
            || cartridge.minimum_kernel_version.as_str().is_none()
        {
            return Err(MetadataError::InvalidCartridgeRecord);
        }
        validate_namespace(
            cartridge
                .namespace
                .as_str()
                .ok_or(MetadataError::InvalidCartridgeRecord)?,
        )?;
        validate_developer_id(
            cartridge
                .developer_id
                .as_str()
                .ok_or(MetadataError::InvalidCartridgeRecord)?,
        )?;
        if !delegations
            .iter()
            .any(|delegation| delegation == &cartridge.delegation_id)
        {
            return Err(MetadataError::UnknownDelegation);
        }
        if cartridges[..index]
            .iter()
            .any(|candidate| candidate.cartridge_id == cartridge.cartridge_id)
        {
            return Err(MetadataError::DuplicateCartridge);
        }
    }
    Ok(())
}

pub fn validate_snapshot_metadata(metadata: &crate::SnapshotMetadata) -> Result<(), MetadataError> {
    if metadata.header.role != MetadataRole::Snapshot
        || metadata.header.version == 0
        || usize::from(metadata.delegation_count) > crate::MAX_SNAPSHOT_REFERENCES
    {
        return Err(MetadataError::InvalidMetadataReference);
    }
    validate_reference(
        metadata.targets.version,
        metadata.targets.length,
        metadata.targets.sha256,
    )?;
    validate_reference(
        metadata.revocations.version,
        metadata.revocations.length,
        metadata.revocations.sha256,
    )?;
    for (index, reference) in metadata
        .delegations
        .iter()
        .take(usize::from(metadata.delegation_count))
        .enumerate()
    {
        if reference.id.as_str().is_none()
            || metadata.delegations[..index]
                .iter()
                .any(|candidate| candidate.id == reference.id)
        {
            return Err(if reference.id.as_str().is_none() {
                MetadataError::InvalidMetadataReference
            } else {
                MetadataError::DuplicateDelegationReference
            });
        }
        validate_reference(reference.version, reference.length, reference.sha256)?;
    }
    Ok(())
}

pub fn validate_timestamp_metadata(
    metadata: &crate::TimestampMetadata,
) -> Result<(), MetadataError> {
    if metadata.header.role != MetadataRole::Timestamp || metadata.header.version == 0 {
        return Err(MetadataError::InvalidMetadataReference);
    }
    validate_reference(
        metadata.snapshot_version,
        metadata.snapshot_length,
        metadata.snapshot_sha256,
    )
}

pub fn validate_revocation_metadata(
    metadata: &crate::RevocationMetadata,
) -> Result<(), MetadataError> {
    if metadata.header.role != MetadataRole::Revocation
        || metadata.header.version == 0
        || usize::from(metadata.record_count) > crate::MAX_REVOCATIONS
    {
        return Err(MetadataError::InvalidRevocation);
    }
    let records = &metadata.records[..usize::from(metadata.record_count)];
    for (index, record) in records.iter().enumerate() {
        if record.issuer_key_id.0 == [0; KEY_ID_LENGTH]
            || record.key_id.0 == [0; KEY_ID_LENGTH]
            || record.effective_version == 0
            || record.developer_id.as_str().is_none()
            || record.reason.as_str().is_none()
            || records[..index].iter().any(|candidate| {
                candidate.developer_id == record.developer_id && candidate.key_id == record.key_id
            })
            || (index > 0 && !revocation_ordered(records[index - 1], *record))
        {
            return Err(MetadataError::InvalidRevocation);
        }
        validate_developer_id(
            record
                .developer_id
                .as_str()
                .ok_or(MetadataError::InvalidRevocation)?,
        )
        .map_err(|_| MetadataError::InvalidRevocation)?;
    }
    Ok(())
}

fn revocation_ordered(left: crate::RevocationRecord, right: crate::RevocationRecord) -> bool {
    let left_developer = left.developer_id.as_str().unwrap_or("");
    let right_developer = right.developer_id.as_str().unwrap_or("");
    left_developer < right_developer
        || (left_developer == right_developer && left.key_id.0 < right.key_id.0)
}
