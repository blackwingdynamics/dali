//! Central ABI selector for the kernel build.

/// ABI version selected for this kernel build.
#[cfg(feature = "abi-current")]
pub(crate) const CURRENT_VERSION: u8 = dali_amrn::v2::ABI_VERSION;

/// Legacy MVP ABI version selected when the isolation ABI is disabled.
#[cfg(not(feature = "abi-current"))]
pub(crate) const CURRENT_VERSION: u8 = dali_amrn::ABI_VERSION;
