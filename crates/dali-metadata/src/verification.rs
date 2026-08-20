//! Hardware-neutral signature verification policy.

use crate::{
    DelegationMetadata, MetadataRole, PublicKey, RoleDefinition, RoleKey, RootMetadata, Signature,
    SignatureSet, validate_delegation, validate_role, validate_signature_set,
};

/// Errors returned when a signature set cannot satisfy a role policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationError {
    /// The role policy is malformed.
    InvalidRole,
    /// The signature set is malformed or not in canonical order.
    InvalidSignatureSet,
    /// A signature refers to no declared metadata key.
    UnknownSigner,
    /// A declared key is not authorized for the selected role.
    UnauthorizedSigner,
    /// A declared signer key failed cryptographic verification.
    InvalidSignature,
    /// Fewer valid signatures were provided than the role threshold requires.
    ThresholdNotMet,
    /// Root metadata does not declare the requested role.
    MissingRole,
    /// The delegated record is not valid under the bounded contract.
    InvalidDelegation,
}

/// Cryptographic backend required by the metadata policy layer.
pub trait SignatureVerifier {
    /// Verifies one signature over the exact canonical signed bytes.
    fn verify(&self, public_key: PublicKey, message: &[u8], signature: Signature) -> bool;
}

/// Verifies a signature set against one role's authorized keys and threshold.
pub fn verify_role_signatures<V: SignatureVerifier>(
    verifier: &V,
    message: &[u8],
    role: RoleDefinition,
    keys: &[RoleKey],
    signatures: SignatureSet,
) -> Result<(), VerificationError> {
    validate_role(role).map_err(|_| VerificationError::InvalidRole)?;
    validate_signature_set(&signatures).map_err(|_| VerificationError::InvalidSignatureSet)?;
    let active_signatures = &signatures.records[..usize::from(signatures.count)];
    let mut valid = 0_usize;
    for record in active_signatures {
        let key = keys
            .iter()
            .find(|key| key.key_id == record.key_id)
            .ok_or(VerificationError::UnknownSigner)?;
        if key.role != role.role {
            return Err(VerificationError::UnauthorizedSigner);
        }
        if !role.keys[..usize::from(role.key_count)]
            .iter()
            .any(|key_id| key_id == &record.key_id)
        {
            return Err(VerificationError::UnauthorizedSigner);
        }
        if !verifier.verify(key.public_key, message, record.signature) {
            return Err(VerificationError::InvalidSignature);
        }
        valid += 1;
    }
    if valid < usize::from(role.threshold) {
        return Err(VerificationError::ThresholdNotMet);
    }
    Ok(())
}

