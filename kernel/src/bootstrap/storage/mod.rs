//! Storage discovery, filesystem acceptance, and package handoff.

#[cfg(feature = "storage-write")]
mod acceptance;
mod initialization;

pub(super) use initialization::initialize;
