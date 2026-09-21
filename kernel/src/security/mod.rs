//! Kernel security boundaries that are disabled in the default MVP ABI.

#[cfg(feature = "abi-current")]
pub(crate) mod fault;

#[cfg(feature = "abi-current")]
pub(crate) mod launch;

#[cfg(feature = "abi-current")]
pub(crate) mod privilege;

#[cfg(feature = "abi-context-switch")]
pub(crate) mod scheduling;
