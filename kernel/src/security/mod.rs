//! Kernel security boundaries that are disabled in the default MVP ABI.

#[cfg(feature = "abi-current")]
mod fault;

#[cfg(feature = "abi-current")]
pub(crate) mod launch;

#[cfg(feature = "abi-current")]
pub(crate) mod svc;
