//! Hardware-neutral signature verification policy.

use crate::{
    PublicKey, RoleDefinition, RoleKey, Signature, SignatureSet, validate_role,
    validate_signature_set,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyId, MAX_ROLE_KEYS, MAX_SIGNATURES, MetadataRole, SignatureRecord};

    struct DeterministicVerifier;

    impl SignatureVerifier for DeterministicVerifier {
        fn verify(&self, public_key: PublicKey, message: &[u8], signature: Signature) -> bool {
            public_key.0[0] == message[0] && signature.0[0] == message[1]
        }
    }

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
            public_key: PublicKey([7; crate::PUBLIC_KEY_LENGTH]),
        }
    }

    #[test]
    fn verifies_an_authorized_signature_over_exact_bytes() {
        let mut records = [SignatureRecord::default(); MAX_SIGNATURES];
        records[0] = SignatureRecord {
            key_id: key().key_id,
            signature: Signature([9; crate::SIGNATURE_LENGTH]),
        };
        assert_eq!(
            verify_role_signatures(
                &DeterministicVerifier,
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
            signature: Signature([9; crate::SIGNATURE_LENGTH]),
        };
        let wrong_role_key = RoleKey {
            role: MetadataRole::Root,
            ..key()
        };
        assert_eq!(
            verify_role_signatures(
                &DeterministicVerifier,
                &[7, 9],
                role(),
                &[wrong_role_key],
                SignatureSet { records, count: 1 },
            ),
            Err(VerificationError::UnauthorizedSigner)
        );
    }
}
