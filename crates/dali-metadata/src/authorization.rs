//! Hardware-neutral cartridge authorization against a developer delegation.

use crate::{
    DelegationMetadata, KeyId, RevocationMetadata, TargetCartridge, validate_delegation,
    validate_revocation_metadata, validate_target_profile,
};

/// Errors returned when a target cartridge is outside its delegation scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CartridgeAuthorizationError {
    /// The cartridge or delegation violates its bounded contract.
    InvalidMetadata,
    /// The cartridge refers to a different delegation record.
    DelegationMismatch,
    /// The cartridge developer identity differs from the delegation.
    DeveloperMismatch,
    /// The cartridge signing key differs from the delegated key.
    KeyMismatch,
    /// The cartridge namespace is not authorized.
    NamespaceDenied,
    /// The cartridge target profile is not authorized.
    TargetDenied,
    /// The cartridge ABI is not authorized.
    AbiDenied,
    /// The cartridge is outside the delegation validity interval.
    NotYetValid,
    /// The cartridge delegation has expired.
    Expired,
    /// The developer signing key is revoked for this repository generation.
    RevokedKey,
}

/// Checks that a target cartridge is authorized by one signed delegation.
pub fn authorize_target_cartridge(
    cartridge: &TargetCartridge,
    delegation_id: &crate::BoundedText<{ crate::MAX_DELEGATION_ID_BYTES }>,
    delegation: &DelegationMetadata,
    now: Option<u64>,
) -> Result<(), CartridgeAuthorizationError> {
    validate_cartridge(cartridge)?;
    validate_delegation(delegation).map_err(|_| CartridgeAuthorizationError::InvalidMetadata)?;
    if cartridge.delegation_id != *delegation_id {
        return Err(CartridgeAuthorizationError::DelegationMismatch);
    }
    if cartridge.developer_id != delegation.developer_id {
        return Err(CartridgeAuthorizationError::DeveloperMismatch);
    }
    if cartridge.developer_key_id != delegation.key_id {
        return Err(CartridgeAuthorizationError::KeyMismatch);
    }
    if !delegation.allowed_namespaces[..usize::from(delegation.namespace_count)]
        .contains(&cartridge.namespace)
    {
        return Err(CartridgeAuthorizationError::NamespaceDenied);
    }
    if !delegation.allowed_targets[..usize::from(delegation.target_count)]
        .contains(&cartridge.target_profile)
    {
        return Err(CartridgeAuthorizationError::TargetDenied);
    }
    if !delegation.allowed_abis[..usize::from(delegation.abi_count)]
        .contains(&cartridge.abi_version)
    {
        return Err(CartridgeAuthorizationError::AbiDenied);
    }
    validate_validity(delegation, now)
}

/// Authorizes a cartridge while applying the signed revocation document.
pub fn authorize_target_cartridge_with_revocations(
    cartridge: &TargetCartridge,
    delegation_id: &crate::BoundedText<{ crate::MAX_DELEGATION_ID_BYTES }>,
    delegation: &DelegationMetadata,
    revocations: &RevocationMetadata,
    repository_version: u64,
    now: Option<u64>,
) -> Result<(), CartridgeAuthorizationError> {
    authorize_target_cartridge(cartridge, delegation_id, delegation, now)?;
    if is_developer_key_revoked(
        revocations,
        &cartridge.developer_id,
        cartridge.developer_key_id,
        repository_version,
    )? {
        return Err(CartridgeAuthorizationError::RevokedKey);
    }
    Ok(())
}

/// Returns whether a developer key is revoked at a repository generation.
pub fn is_developer_key_revoked(
    metadata: &RevocationMetadata,
    developer_id: &crate::BoundedText<{ crate::MAX_DEVELOPER_ID_BYTES }>,
    key_id: KeyId,
    repository_version: u64,
) -> Result<bool, CartridgeAuthorizationError> {
    validate_revocation_metadata(metadata)
        .map_err(|_| CartridgeAuthorizationError::InvalidMetadata)?;
    Ok(metadata.records[..usize::from(metadata.record_count)]
        .iter()
        .any(|record| {
            record.developer_id == *developer_id
                && record.key_id == key_id
                && repository_version >= record.effective_version
        }))
}

fn validate_cartridge(cartridge: &TargetCartridge) -> Result<(), CartridgeAuthorizationError> {
    if cartridge.cartridge_id.0 == [0; crate::KEY_ID_LENGTH]
        || cartridge.developer_key_id.0 == [0; crate::KEY_ID_LENGTH]
        || cartridge.sha256.0 == [0; crate::SHA256_LENGTH]
        || cartridge.length == 0
        || cartridge.amrn_format == 0
        || cartridge.abi_version == 0
    {
        return Err(CartridgeAuthorizationError::InvalidMetadata);
    }
    let namespace = cartridge
        .namespace
        .as_str()
        .ok_or(CartridgeAuthorizationError::InvalidMetadata)?;
    crate::validate_namespace(namespace)
        .map_err(|_| CartridgeAuthorizationError::InvalidMetadata)?;
    validate_target_profile(
        cartridge
            .target_profile
            .as_str()
            .ok_or(CartridgeAuthorizationError::InvalidMetadata)?,
    )
    .map_err(|_| CartridgeAuthorizationError::InvalidMetadata)?;
    if cartridge.developer_id.as_str().is_none()
        || cartridge.delegation_id.as_str().is_none()
        || cartridge.cartridge_version.as_str().is_none()
        || cartridge.minimum_kernel_version.as_str().is_none()
    {
        return Err(CartridgeAuthorizationError::InvalidMetadata);
    }
    Ok(())
}

