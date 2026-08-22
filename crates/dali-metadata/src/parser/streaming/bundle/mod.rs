//! Bounded bundle-manifest parsing components.

mod parser;
mod queue;

#[cfg(test)]
mod tests;

pub use parser::{BinaryBundleBodyStreamParser, BundleManifestSummary};
