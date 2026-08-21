//! Strict, bounded parser for canonical metadata bodies.

mod binary;
mod bundle;
mod cursor;
mod delegation;
mod envelope;
mod revocation;
mod root;
mod signatures;
mod snapshot;
mod streaming;
mod streaming_chain;
mod streaming_delegation;
mod streaming_revocation;
mod streaming_root;
mod streaming_snapshot;
mod targets;
mod timestamp;

#[cfg(test)]
mod tests;

pub use binary::*;
pub use bundle::parse_bundle_signed;
pub use delegation::parse_delegation_signed;
pub use envelope::parse_signed_envelope;
pub use revocation::parse_revocation_signed;
pub use root::parse_root_signed;
pub use signatures::parse_signature_list;
pub use snapshot::parse_snapshot_signed;
pub use streaming::*;
pub use streaming_chain::{
    BinaryRoleBodyParser, StreamingChainError, matches_reference, role_definition, root_keys,
    verify_binary_role_envelope,
};
pub use streaming_delegation::BinaryDelegationBodyStreamParser;
pub use streaming_revocation::BinaryRevocationBodyStreamParser;
pub use streaming_root::BinaryRootBodyStreamParser;
pub use streaming_snapshot::BinarySnapshotBodyStreamParser;
pub use targets::parse_targets_signed;
pub use timestamp::parse_timestamp_signed;

/// Errors returned by the canonical metadata parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// The input ended before a complete value was available.
    UnexpectedEnd,
    /// A punctuation or field token did not match the canonical grammar.
    UnexpectedToken,
    /// A string contains invalid UTF-8 or forbidden escape/control bytes.
    InvalidString,
    /// A number is not a canonical unsigned integer.
    InvalidNumber,
    /// A fixed-width hexadecimal value is malformed.
    InvalidHex,
    /// A field name is unknown or appears out of canonical order.
    InvalidFieldOrder,
    /// A fixed-capacity record list is full.
    TooManyRecords,
    /// A value violates the metadata contract.
    InvalidValue,
    /// Bytes remain after one complete document.
    TrailingBytes,
    /// The binary metadata magic does not match the v2 contract.
    InvalidMagic,
    /// The binary metadata format version is unsupported.
    UnsupportedFormat,
}
