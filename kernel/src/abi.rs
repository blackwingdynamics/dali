//! Central ABI selector for the kernel build.

use dali_amrn::compatibility;

/// Compatibility contract selected by the current kernel ABI feature.
#[cfg(feature = "abi-current")]
pub(crate) const CURRENT_CONTRACT: compatibility::AbiContract = compatibility::ISOLATION;

/// Compatibility contract selected by the legacy kernel ABI configuration.
#[cfg(not(feature = "abi-current"))]
pub(crate) const CURRENT_CONTRACT: compatibility::AbiContract = compatibility::LEGACY;

/// ABI version selected for this kernel build.
pub(crate) const CURRENT_VERSION: u8 = CURRENT_CONTRACT.abi_version;
