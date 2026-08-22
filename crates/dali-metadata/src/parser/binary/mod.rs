//! Canonical Binary Metadata v2 role bodies.

mod bundle;
mod delegation;
mod helpers;
mod revocation;
mod root;
mod snapshot;
mod targets;
mod trust_store;
pub use bundle::{encode_binary_bundle_body, parse_binary_bundle_body};
pub use delegation::{encode_binary_delegation_body, parse_binary_delegation_body};
pub(super) use helpers::{
    bundle_kind_from_number, bundle_kind_number, header, read_header, read_reference,
    read_revocation_reference, read_text, reference, text,
};
pub use revocation::{encode_binary_revocation_body, parse_binary_revocation_body};
pub use root::{encode_binary_root_body, parse_binary_root_body};
pub use snapshot::{encode_binary_snapshot_body, parse_binary_snapshot_body};
pub use targets::{
    encode_binary_target_record, encode_binary_targets_body, parse_binary_target_record,
    parse_binary_targets_body,
};
pub use trust_store::{encode_binary_trust_store_body, parse_binary_trust_store_body};

#[cfg(test)]
mod tests;
