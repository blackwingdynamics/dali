//! Canonical bounded body and envelope parsers.

mod bundle;
mod cursor;
mod delegation;
mod envelope;
mod revocation;
mod root;
mod signatures;
mod snapshot;
mod targets;
mod timestamp;

pub(super) use super::DecodeError;
pub use bundle::parse_bundle_signed;
pub use delegation::parse_delegation_signed;
pub use envelope::parse_signed_envelope;
pub use revocation::parse_revocation_signed;
pub use root::parse_root_signed;
pub use signatures::parse_signature_list;
pub use snapshot::parse_snapshot_signed;
pub use targets::parse_targets_signed;
pub use timestamp::parse_timestamp_signed;
