//! Platform selection boundary for hardware-specific kernel entry points.

#[cfg(feature = "board-stm32f405-sd")]
mod stm32f405;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-v3"))]
pub(crate) use stm32f405::TARGET_PROFILE;

#[cfg(not(feature = "board-stm32f405-sd"))]
compile_error!("Select a supported Dali OS platform feature");
