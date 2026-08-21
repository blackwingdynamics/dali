//! Second-pass verification for Binary Metadata v2 role envelopes.

use crate::{
    BinaryDelegationBodyStreamParser, BinaryEnvelopeStreamParser, BinaryRevocationBodyStreamParser,
    BinaryRootBodyStreamParser, BinarySnapshotBodyStreamParser, BinaryTimestampBodyStreamParser,
    DecodeError, MetadataRole, RoleDefinition, RoleKey, Sha256Digest, StreamVerificationResult,
    StreamingBodyError, StreamingRoleVerifier,
};
use core::mem::MaybeUninit;

/// Typed role-body parser used by the Binary v2 second pass.
pub trait BinaryRoleBodyParser: Sized {
    /// Typed metadata produced after the body has been parsed.
    type Output: Copy;

    /// Feeds a body fragment into the bounded state machine.
    fn feed(&mut self, bytes: &[u8]) -> Result<(), StreamingBodyError>;

    /// Finishes the state machine and validates the typed body.
    fn finish(&mut self) -> Result<Self::Output, StreamingBodyError>;

    /// Finishes directly into caller-owned output storage.
    fn finish_into(
        &mut self,
        output: &mut MaybeUninit<Self::Output>,
    ) -> Result<(), StreamingBodyError> {
        output.write(self.finish()?);
        Ok(())
    }
}

macro_rules! role_parser {
    ($parser:ty, $output:ty) => {
        impl BinaryRoleBodyParser for $parser {
            type Output = $output;
            fn feed(&mut self, bytes: &[u8]) -> Result<(), StreamingBodyError> {
                Self::feed(self, bytes)
            }
            fn finish(&mut self) -> Result<Self::Output, StreamingBodyError> {
                Self::finish(self)
            }
        }
    };
}

role_parser!(BinaryRootBodyStreamParser, crate::RootMetadata);
role_parser!(BinaryTimestampBodyStreamParser, crate::TimestampMetadata);
impl<const CAPACITY: usize> BinaryRoleBodyParser for BinarySnapshotBodyStreamParser<CAPACITY> {
    type Output = crate::SnapshotMetadata;

    fn feed(&mut self, bytes: &[u8]) -> Result<(), StreamingBodyError> {
        Self::feed(self, bytes)
    }

    fn finish(&mut self) -> Result<Self::Output, StreamingBodyError> {
        Self::finish(self)
    }

    fn finish_into(
        &mut self,
        output: &mut MaybeUninit<Self::Output>,
    ) -> Result<(), StreamingBodyError> {
        Self::finish_into(self, output)
    }
}
impl<const NAMESPACE_CAPACITY: usize, const TARGET_CAPACITY: usize, const ABI_CAPACITY: usize>
    BinaryRoleBodyParser
    for BinaryDelegationBodyStreamParser<NAMESPACE_CAPACITY, TARGET_CAPACITY, ABI_CAPACITY>
{
    type Output = crate::DelegationMetadata;

    fn feed(&mut self, bytes: &[u8]) -> Result<(), StreamingBodyError> {
        Self::feed(self, bytes)
    }

    fn finish(&mut self) -> Result<Self::Output, StreamingBodyError> {
        Self::finish(self)
    }

    fn finish_into(
        &mut self,
        output: &mut MaybeUninit<Self::Output>,
    ) -> Result<(), StreamingBodyError> {
        Self::finish_into(self, output)
    }
}
impl<const CAPACITY: usize> BinaryRoleBodyParser for BinaryRevocationBodyStreamParser<CAPACITY> {
    type Output = crate::RevocationMetadata;

    fn feed(&mut self, bytes: &[u8]) -> Result<(), StreamingBodyError> {
        Self::feed(self, bytes)
    }

    fn finish(&mut self) -> Result<Self::Output, StreamingBodyError> {
        Self::finish(self)
    }

    fn finish_into(
        &mut self,
        output: &mut MaybeUninit<Self::Output>,
    ) -> Result<(), StreamingBodyError> {
        Self::finish_into(self, output)
    }
}

/// Errors returned by one bounded Binary v2 parse-and-replay verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingChainError {
    /// The envelope or typed body was malformed.
    Decode,
    /// The replayed body did not authenticate under the supplied role policy.
    Signature,
    /// The first and second passes produced different body digests.
    DigestMismatch,
}

/// Parses a role envelope, then replays its body through Ed25519 and SHA-256.
pub fn verify_binary_role_envelope<P>(
    bytes: &[u8],
    expected_role: MetadataRole,
    role: RoleDefinition,
    keys: &[RoleKey],
    parser: P,
) -> Result<(P::Output, StreamVerificationResult), StreamingChainError>
where
    P: BinaryRoleBodyParser,
{
    let (metadata, envelope) = parse_first_pass(bytes, expected_role, parser)?;
    let mut verifier = StreamingRoleVerifier::new(role, keys, envelope.signatures)
        .map_err(|_| StreamingChainError::Signature)?;
    let mut replay = BinaryEnvelopeStreamParser::new(expected_role);
    replay
        .feed(bytes, |chunk| {
            verifier.update(chunk);
            Ok::<(), DecodeError>(())
        })
        .map_err(|_| StreamingChainError::Decode)?;
    let replayed = replay.finish().map_err(|_| StreamingChainError::Decode)?;
    if replayed.body_sha256 != envelope.body_sha256 {
        return Err(StreamingChainError::DigestMismatch);
    }
    let result = verifier
        .finish()
        .map_err(|_| StreamingChainError::Signature)?;
    Ok((metadata, result))
}

