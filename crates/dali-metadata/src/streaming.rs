//! Bounded cryptographic verification for Binary Metadata v2 streams.

use core::mem::MaybeUninit;
use core::ptr::addr_of_mut;

use crate::{
    KeyId, MetadataRole, RoleDefinition, RoleKey, SignatureSet, validate_role,
    validate_signature_set,
};

/// Maximum number of Ed25519 signatures authenticated by one role envelope.
pub const MAX_STREAMING_SIGNERS: usize = crate::MAX_SIGNATURES;
/// Maximum cryptographic update interval between progress callbacks.
pub const STREAMING_CRYPTO_PROGRESS_BYTES: usize = 64;

/// Errors returned while preparing or completing one signed role stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingVerificationError {
    /// The role definition is malformed.
    InvalidRole,
    /// The envelope signature set is malformed.
    InvalidSignatureSet,
    /// A signature references no declared key.
    UnknownSigner,
    /// A key exists but is not authorized for this role.
    UnauthorizedSigner,
    /// A key or signature cannot initialize the Ed25519 verifier.
    InvalidKey,
    /// A signer did not authenticate the complete streamed body.
    InvalidSignature,
    /// The role threshold was not met.
    ThresholdNotMet,
}

/// Error returned when a bounded cryptographic progress callback aborts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingProgressError {
    /// The caller could not service its progress boundary.
    Aborted,
}

/// Result of one verified role body stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamVerificationResult {
    /// SHA-256 digest of the exact body bytes supplied to [`StreamingRoleVerifier::update`].
    pub sha256: crate::Sha256Digest,
    /// Number of distinct valid signatures satisfying the role threshold.
    pub valid_signatures: u8,
}

/// Incrementally authenticates one canonical role body.
///
/// The verifier owns only SHA-256 state and one Ed25519 state per signature
/// record. Body chunks are borrowed for the duration of [`Self::update`].
pub struct StreamingRoleVerifier {
    digest: dali_crypto::Sha256Accumulator,
    verifiers: [MaybeUninit<dali_crypto::StreamingVerifier>; MAX_STREAMING_SIGNERS],
    signature_count: usize,
    threshold: u8,
}

impl StreamingRoleVerifier {
    /// Prepares verification for a role using its already trusted key set.
    pub fn new(
        role: RoleDefinition,
        keys: &[RoleKey],
        signatures: SignatureSet,
    ) -> Result<Self, StreamingVerificationError> {
        validate_role(role).map_err(|_| StreamingVerificationError::InvalidRole)?;
        validate_signature_set(&signatures)
            .map_err(|_| StreamingVerificationError::InvalidSignatureSet)?;
        let mut verifiers = core::array::from_fn(|_| None);
        let active = &signatures.records[..usize::from(signatures.count)];
        for (index, record) in active.iter().enumerate() {
            let key =
                find_key(keys, record.key_id).ok_or(StreamingVerificationError::UnknownSigner)?;
            if key.role != role.role
                || !role.keys[..usize::from(role.key_count)].contains(&record.key_id)
            {
                return Err(StreamingVerificationError::UnauthorizedSigner);
            }
            verifiers[index] = Some(
                dali_crypto::begin_verify(&key.public_key.0, &record.signature.0)
                    .map_err(|_| StreamingVerificationError::InvalidKey)?,
            );
        }
        Ok(Self {
            digest: dali_crypto::Sha256Accumulator::new(),
            verifiers: verifiers.map(|verifier| {
                let mut slot = MaybeUninit::uninit();
                if let Some(verifier) = verifier {
                    slot.write(verifier);
                }
                slot
            }),
            signature_count: active.len(),
            threshold: role.threshold,
        })
    }

    /// Initializes a verifier directly in caller-owned storage.
    ///
    /// This form is intended for embedded callers whose stack cannot hold the
    /// Ed25519 backend state. The destination must not be read unless this
    /// function returns `Ok(())`.
    #[inline(never)]
    pub fn initialize(
        destination: &mut MaybeUninit<Self>,
        role: RoleDefinition,
        keys: &[RoleKey],
        signatures: SignatureSet,
    ) -> Result<(), StreamingVerificationError> {
        validate_role(role).map_err(|_| StreamingVerificationError::InvalidRole)?;
        validate_signature_set(&signatures)
            .map_err(|_| StreamingVerificationError::InvalidSignatureSet)?;
        let active = &signatures.records[..usize::from(signatures.count)];
        let destination = destination.as_mut_ptr();
        unsafe {
            addr_of_mut!((*destination).digest).write(dali_crypto::Sha256Accumulator::new());
            addr_of_mut!((*destination).verifiers)
                .write(core::array::from_fn(|_| MaybeUninit::uninit()));
            addr_of_mut!((*destination).signature_count).write(active.len());
            addr_of_mut!((*destination).threshold).write(role.threshold);
        }
        for (index, record) in active.iter().enumerate() {
            let key =
                find_key(keys, record.key_id).ok_or(StreamingVerificationError::UnknownSigner)?;
            if key.role != role.role
                || !role.keys[..usize::from(role.key_count)].contains(&record.key_id)
            {
                return Err(StreamingVerificationError::UnauthorizedSigner);
            }
            let verifier = dali_crypto::begin_verify(&key.public_key.0, &record.signature.0)
                .map_err(|_| StreamingVerificationError::InvalidKey)?;
            unsafe {
                (*destination).verifiers[index].write(verifier);
            }
        }
        Ok(())
    }

