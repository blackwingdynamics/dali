//! Hardware-neutral package authorization against a developer delegation.

use crate::{DelegationMetadata, TargetPackage, validate_delegation, validate_target_profile};

/// Errors returned when a target package is outside its delegation scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageAuthorizationError {
    /// The package or delegation violates its bounded contract.
    InvalidMetadata,
    /// The package refers to a different delegation record.
    DelegationMismatch,
    /// The package developer identity differs from the delegation.
    DeveloperMismatch,
    /// The package signing key differs from the delegated key.
    KeyMismatch,
    /// The package namespace is not authorized.
    NamespaceDenied,
    /// The package target profile is not authorized.
    TargetDenied,
    /// The package ABI is not authorized.
    AbiDenied,
    /// The package is outside the delegation validity interval.
    NotYetValid,
    /// The package delegation has expired.
    Expired,
}

/// Checks that a target package is authorized by one signed delegation.
pub fn authorize_target_package(
    package: &TargetPackage,
    delegation_id: &crate::BoundedText<{ crate::MAX_DELEGATION_ID_BYTES }>,
    delegation: &DelegationMetadata,
    now: Option<u64>,
) -> Result<(), PackageAuthorizationError> {
    validate_package(package)?;
    validate_delegation(delegation).map_err(|_| PackageAuthorizationError::InvalidMetadata)?;
    if package.delegation_id != *delegation_id {
        return Err(PackageAuthorizationError::DelegationMismatch);
    }
    if package.developer_id != delegation.developer_id {
        return Err(PackageAuthorizationError::DeveloperMismatch);
    }
    if package.developer_key_id != delegation.key_id {
        return Err(PackageAuthorizationError::KeyMismatch);
    }
    if !delegation.allowed_namespaces[..usize::from(delegation.namespace_count)]
        .contains(&package.namespace)
    {
        return Err(PackageAuthorizationError::NamespaceDenied);
    }
    if !delegation.allowed_targets[..usize::from(delegation.target_count)]
        .contains(&package.target_profile)
    {
        return Err(PackageAuthorizationError::TargetDenied);
    }
    if !delegation.allowed_abis[..usize::from(delegation.abi_count)].contains(&package.abi_version)
    {
        return Err(PackageAuthorizationError::AbiDenied);
    }
    validate_validity(delegation, now)
}

fn validate_package(package: &TargetPackage) -> Result<(), PackageAuthorizationError> {
    if package.package_id.0 == [0; crate::KEY_ID_LENGTH]
        || package.developer_key_id.0 == [0; crate::KEY_ID_LENGTH]
        || package.sha256.0 == [0; crate::SHA256_LENGTH]
        || package.length == 0
        || package.amrn_format == 0
        || package.abi_version == 0
    {
        return Err(PackageAuthorizationError::InvalidMetadata);
    }
    let namespace = package
        .namespace
        .as_str()
        .ok_or(PackageAuthorizationError::InvalidMetadata)?;
    crate::validate_namespace(namespace).map_err(|_| PackageAuthorizationError::InvalidMetadata)?;
    validate_target_profile(
        package
            .target_profile
            .as_str()
            .ok_or(PackageAuthorizationError::InvalidMetadata)?,
    )
    .map_err(|_| PackageAuthorizationError::InvalidMetadata)?;
    if package.developer_id.as_str().is_none()
        || package.delegation_id.as_str().is_none()
        || package.package_version.as_str().is_none()
        || package.minimum_kernel_version.as_str().is_none()
    {
        return Err(PackageAuthorizationError::InvalidMetadata);
    }
    Ok(())
}

fn validate_validity(
    delegation: &DelegationMetadata,
    now: Option<u64>,
) -> Result<(), PackageAuthorizationError> {
    let Some(now) = now else {
        return Ok(());
    };
    if delegation.not_before != 0 && now < delegation.not_before {
        return Err(PackageAuthorizationError::NotYetValid);
    }
    if delegation.not_after != 0 && now > delegation.not_after {
        return Err(PackageAuthorizationError::Expired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundedText, KeyId, MetadataHeader, MetadataRole, PublicKey, Sha256Digest};

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

    fn package() -> TargetPackage {
        TargetPackage {
            package_id: crate::PackageId([1; crate::KEY_ID_LENGTH]),
            namespace: text("developer/app"),
            developer_id: text("developer"),
            delegation_id: text("delegation-1"),
            developer_key_id: KeyId([8; crate::KEY_ID_LENGTH]),
            target_profile: text("f405"),
            amrn_format: 5,
            abi_version: 3,
            package_version: text("1.0.0"),
            minimum_kernel_version: text("0.1.0"),
            length: 256,
            sha256: Sha256Digest([2; crate::SHA256_LENGTH]),
            required_services: 0,
            slot_id: 0,
        }
    }

    #[test]
    fn accepts_a_package_inside_the_delegation_scope() {
        assert_eq!(
            authorize_target_package(&package(), &text("delegation-1"), &delegation(), Some(150)),
            Ok(())
        );
    }

    #[test]
    fn rejects_a_package_outside_the_namespace_scope() {
        let mut package = package();
        package.namespace = text("other/app");
        assert_eq!(
            authorize_target_package(&package, &text("delegation-1"), &delegation(), Some(150)),
            Err(PackageAuthorizationError::NamespaceDenied)
        );
    }

    #[test]
    fn rejects_packages_before_and_after_delegation_validity() {
        assert_eq!(
            authorize_target_package(&package(), &text("delegation-1"), &delegation(), Some(99)),
            Err(PackageAuthorizationError::NotYetValid)
        );
        assert_eq!(
            authorize_target_package(&package(), &text("delegation-1"), &delegation(), Some(201)),
            Err(PackageAuthorizationError::Expired)
        );
    }

    #[test]
    fn rejects_a_different_developer_key() {
        let mut package = package();
        package.developer_key_id = KeyId([7; crate::KEY_ID_LENGTH]);
        assert_eq!(
            authorize_target_package(&package, &text("delegation-1"), &delegation(), Some(150)),
            Err(PackageAuthorizationError::KeyMismatch)
        );
    }
}
