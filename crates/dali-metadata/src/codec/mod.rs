//! Canonical, bounded metadata encoders.

mod root;
mod targets;
mod writer;

#[cfg(test)]
mod tests;

pub use root::encode_root_signed;
pub use targets::encode_targets_signed;
pub use writer::EncodeError;
