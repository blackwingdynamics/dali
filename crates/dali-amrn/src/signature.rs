//! Bounded AMRN signature-envelope contract.

/// Signature envelope magic bytes.
pub const MAGIC: [u8; 4] = *b"DSIG";
/// Current signature-envelope encoding revision.
pub const ENVELOPE_VERSION: u8 = 1;
/// Ed25519 algorithm identifier reserved by the package contract.
pub const ED25519_ALGORITHM: u8 = 1;
/// Length of the opaque trust-anchor key identifier.
pub const KEY_ID_LENGTH: usize = 16;
/// Length of an Ed25519 signature.
pub const SIGNATURE_LENGTH: usize = 64;
/// Length of the fixed signature envelope.
pub const ENVELOPE_SIZE: usize = 8 + KEY_ID_LENGTH + SIGNATURE_LENGTH;

pub const VERSION_OFFSET: usize = 4;
pub const ALGORITHM_OFFSET: usize = 5;
pub const KEY_ID_LENGTH_OFFSET: usize = 6;
pub const SIGNATURE_LENGTH_OFFSET: usize = 7;
pub const KEY_ID_OFFSET: usize = 8;
pub const SIGNATURE_OFFSET: usize = KEY_ID_OFFSET + KEY_ID_LENGTH;

/// A validated signature envelope borrowing caller-owned bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Envelope<'a> {
    /// Algorithm identifier declared by the envelope.
    pub algorithm: u8,
    /// Opaque trust-anchor identifier used for key lookup.
    pub key_id: &'a [u8],
    /// Signature bytes covered by the declared algorithm.
    pub signature: &'a [u8],
}

/// Errors returned when decoding a bounded signature envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The input does not contain exactly one complete envelope.
    InvalidLength,
    /// The envelope magic or encoding revision is unsupported.
    InvalidHeader,
    /// The envelope declares an unsupported signature algorithm.
    UnsupportedAlgorithm,
}

/// Cryptographic verification boundary owned by the selected platform or host
/// signer backend.
pub trait SignatureVerifier {
    /// Backend-specific verification failure.
    type Error;

    /// Verifies an envelope over the caller-owned signed byte range.
    fn verify(&self, signed_bytes: &[u8], envelope: &Envelope<'_>) -> Result<(), Self::Error>;
}

/// Parses one fixed-size signature envelope without performing cryptographic verification.
pub fn parse(bytes: &[u8]) -> Result<Envelope<'_>, Error> {
    if bytes.len() != ENVELOPE_SIZE {
        return Err(Error::InvalidLength);
    }
    if bytes[..MAGIC.len()] != MAGIC
        || bytes[VERSION_OFFSET] != ENVELOPE_VERSION
        || bytes[KEY_ID_LENGTH_OFFSET] != KEY_ID_LENGTH as u8
        || bytes[SIGNATURE_LENGTH_OFFSET] != SIGNATURE_LENGTH as u8
    {
        return Err(Error::InvalidHeader);
    }
    if bytes[ALGORITHM_OFFSET] != ED25519_ALGORITHM {
        return Err(Error::UnsupportedAlgorithm);
    }
    Ok(Envelope {
        algorithm: bytes[ALGORITHM_OFFSET],
        key_id: &bytes[KEY_ID_OFFSET..SIGNATURE_OFFSET],
        signature: &bytes[SIGNATURE_OFFSET..],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_bounded_ed25519_envelope() {
        let mut bytes = [0; ENVELOPE_SIZE];
        bytes[..MAGIC.len()].copy_from_slice(&MAGIC);
        bytes[VERSION_OFFSET] = ENVELOPE_VERSION;
        bytes[ALGORITHM_OFFSET] = ED25519_ALGORITHM;
        bytes[KEY_ID_LENGTH_OFFSET] = KEY_ID_LENGTH as u8;
        bytes[SIGNATURE_LENGTH_OFFSET] = SIGNATURE_LENGTH as u8;
        let envelope = parse(&bytes).expect("contract-valid envelope parses");
        assert_eq!(envelope.algorithm, ED25519_ALGORITHM);
        assert_eq!(envelope.key_id.len(), KEY_ID_LENGTH);
        assert_eq!(envelope.signature.len(), SIGNATURE_LENGTH);
    }

    #[test]
    fn rejects_truncated_or_extended_envelopes() {
        assert_eq!(parse(&[0; ENVELOPE_SIZE - 1]), Err(Error::InvalidLength));
        assert_eq!(parse(&[0; ENVELOPE_SIZE + 1]), Err(Error::InvalidLength));
    }

    #[test]
    fn rejects_an_unknown_algorithm() {
        let mut bytes = [0; ENVELOPE_SIZE];
        bytes[..MAGIC.len()].copy_from_slice(&MAGIC);
        bytes[VERSION_OFFSET] = ENVELOPE_VERSION;
        bytes[ALGORITHM_OFFSET] = ED25519_ALGORITHM + 1;
        bytes[KEY_ID_LENGTH_OFFSET] = KEY_ID_LENGTH as u8;
        bytes[SIGNATURE_LENGTH_OFFSET] = SIGNATURE_LENGTH as u8;
        assert_eq!(parse(&bytes), Err(Error::UnsupportedAlgorithm));
    }
}
