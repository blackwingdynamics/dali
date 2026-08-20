//! Validation for bounded metadata contract values.

use crate::{
    KEY_ID_LENGTH, MAX_DEVELOPER_ID_BYTES, MAX_NAMESPACE_BYTES, MAX_ROLE_KEYS, MetadataHeader,
    MetadataRole, RoleDefinition,
};

/// Errors returned when contract values violate the initial metadata profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataError {
    /// The metadata role is not supported by this profile.
    UnsupportedRole,
    /// The role version is zero or cannot be accepted as monotonic state.
    InvalidVersion,
    /// The role expiration is invalid for the supplied clock policy.
    Expired,
    /// A key identifier contains no meaningful identity.
    InvalidKeyId,
    /// A role threshold or key count is outside its declared bounds.
    InvalidRoleThreshold,
    /// A namespace is empty, too long, or not in canonical form.
    InvalidNamespace,
    /// A developer identifier is empty or too long.
    InvalidDeveloperId,
}

/// Validates common signed metadata fields.
pub fn validate_header(header: MetadataHeader, now: Option<u64>) -> Result<(), MetadataError> {
    if header.version == 0 {
        return Err(MetadataError::InvalidVersion);
    }
    if let (Some(now), expiration) = (now, header.expires)
        && expiration != 0
        && now > expiration
    {
        return Err(MetadataError::Expired);
    }
    Ok(())
}

/// Validates one role's bounded key list and signature threshold.
pub fn validate_role(role: RoleDefinition) -> Result<(), MetadataError> {
    let key_count = usize::from(role.key_count);
    let threshold = usize::from(role.threshold);
    if key_count == 0 || key_count > MAX_ROLE_KEYS || threshold == 0 || threshold > key_count {
        return Err(MetadataError::InvalidRoleThreshold);
    }
    for key in role.keys.iter().take(key_count) {
        if key.0 == [0; KEY_ID_LENGTH] {
            return Err(MetadataError::InvalidKeyId);
        }
    }
    Ok(())
}

/// Validates a developer identifier under the bounded ASCII-compatible rule.
pub fn validate_developer_id(value: &str) -> Result<(), MetadataError> {
    if value.is_empty()
        || value.len() > MAX_DEVELOPER_ID_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
    {
        return Err(MetadataError::InvalidDeveloperId);
    }
    Ok(())
}

/// Validates an exact or bounded namespace according to the initial profile.
pub fn validate_namespace(value: &str) -> Result<(), MetadataError> {
    if value.is_empty()
        || value.len() > MAX_NAMESPACE_BYTES
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
        return Err(MetadataError::InvalidNamespace);
    }
    Ok(())
}

/// Returns whether a role is one of the profile's repository roles.
pub const fn is_repository_role(role: MetadataRole) -> bool {
    matches!(
        role,
        MetadataRole::Root
            | MetadataRole::Timestamp
            | MetadataRole::Snapshot
            | MetadataRole::Targets
            | MetadataRole::Delegation
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyId, MetadataHeader, MetadataRole, RoleDefinition};

    #[test]
    fn accepts_a_valid_header_without_a_clock() {
        assert_eq!(
            validate_header(
                MetadataHeader {
                    role: MetadataRole::Targets,
                    version: 1,
                    expires: 0,
                },
                None,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_expired_metadata_when_a_clock_is_available() {
        assert_eq!(
            validate_header(
                MetadataHeader {
                    role: MetadataRole::Targets,
                    version: 1,
                    expires: 10,
                },
                Some(11),
            ),
            Err(MetadataError::Expired)
        );
    }

    #[test]
    fn validates_threshold_against_active_keys() {
        let mut keys = [KeyId([0; crate::KEY_ID_LENGTH]); MAX_ROLE_KEYS];
        keys[0] = KeyId([1; crate::KEY_ID_LENGTH]);
        assert_eq!(
            validate_role(RoleDefinition {
                role: MetadataRole::Targets,
                keys,
                key_count: 1,
                threshold: 1,
            }),
            Ok(())
        );
        assert_eq!(
            validate_role(RoleDefinition {
                role: MetadataRole::Targets,
                keys,
                key_count: 1,
                threshold: 2,
            }),
            Err(MetadataError::InvalidRoleThreshold)
        );
    }

    #[test]
    fn rejects_non_canonical_namespace_values() {
        assert_eq!(validate_namespace("developer/app"), Ok(()));
        assert_eq!(
            validate_namespace("Developer/app"),
            Err(MetadataError::InvalidNamespace)
        );
        assert_eq!(
            validate_namespace("developer//app"),
            Err(MetadataError::InvalidNamespace)
        );
    }

    #[test]
    fn validates_bounded_developer_identifiers() {
        assert_eq!(validate_developer_id("developer-1"), Ok(()));
        assert_eq!(
            validate_developer_id("Developer"),
            Err(MetadataError::InvalidDeveloperId)
        );
    }
}
