//! Kernel security boundaries that are disabled in the default MVP ABI.

#[cfg(feature = "abi-v3")]
pub(crate) mod svc;
