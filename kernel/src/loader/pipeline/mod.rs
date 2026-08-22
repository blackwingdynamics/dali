//! Responsibility-specific AMRN loading pipelines.

#[cfg(any(feature = "abi-current", feature = "abi-relocation"))]
pub(super) use crate::loader::LoaderError;
#[cfg(any(
    feature = "abi-authentication",
    all(feature = "abi-relocation", not(feature = "repository-loader")),
))]
pub(super) use crate::loader::read_exact;

#[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
pub(crate) mod discovery;
#[cfg(feature = "abi-current")]
pub(crate) mod execution;
#[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
pub(crate) mod identity;
#[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
pub(crate) mod relocation;
#[cfg(feature = "abi-authentication")]
pub(crate) mod signed;
#[cfg(any(
    feature = "abi-authentication",
    all(feature = "abi-relocation", not(feature = "repository-loader")),
))]
pub(super) use services::supports_required_services;

#[cfg(any(
    feature = "abi-authentication",
    all(feature = "abi-relocation", not(feature = "repository-loader")),
))]
mod services;
