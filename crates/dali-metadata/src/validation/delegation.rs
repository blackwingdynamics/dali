//! Validation for developer delegation authorization scopes.

use super::MetadataError;
use crate::{
    DelegationMetadata, KEY_ID_LENGTH, MAX_DELEGATION_ABIS, MAX_DELEGATION_SCOPES,
    MAX_DELEGATION_TARGETS, MetadataRole, PUBLIC_KEY_LENGTH, validate_developer_id,
    validate_namespace,
};

/// Validates one bounded developer delegation.
pub fn validate_delegation(metadata: &DelegationMetadata) -> Result<(), MetadataError> {
    if metadata.header.role != MetadataRole::Delegation
        || metadata.header.version == 0
        || usize::from(metadata.namespace_count) > MAX_DELEGATION_SCOPES
        || usize::from(metadata.target_count) > MAX_DELEGATION_TARGETS
        || usize::from(metadata.abi_count) > MAX_DELEGATION_ABIS
        || metadata.key_id.0 == [0; KEY_ID_LENGTH]
        || metadata.public_key.0 == [0; PUBLIC_KEY_LENGTH]
        || (metadata.not_after != 0 && metadata.not_before > metadata.not_after)
    {
        return Err(MetadataError::InvalidDelegation);
    }
    let developer_id = metadata
        .developer_id
        .as_str()
        .ok_or(MetadataError::InvalidDelegation)?;
    validate_developer_id(developer_id)?;
    validate_unique_texts(
        &metadata.allowed_namespaces[..usize::from(metadata.namespace_count)],
        validate_namespace,
    )?;
    validate_unique_texts(
        &metadata.allowed_targets[..usize::from(metadata.target_count)],
        validate_target_profile,
    )?;
    let abis = &metadata.allowed_abis[..usize::from(metadata.abi_count)];
    if abis.is_empty() || abis.contains(&0) || abis.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(MetadataError::InvalidDelegation);
    }
    Ok(())
}

fn validate_unique_texts<const CAPACITY: usize>(
    values: &[crate::BoundedText<CAPACITY>],
    validate: impl Fn(&str) -> Result<(), MetadataError>,
) -> Result<(), MetadataError> {
    if values.is_empty() {
        return Err(MetadataError::InvalidDelegation);
    }
    for (index, value) in values.iter().enumerate() {
        let value = value.as_str().ok_or(MetadataError::InvalidDelegation)?;
        validate(value)?;
        if values[..index]
            .iter()
            .any(|candidate| candidate.as_str() == Some(value))
            || (index > 0
                && values[index - 1]
                    .as_str()
                    .is_none_or(|previous| previous >= value))
        {
            return Err(MetadataError::InvalidDelegation);
        }
    }
    Ok(())
}

fn validate_target_profile(value: &str) -> Result<(), MetadataError> {
    if value.is_empty()
        || value.len() > crate::MAX_TARGET_PROFILE_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
    {
        Err(MetadataError::InvalidDelegation)
    } else {
        Ok(())
    }
}
