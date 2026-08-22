//! Storage discovery, filesystem acceptance, and package handoff.

#[cfg(feature = "storage-write")]
mod acceptance;
mod initialization;
#[cfg(feature = "storage-interruption-test")]
pub(super) mod interruption;

pub(super) use initialization::initialize;
