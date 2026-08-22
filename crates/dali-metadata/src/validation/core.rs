//! Validation of common metadata headers and role declarations.

use super::MetadataError;

/// Trusted wall-clock boundary used by expiry validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustedTime {
    /// No trustworthy wall clock is available; expiry is not evaluated.
    Unavailable,
    /// Unix timestamp supplied by a trusted platform time source.
    UnixSeconds(u64),
}

/// Validates common signed metadata fields.
pub fn validate_header(
    header: crate::MetadataHeader,
    now: Option<u64>,
) -> Result<(), MetadataError> {
    validate_header_at(
        header,
        match now {
            Some(value) => TrustedTime::UnixSeconds(value),
            None => TrustedTime::Unavailable,
        },
    )
}

/// Validates common signed metadata fields against an explicit time boundary.
pub fn validate_header_at(
    header: crate::MetadataHeader,
    trusted_time: TrustedTime,
) -> Result<(), MetadataError> {
    if header.version == 0 {
        return Err(MetadataError::InvalidVersion);
    }
    if let TrustedTime::UnixSeconds(now) = trusted_time
        && header.expires != 0
        && now > header.expires
    {
        return Err(MetadataError::Expired);
    }
    Ok(())
}

/// Validates one role's bounded key list and signature threshold.
pub fn validate_role(role: crate::RoleDefinition) -> Result<(), MetadataError> {
    let key_count = usize::from(role.key_count);
    let threshold = usize::from(role.threshold);
    if key_count == 0 || key_count > crate::MAX_ROLE_KEYS || threshold == 0 || threshold > key_count
    {
        return Err(MetadataError::InvalidRoleThreshold);
    }
    for key in role.keys.iter().take(key_count) {
        if key.0 == [0; crate::KEY_ID_LENGTH] {
            return Err(MetadataError::InvalidKeyId);
        }
    }
    Ok(())
}

/// Validates that every role reference resolves to a declared public key.
pub fn validate_role_references(
    keys: &[crate::RoleKey],
    roles: &[crate::RoleDefinition],
) -> Result<(), MetadataError> {
    for role in roles {
        for key_id in role.keys.iter().take(usize::from(role.key_count)) {
            if !keys.iter().any(|key| key.key_id == *key_id) {
                return Err(MetadataError::UnknownRoleKey);
            }
        }
    }
    Ok(())
}
