//! Canonical, bounded metadata encoders.

mod root;
mod writer;

#[cfg(test)]
mod tests;

pub use root::encode_root_signed;
pub use writer::EncodeError;
