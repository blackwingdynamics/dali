#![no_std]

//! Hardware-neutral Ed25519 signing and verification primitives.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Ed25519 public-key length in bytes.
pub const PUBLIC_KEY_LENGTH: usize = 32;
/// Ed25519 private-key seed length in bytes.
pub const PRIVATE_KEY_LENGTH: usize = 32;
/// Ed25519 signature length in bytes.
pub const SIGNATURE_LENGTH: usize = 64;

/// Errors returned when a public key cannot be decoded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationError {
    /// The supplied public key is not a valid Ed25519 key.
    InvalidPublicKey,
    /// The signature does not authenticate the supplied message.
    InvalidSignature,
}

/// Verifies one Ed25519 signature over caller-owned bytes.
pub fn verify(
    public_key: &[u8; PUBLIC_KEY_LENGTH],
    message: &[u8],
    signature: &[u8; SIGNATURE_LENGTH],
) -> Result<(), VerificationError> {
    let key =
        VerifyingKey::from_bytes(public_key).map_err(|_| VerificationError::InvalidPublicKey)?;
    let signature = Signature::from_bytes(signature);
    key.verify(message, &signature)
        .map_err(|_| VerificationError::InvalidSignature)
}

/// Signs caller-owned bytes with an externally supplied private-key seed.
pub fn sign(private_key: &[u8; PRIVATE_KEY_LENGTH], message: &[u8]) -> [u8; SIGNATURE_LENGTH] {
    SigningKey::from_bytes(private_key).sign(message).to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIVATE_KEY: [u8; PRIVATE_KEY_LENGTH] = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ];
    const PUBLIC_KEY: [u8; PUBLIC_KEY_LENGTH] = [
        0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07,
        0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07,
        0x51, 0x1a,
    ];

    #[test]
    fn signs_and_verifies_the_rfc8032_empty_message_vector() {
        let signature = sign(&PRIVATE_KEY, &[]);
        assert_eq!(verify(&PUBLIC_KEY, &[], &signature), Ok(()));
    }

    #[test]
    fn rejects_a_modified_message() {
        let signature = sign(&PRIVATE_KEY, b"signed");
        assert_eq!(
            verify(&PUBLIC_KEY, b"modified", &signature),
            Err(VerificationError::InvalidSignature)
        );
    }
}
