//! Canonical, bounded metadata encoders.

mod root;
mod signatures;
mod snapshot;
mod targets;
mod timestamp;
mod writer;

#[cfg(test)]
mod tests;

pub use root::encode_root_signed;
pub use signatures::encode_signature_list;
pub use snapshot::encode_snapshot_signed;
pub use targets::encode_targets_signed;
pub use timestamp::encode_timestamp_signed;
pub use writer::EncodeError;