/// Verifies a developer delegation against the root-declared delegation role.
pub fn verify_delegation_signatures<V: SignatureVerifier>(
    verifier: &V,
    message: &[u8],
    root: RootMetadata,
    delegation: DelegationMetadata,
    signatures: SignatureSet,
) -> Result<(), VerificationError> {
    validate_delegation(&delegation).map_err(|_| VerificationError::InvalidDelegation)?;
    let role = root
        .roles
        .iter()
        .take(usize::from(root.role_count))
        .find(|role| role.role == MetadataRole::Delegation)
        .ok_or(VerificationError::MissingRole)?;
    verify_role_signatures(
        verifier,
        message,
        *role,
        &root.keys[..usize::from(root.key_count)],
        signatures,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyId, MAX_ROLE_KEYS, MAX_SIGNATURES, MetadataRole, SignatureRecord};

    const SEED: [u8; 32] = [7; 32];

    fn role() -> RoleDefinition {
        let mut keys = [KeyId([0; crate::KEY_ID_LENGTH]); MAX_ROLE_KEYS];
        keys[0] = KeyId([1; crate::KEY_ID_LENGTH]);
        RoleDefinition {
            role: MetadataRole::Targets,
            keys,
            key_count: 1,
            threshold: 1,
        }
    }

    fn key() -> RoleKey {
        RoleKey {
            role: MetadataRole::Targets,
            key_id: KeyId([1; crate::KEY_ID_LENGTH]),
            public_key: PublicKey(dali_crypto::public_key_from_seed(&SEED)),
        }
    }

    fn delegation() -> DelegationMetadata {
        let mut namespaces = [crate::BoundedText::default(); crate::MAX_DELEGATION_SCOPES];
        namespaces[0] = crate::BoundedText::new("developer/app").expect("namespace fits");
        let mut targets = [crate::BoundedText::default(); crate::MAX_DELEGATION_TARGETS];
        targets[0] = crate::BoundedText::new("f405").expect("target fits");
        let mut abis = [0; crate::MAX_DELEGATION_ABIS];
        abis[0] = 3;
        DelegationMetadata {
            header: crate::MetadataHeader {
                role: MetadataRole::Delegation,
                version: 1,
                expires: 0,
            },
            developer_id: crate::BoundedText::new("developer").expect("developer fits"),
            key_id: KeyId([8; crate::KEY_ID_LENGTH]),
            public_key: PublicKey([9; crate::PUBLIC_KEY_LENGTH]),
            allowed_namespaces: namespaces,
            namespace_count: 1,
            allowed_targets: targets,
            target_count: 1,
            allowed_abis: abis,
            abi_count: 1,
            not_before: 0,
            not_after: 0,
        }
    }

    #[test]
    fn verifies_an_authorized_signature_over_exact_bytes() {
        let mut records = [SignatureRecord::default(); MAX_SIGNATURES];
        records[0] = SignatureRecord {
            key_id: key().key_id,
            signature: Signature(dali_crypto::sign(&SEED, &[7, 9, 0])),
        };
        assert_eq!(
            verify_role_signatures(
                &crate::Ed25519Verifier,
                &[7, 9, 0],
                role(),
                &[key()],
                SignatureSet { records, count: 1 },
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_a_signer_from_another_role() {
        let mut records = [SignatureRecord::default(); MAX_SIGNATURES];
        records[0] = SignatureRecord {
            key_id: key().key_id,
            signature: Signature(dali_crypto::sign(&SEED, &[7, 9])),
        };
        let wrong_role_key = RoleKey {
            role: MetadataRole::Root,
            ..key()
        };
        assert_eq!(
            verify_role_signatures(
                &crate::Ed25519Verifier,
                &[7, 9],
                role(),
                &[wrong_role_key],
                SignatureSet { records, count: 1 },
            ),
            Err(VerificationError::UnauthorizedSigner)
        );
    }

    #[test]
    fn verifies_a_delegation_against_the_root_role() {
        let key_id = KeyId([1; crate::KEY_ID_LENGTH]);
        let root = RootMetadata {
            header: crate::MetadataHeader {
                role: MetadataRole::Root,
                version: 1,
                expires: 0,
            },
            keys: [RoleKey {
                role: MetadataRole::Delegation,
                key_id,
                public_key: PublicKey(dali_crypto::public_key_from_seed(&SEED)),
            }; crate::MAX_ROOT_KEYS],
            key_count: 1,
            roles: [RoleDefinition {
                role: MetadataRole::Delegation,
                keys: [key_id; crate::MAX_ROLE_KEYS],
                key_count: 1,
                threshold: 1,
            }; crate::MAX_ROOT_ROLES],
            role_count: 1,
        };
        let mut records = [SignatureRecord::default(); MAX_SIGNATURES];
        records[0] = SignatureRecord {
            key_id,
            signature: Signature(dali_crypto::sign(&SEED, &[7, 9])),
        };
        assert_eq!(
            verify_delegation_signatures(
                &crate::Ed25519Verifier,
                &[7, 9],
                root,
                delegation(),
                SignatureSet { records, count: 1 },
            ),
            Ok(())
        );
    }
}
