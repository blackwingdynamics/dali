//! Canonical, bounded metadata encoders.

mod root;
mod snapshot;
mod targets;
mod timestamp;
mod writer;

#[cfg(test)]
mod tests;

pub use root::encode_root_signed;
pub use snapshot::encode_snapshot_signed;
pub use targets::encode_targets_signed;
pub use timestamp::encode_timestamp_signed;
pub use writer::EncodeError;