fn validate_validity(
    delegation: &DelegationMetadata,
    now: Option<u64>,
) -> Result<(), CartridgeAuthorizationError> {
    let Some(now) = now else {
        return Ok(());
    };
    if delegation.not_before != 0 && now < delegation.not_before {
        return Err(CartridgeAuthorizationError::NotYetValid);
    }
    if delegation.not_after != 0 && now > delegation.not_after {
        return Err(CartridgeAuthorizationError::Expired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BoundedText, KeyId, MetadataHeader, MetadataRole, PublicKey, RevocationMetadata,
        RevocationRecord, Sha256Digest,
    };

    fn text<const CAPACITY: usize>(value: &str) -> BoundedText<CAPACITY> {
        BoundedText::new(value).expect("test text fits")
    }

    fn delegation() -> DelegationMetadata {
        let mut namespaces = [BoundedText::default(); crate::MAX_DELEGATION_SCOPES];
        namespaces[0] = text("developer/app");
        let mut targets = [BoundedText::default(); crate::MAX_DELEGATION_TARGETS];
        targets[0] = text("f405");
        let mut abis = [0; crate::MAX_DELEGATION_ABIS];
        abis[0] = 3;
        DelegationMetadata {
            header: MetadataHeader {
                role: MetadataRole::Delegation,
                version: 1,
                expires: 0,
            },
            developer_id: text("developer"),
            key_id: KeyId([8; crate::KEY_ID_LENGTH]),
            public_key: PublicKey([9; crate::PUBLIC_KEY_LENGTH]),
            allowed_namespaces: namespaces,
            namespace_count: 1,
            allowed_targets: targets,
            target_count: 1,
            allowed_abis: abis,
            abi_count: 1,
            not_before: 100,
            not_after: 200,
        }
    }

    fn cartridge() -> TargetCartridge {
        TargetCartridge {
            cartridge_id: crate::CartridgeId([1; crate::KEY_ID_LENGTH]),
            namespace: text("developer/app"),
            developer_id: text("developer"),
            delegation_id: text("delegation-1"),
            developer_key_id: KeyId([8; crate::KEY_ID_LENGTH]),
            target_profile: text("f405"),
            amrn_format: 5,
            abi_version: 3,
            cartridge_version: text("1.0.0"),
            minimum_kernel_version: text("0.1.0"),
            length: 256,
            sha256: Sha256Digest([2; crate::SHA256_LENGTH]),
            required_services: 0,
            slot_id: 0,
        }
    }

    #[test]
    fn accepts_a_cartridge_inside_the_delegation_scope() {
        assert_eq!(
            authorize_target_cartridge(
                &cartridge(),
                &text("delegation-1"),
                &delegation(),
                Some(150)
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_a_cartridge_outside_the_namespace_scope() {
        let mut cartridge = cartridge();
        cartridge.namespace = text("other/app");
        assert_eq!(
            authorize_target_cartridge(&cartridge, &text("delegation-1"), &delegation(), Some(150)),
            Err(CartridgeAuthorizationError::NamespaceDenied)
        );
    }

    #[test]
    fn rejects_cartridges_before_and_after_delegation_validity() {
        assert_eq!(
            authorize_target_cartridge(
                &cartridge(),
                &text("delegation-1"),
                &delegation(),
                Some(99)
            ),
            Err(CartridgeAuthorizationError::NotYetValid)
        );
        assert_eq!(
            authorize_target_cartridge(
                &cartridge(),
                &text("delegation-1"),
                &delegation(),
                Some(201)
            ),
            Err(CartridgeAuthorizationError::Expired)
        );
    }

    #[test]
    fn rejects_a_different_developer_key() {
        let mut cartridge = cartridge();
        cartridge.developer_key_id = KeyId([7; crate::KEY_ID_LENGTH]);
        assert_eq!(
            authorize_target_cartridge(&cartridge, &text("delegation-1"), &delegation(), Some(150)),
            Err(CartridgeAuthorizationError::KeyMismatch)
        );
    }

    #[test]
    fn rejects_a_cartridge_after_its_developer_key_is_revoked() {
        let mut records = [RevocationRecord::default(); crate::MAX_REVOCATIONS];
        records[0] = RevocationRecord {
            developer_id: text("developer"),
            effective_version: 2,
            issuer_key_id: KeyId([1; crate::KEY_ID_LENGTH]),
            key_id: KeyId([8; crate::KEY_ID_LENGTH]),
            reason: text("compromised"),
        };
        let revocations = RevocationMetadata {
            header: MetadataHeader {
                role: MetadataRole::Revocation,
                version: 1,
                expires: 0,
            },
            records,
            record_count: 1,
        };
        assert_eq!(
            authorize_target_cartridge_with_revocations(
                &cartridge(),
                &text("delegation-1"),
                &delegation(),
                &revocations,
                2,
                Some(150),
            ),
            Err(CartridgeAuthorizationError::RevokedKey)
        );
    }
}
