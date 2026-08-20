//! Real cryptographic backend adapters for repository metadata.

use crate::{PublicKey, Signature, SignatureVerifier};

/// Ed25519 verifier used by the repository metadata chain.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ed25519Verifier;

impl SignatureVerifier for Ed25519Verifier {
    fn verify(&self, public_key: PublicKey, message: &[u8], signature: Signature) -> bool {
        dali_crypto::verify(&public_key.0, message, &signature.0).is_ok()
    }
}
