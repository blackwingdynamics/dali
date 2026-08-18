//! Responsibility-specific AMRN loading pipelines.

#[cfg(any(feature = "abi-current", feature = "abi-relocation"))]
pub(super) use crate::loader::LoaderError;
#[cfg(feature = "abi-relocation")]
pub(super) use crate::loader::read_exact;

#[cfg(feature = "abi-relocation")]
pub(crate) mod discovery;
#[cfg(feature = "abi-current")]
pub(crate) mod execution;
#[cfg(feature = "abi-relocation")]
pub(crate) mod identity;
#[cfg(feature = "abi-relocation")]
pub(crate) mod relocation;