fn parse_first_pass<P>(
    bytes: &[u8],
    expected_role: MetadataRole,
    mut parser: P,
) -> Result<(P::Output, crate::StreamedEnvelope), StreamingChainError>
where
    P: BinaryRoleBodyParser,
{
    let mut envelope = BinaryEnvelopeStreamParser::new(expected_role);
    envelope
        .feed(bytes, |chunk| {
            parser.feed(chunk).map_err(|_| DecodeError::InvalidValue)
        })
        .map_err(|_| StreamingChainError::Decode)?;
    let envelope = envelope.finish().map_err(|_| StreamingChainError::Decode)?;
    let metadata = parser.finish().map_err(|_| StreamingChainError::Decode)?;
    Ok((metadata, envelope))
}

/// Verifies that a streamed digest matches a declared metadata reference.
pub fn matches_reference(
    version: u64,
    length: u32,
    digest: Sha256Digest,
    actual_version: u64,
    actual_length: u32,
    actual_digest: Sha256Digest,
) -> bool {
    version == actual_version && length == actual_length && digest == actual_digest
}

/// Keeps the role policy lookup used by chain assemblers explicit.
pub fn role_definition(root: crate::RootMetadata, role: MetadataRole) -> Option<RoleDefinition> {
    root.roles
        .iter()
        .take(usize::from(root.role_count))
        .find(|definition| definition.role == role)
        .copied()
}

/// Keeps the root key slice bounded for callers assembling the full chain.
pub fn root_keys(root: crate::RootMetadata) -> [RoleKey; crate::MAX_ROOT_KEYS] {
    root.keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyId, MetadataHeader, PublicKey, Signature, SignatureRecord, TimestampMetadata};

    const SEED: [u8; 32] = [31; 32];

    #[test]
    fn authenticates_a_timestamp_on_the_second_pass() {
        let key_id = KeyId([4; crate::KEY_ID_LENGTH]);
        let key = RoleKey {
            role: MetadataRole::Timestamp,
            key_id,
            public_key: PublicKey(dali_crypto::public_key_from_seed(&SEED)),
        };
        let role = RoleDefinition {
            role: MetadataRole::Timestamp,
            keys: [key_id; crate::MAX_ROLE_KEYS],
            key_count: 1,
            threshold: 1,
        };
        let timestamp = TimestampMetadata {
            header: MetadataHeader {
                role: MetadataRole::Timestamp,
                version: 1,
                expires: 0,
            },
            snapshot_version: 1,
            snapshot_length: 64,
            snapshot_sha256: crate::Sha256Digest([8; crate::SHA256_LENGTH]),
        };
        let mut body = [0; crate::MAX_TIMESTAMP_BYTES];
        let body_length =
            crate::encode_binary_timestamp_body(timestamp, &mut body).expect("timestamp encodes");
        let signature = Signature(dali_crypto::sign(&SEED, &body[..body_length]));
        let mut signatures = [SignatureRecord::default(); crate::MAX_SIGNATURES];
        signatures[0] = SignatureRecord { key_id, signature };
        let mut envelope = [0; crate::MAX_TIMESTAMP_BYTES
            + crate::BINARY_ENVELOPE_HEADER_BYTES
            + crate::BINARY_SIGNATURE_RECORD_BYTES];
        let length = crate::encode_binary_envelope(
            MetadataRole::Timestamp,
            &body[..body_length],
            crate::SignatureSet {
                records: signatures,
                count: 1,
            },
            &mut envelope,
        )
        .expect("envelope encodes");
        let (parsed, result) = verify_binary_role_envelope(
            &envelope[..length],
            MetadataRole::Timestamp,
            role,
            &[key],
            BinaryTimestampBodyStreamParser::new(),
        )
        .expect("chain verifies");
        assert_eq!(parsed, timestamp);
        assert_eq!(result.valid_signatures, 1);
    }

    #[test]
    fn rejects_a_tampered_body_before_chain_completion() {
        let key_id = KeyId([4; crate::KEY_ID_LENGTH]);
        let key = RoleKey {
            role: MetadataRole::Timestamp,
            key_id,
            public_key: PublicKey(dali_crypto::public_key_from_seed(&SEED)),
        };
        let role = RoleDefinition {
            role: MetadataRole::Timestamp,
            keys: [key_id; crate::MAX_ROLE_KEYS],
            key_count: 1,
            threshold: 1,
        };
        let timestamp = TimestampMetadata {
            header: MetadataHeader {
                role: MetadataRole::Timestamp,
                version: 1,
                expires: 0,
            },
            snapshot_version: 1,
            snapshot_length: 64,
            snapshot_sha256: crate::Sha256Digest([8; crate::SHA256_LENGTH]),
        };
        let mut body = [0; crate::MAX_TIMESTAMP_BYTES];
        let body_length =
            crate::encode_binary_timestamp_body(timestamp, &mut body).expect("timestamp encodes");
        let mut signatures = [SignatureRecord::default(); crate::MAX_SIGNATURES];
        signatures[0] = SignatureRecord {
            key_id,
            signature: Signature(dali_crypto::sign(&SEED, &body[..body_length])),
        };
        let mut envelope = [0; crate::MAX_TIMESTAMP_BYTES
            + crate::BINARY_ENVELOPE_HEADER_BYTES
            + crate::BINARY_SIGNATURE_RECORD_BYTES];
        let length = crate::encode_binary_envelope(
            MetadataRole::Timestamp,
            &body[..body_length],
            crate::SignatureSet {
                records: signatures,
                count: 1,
            },
            &mut envelope,
        )
        .expect("envelope encodes");
        envelope[crate::BINARY_ENVELOPE_HEADER_BYTES] ^= 1;
        assert_eq!(
            verify_binary_role_envelope(
                &envelope[..length],
                MetadataRole::Timestamp,
                role,
                &[key],
                BinaryTimestampBodyStreamParser::new(),
            ),
            Err(StreamingChainError::Decode)
        );
    }
}
