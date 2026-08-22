//! Root-key custody and Secure Boot admission contracts.

mod descriptor;
#[cfg(test)]
mod tests;

pub use descriptor::*;

use crate::{
    BoundedText, MetadataRole, RootMetadata, SignatureSet, SignatureVerifier, VerificationError,
    verify_role_signatures,
};
use sha2::{Digest, Sha256};

/// Versioned identifier for the bounded Secure Boot descriptor contract.
pub const SECURE_BOOT_CONTRACT_ID: &str = "dali.secure-boot.v1";
/// Minimum independently held root keys required by the production profile.
pub const PRODUCTION_ROOT_KEY_MINIMUM: u8 = 3;
/// Root signature threshold required by the production profile.
pub const PRODUCTION_ROOT_THRESHOLD: u8 = 2;
/// Root-key custody profile enforced by the metadata policy layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootKeyCustodyPolicy {
    /// One non-production root key is permitted for development.
    Development,
    /// Production requires independent custody and a two-of-three threshold.
    Production,
}

impl RootKeyCustodyPolicy {
    /// Returns the development policy used by local fixtures.
    pub const fn development() -> Self {
        Self::Development
    }

    /// Returns the production policy defined by the distribution contract.
    pub const fn production() -> Self {
        Self::Production
    }

    /// Returns the minimum root-key count required by this profile.
    pub const fn minimum_root_keys(self) -> u8 {
        match self {
            Self::Development => 1,
            Self::Production => PRODUCTION_ROOT_KEY_MINIMUM,
        }
    }

    /// Returns the root signature threshold required by this profile.
    pub const fn threshold(self) -> u8 {
        match self {
            Self::Development => 1,
            Self::Production => PRODUCTION_ROOT_THRESHOLD,
        }
    }

    /// Indicates that private root keys must remain off-target.
    pub const fn private_keys_on_target_allowed(self) -> bool {
        false
    }

    /// Validates root metadata against the selected custody profile.
    pub fn validate(self, root: &RootMetadata) -> Result<(), RootKeyCustodyError> {
        let key_count = usize::from(root.key_count);
        let role_count = usize::from(root.role_count);
        if root.header.role != MetadataRole::Root
            || root.header.version == 0
            || key_count > crate::MAX_ROOT_KEYS
            || role_count > crate::MAX_ROOT_ROLES
        {
            return Err(RootKeyCustodyError::InvalidRootMetadata);
        }
        let mut root_key_count = 0_usize;
        for (index, key) in root.keys[..key_count].iter().enumerate() {
            if key.role != MetadataRole::Root {
                continue;
            }
            if key.key_id.0 == [0; crate::KEY_ID_LENGTH]
                || key.public_key.0 == [0; crate::PUBLIC_KEY_LENGTH]
                || root.keys[..index].iter().any(|previous| {
                    previous.role == MetadataRole::Root
                        && (previous.key_id == key.key_id || previous.public_key == key.public_key)
                })
            {
                return Err(RootKeyCustodyError::DuplicateRootKey);
            }
            root_key_count += 1;
        }
        if root_key_count < usize::from(self.minimum_root_keys()) {
            return Err(RootKeyCustodyError::InsufficientRootKeys);
        }
        let role = root.roles[..role_count]
            .iter()
            .find(|role| role.role == MetadataRole::Root)
            .ok_or(RootKeyCustodyError::MissingRootRole)?;
        if role.threshold != self.threshold()
            || usize::from(role.key_count) != root_key_count
            || usize::from(role.key_count) > crate::MAX_ROLE_KEYS
            || role
                .keys
                .iter()
                .take(usize::from(role.key_count))
                .enumerate()
                .any(|(index, key_id)| {
                    key_id.0 == [0; crate::KEY_ID_LENGTH]
                        || role.keys[..index].contains(key_id)
                        || !root.keys[..key_count]
                            .iter()
                            .any(|key| key.role == MetadataRole::Root && key.key_id == *key_id)
                })
        {
            return Err(RootKeyCustodyError::InvalidRootThreshold);
        }
        Ok(())
    }
}

