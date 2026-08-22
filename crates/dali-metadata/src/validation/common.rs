use super::MetadataError;

pub(crate) fn validate_reference(
    version: u64,
    length: u32,
    sha256: crate::Sha256Digest,
) -> Result<(), MetadataError> {
    if version == 0 || length == 0 || sha256.0 == [0; crate::SHA256_LENGTH] {
        Err(MetadataError::InvalidMetadataReference)
    } else {
        Ok(())
    }
}

pub fn validate_signature_set(set: &crate::SignatureSet) -> Result<(), MetadataError> {
    let count = usize::from(set.count);
    if count > crate::MAX_SIGNATURES {
        return Err(MetadataError::InvalidSignatureSet);
    }
    let records = &set.records[..count];
    if records.iter().any(|record| {
        record.key_id.0 == [0; crate::KEY_ID_LENGTH]
            || record.signature.0 == [0; crate::SIGNATURE_LENGTH]
    }) || !records
        .windows(2)
        .all(|pair| pair[0].key_id.0 < pair[1].key_id.0)
    {
        return Err(MetadataError::InvalidSignatureSet);
    }
    Ok(())
}

pub fn validate_developer_id(value: &str) -> Result<(), MetadataError> {
    if value.is_empty()
        || value.len() > crate::MAX_DEVELOPER_ID_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
    {
        Err(MetadataError::InvalidDeveloperId)
    } else {
        Ok(())
    }
}

pub fn validate_namespace(value: &str) -> Result<(), MetadataError> {
    if value.is_empty()
        || value.len() > crate::MAX_NAMESPACE_BYTES
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b'-'
                || byte == b'_'
                || byte == b'/'
        })
    {
        Err(MetadataError::InvalidNamespace)
    } else {
        Ok(())
    }
}

pub const fn is_repository_role(role: crate::MetadataRole) -> bool {
    matches!(
        role,
        crate::MetadataRole::Root
            | crate::MetadataRole::Timestamp
            | crate::MetadataRole::Snapshot
            | crate::MetadataRole::Targets
            | crate::MetadataRole::Delegation
            | crate::MetadataRole::Revocation
            | crate::MetadataRole::Recovery
            | crate::MetadataRole::Bundle
    )
}
