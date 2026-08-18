//! Kernel security boundaries that are disabled in the default MVP ABI.

pub(crate) mod mpu;

#[cfg(feature = "abi-current")]
mod fault;

#[cfg(feature = "abi-current")]
mod scb;

#[cfg(feature = "abi-current")]
pub(crate) mod launch;

#[cfg(feature = "abi-current")]
pub(crate) mod svc;
