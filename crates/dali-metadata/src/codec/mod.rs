//! Canonical, bounded metadata encoders.

pub(crate) mod binary;
mod bundle;
mod delegation;
mod envelope;
mod revocation;
mod root;
mod signatures;
mod snapshot;
mod targets;
mod timestamp;
mod writer;

#[cfg(test)]
mod tests;

pub use binary::*;
pub use bundle::encode_bundle_signed;
pub use delegation::encode_delegation_signed;
pub use envelope::encode_signed_envelope;
pub use revocation::encode_revocation_signed;
pub use root::encode_root_signed;
pub use signatures::encode_signature_list;
pub use snapshot::encode_snapshot_signed;
pub use targets::encode_targets_signed;
pub use timestamp::encode_timestamp_signed;
pub use writer::EncodeError;