    /// Adds one body chunk to both the digest and every authorized signature verifier.
    pub fn update(&mut self, chunk: &[u8]) {
        self.digest.update(chunk);
        for verifier in &mut self.verifiers[..self.signature_count] {
            // SAFETY: initialize() and new() initialize every active slot.
            unsafe { verifier.assume_init_mut() }.update(chunk);
        }
    }

    /// Adds a body chunk while reporting bounded cryptographic progress.
    ///
    /// The callback runs only after each non-empty sub-chunk has been applied
    /// to the digest and all active signature states. Returning `false`
    /// aborts the update before another sub-chunk is processed.
    pub fn update_with_progress<F>(
        &mut self,
        chunk: &[u8],
        mut progress: F,
    ) -> Result<(), StreamingProgressError>
    where
        F: FnMut() -> bool,
    {
        for part in chunk.chunks(STREAMING_CRYPTO_PROGRESS_BYTES) {
            self.digest.update(part);
            for verifier in &mut self.verifiers[..self.signature_count] {
                // SAFETY: initialize() and new() initialize every active slot.
                unsafe { verifier.assume_init_mut() }.update(part);
            }
            if !progress() {
                return Err(StreamingProgressError::Aborted);
            }
        }
        Ok(())
    }

    /// Finalizes all cryptographic states and returns the authenticated body digest.
    pub fn finish(mut self) -> Result<StreamVerificationResult, StreamingVerificationError> {
        self.finish_in_place()
    }

    /// Finalizes the verifier without moving its large cryptographic state.
    ///
    /// Embedded callers should prefer this method when the verifier lives in
    /// caller-owned BSS storage.
    pub fn finish_in_place(
        &mut self,
    ) -> Result<StreamVerificationResult, StreamingVerificationError> {
        let mut valid_signatures = 0_u8;
        for index in 0..self.signature_count {
            // SAFETY: every active slot was initialized by initialize() or new(),
            // and is consumed exactly once here.
            let verifier = unsafe { self.verifiers[index].assume_init_read() };
            if verifier.finalize().is_ok() {
                valid_signatures = valid_signatures.saturating_add(1);
            } else {
                return Err(StreamingVerificationError::InvalidSignature);
            }
        }
        if valid_signatures < self.threshold {
            return Err(StreamingVerificationError::ThresholdNotMet);
        }
        Ok(StreamVerificationResult {
            sha256: crate::Sha256Digest(
                core::mem::replace(&mut self.digest, dali_crypto::Sha256Accumulator::new())
                    .finalize(),
            ),
            valid_signatures,
        })
    }
}

fn find_key(keys: &[RoleKey], key_id: KeyId) -> Option<RoleKey> {
    keys.iter().find(|key| key.key_id == key_id).copied()
}

/// Keeps the role binding explicit when a parser is fed from a generic envelope stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamingRoleBinding {
    /// Role expected in the Binary Metadata v2 envelope.
    pub role: MetadataRole,
}

impl StreamingRoleBinding {
    /// Creates a binding for one metadata role.
    pub const fn new(role: MetadataRole) -> Self {
        Self { role }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyId, RoleKey, Signature, SignatureRecord};

    const SEED: [u8; 32] = [19; 32];

    fn role_and_key() -> (RoleDefinition, RoleKey) {
        let key_id = KeyId([3; crate::KEY_ID_LENGTH]);
        let mut allowed = [KeyId([0; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS];
        allowed[0] = key_id;
        (
            RoleDefinition {
                role: MetadataRole::Targets,
                keys: allowed,
                key_count: 1,
                threshold: 1,
            },
            RoleKey {
                role: MetadataRole::Targets,
                key_id,
                public_key: crate::PublicKey(dali_crypto::public_key_from_seed(&SEED)),
            },
        )
    }

    fn signatures(body: &[u8], key_id: KeyId) -> SignatureSet {
        let mut records = [SignatureRecord::default(); crate::MAX_SIGNATURES];
        records[0] = SignatureRecord {
            key_id,
            signature: Signature(dali_crypto::sign(&SEED, body)),
        };
        SignatureSet { records, count: 1 }
    }

    #[test]
    fn authenticates_fragmented_body_and_returns_digest() {
        let (role, key) = role_and_key();
        let body = b"binary role body";
        let mut verifier = StreamingRoleVerifier::new(role, &[key], signatures(body, key.key_id))
            .expect("stream verifier should initialize");
        verifier.update(&body[..5]);
        verifier.update(&body[5..]);
        let result = verifier.finish().expect("signature should verify");
        assert_eq!(result.valid_signatures, 1);
        let mut expected = dali_crypto::Sha256Accumulator::new();
        expected.update(body);
        assert_eq!(result.sha256.0, expected.finalize());
    }

    #[test]
    fn rejects_a_tampered_fragment() {
        let (role, key) = role_and_key();
        let mut verifier =
            StreamingRoleVerifier::new(role, &[key], signatures(b"binary role body", key.key_id))
                .expect("stream verifier should initialize");
        verifier.update(b"binary role tampered");
        assert_eq!(
            verifier.finish(),
            Err(StreamingVerificationError::InvalidSignature)
        );
    }

    #[test]
    fn reports_bounded_progress_after_each_crypto_subchunk() {
        let (role, key) = role_and_key();
        let body = [7_u8; STREAMING_CRYPTO_PROGRESS_BYTES * 2 + 1];
        let mut verifier = StreamingRoleVerifier::new(role, &[key], signatures(&body, key.key_id))
            .expect("stream verifier should initialize");
        let mut progress_calls = 0;
        verifier
            .update_with_progress(&body, || {
                progress_calls += 1;
                true
            })
            .expect("progress callback should accept every subchunk");
        assert_eq!(progress_calls, 3);
        verifier.finish().expect("signature should verify");
    }
}