/// Errors raised by root-key custody validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootKeyCustodyError {
    /// The root header or bounded list counts are invalid.
    InvalidRootMetadata,
    /// The profile does not have enough distinct root keys.
    InsufficientRootKeys,
    /// Root identifiers or public keys are duplicated or empty.
    DuplicateRootKey,
    /// Root metadata does not declare a root role.
    MissingRootRole,
    /// The root role threshold or membership does not match the custody profile.
    InvalidRootThreshold,
}

/// Kernel image and root signatures submitted to the Secure Boot contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SecureBootImage<'a> {
    /// Descriptor whose canonical bytes are signed by the root role.
    pub descriptor: KernelImageDescriptor,
    /// Complete kernel image bytes.
    pub image: &'a [u8],
    /// Root-role signatures over `descriptor.encode()`.
    pub signatures: SignatureSet,
}

/// Secure Boot admission policy for one target profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SecureBootContract {
    /// Target profile accepted by this contract.
    pub target_profile: BoundedText<{ crate::MAX_TARGET_PROFILE_BYTES }>,
    /// Root custody profile authorizing kernel images.
    pub root_custody: RootKeyCustodyPolicy,
    /// Whether an image must carry a valid root signature set.
    pub require_signature: bool,
    /// Whether image versions must advance the durable version.
    pub reject_rollback: bool,
}

impl SecureBootContract {
    /// Creates the production admission contract for a target profile.
    pub const fn production(
        target_profile: BoundedText<{ crate::MAX_TARGET_PROFILE_BYTES }>,
    ) -> Self {
        Self {
            target_profile,
            root_custody: RootKeyCustodyPolicy::Production,
            require_signature: true,
            reject_rollback: true,
        }
    }
}

/// Errors returned before a kernel image can be admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecureBootError {
    /// Root metadata violates the selected custody policy.
    RootCustody(RootKeyCustodyError),
    /// The image descriptor is not valid for the supplied image.
    InvalidImageDescriptor,
    /// The image targets a different profile.
    TargetMismatch,
    /// The image version is not newer than the durable version.
    Rollback,
    /// The descriptor digest does not match the image bytes.
    DigestMismatch,
    /// The descriptor length does not match the image bytes.
    LengthMismatch,
    /// The root role is absent when a signature is required.
    MissingRootRole,
    /// Root signatures failed threshold or cryptographic verification.
    Signature(VerificationError),
}

/// Verifies a kernel image against the production Secure Boot contract.
pub fn verify_secure_boot_image<V: SignatureVerifier>(
    contract: SecureBootContract,
    verifier: &V,
    root: &RootMetadata,
    image: SecureBootImage<'_>,
    previous_version: Option<u64>,
) -> Result<(), SecureBootError> {
    contract
        .root_custody
        .validate(root)
        .map_err(SecureBootError::RootCustody)?;
    if image.descriptor.target_profile != contract.target_profile {
        return Err(SecureBootError::TargetMismatch);
    }
    if image.descriptor.version == 0 || image.descriptor.length == 0 {
        return Err(SecureBootError::InvalidImageDescriptor);
    }
    if contract.reject_rollback
        && previous_version.is_some_and(|version| image.descriptor.version <= version)
    {
        return Err(SecureBootError::Rollback);
    }
    if usize::try_from(image.descriptor.length).ok() != Some(image.image.len()) {
        return Err(SecureBootError::LengthMismatch);
    }
    if image.descriptor.sha256.0 != Sha256::digest(image.image)[..] {
        return Err(SecureBootError::DigestMismatch);
    }
    if contract.require_signature {
        let role = root
            .roles
            .iter()
            .take(usize::from(root.role_count))
            .find(|role| role.role == MetadataRole::Root)
            .ok_or(SecureBootError::MissingRootRole)?;
        verify_role_signatures(
            verifier,
            &image.descriptor.encode(),
            *role,
            &root.keys[..usize::from(root.key_count)],
            image.signatures,
        )
        .map_err(SecureBootError::Signature)?;
    }
    Ok(())
}
