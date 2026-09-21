//! Cartridge validation and application launch handoff.

#[cfg(feature = "abi-current")]
mod activation;
mod cartridge;

#[cfg(feature = "sdio")]
pub(super) use cartridge::load;
#[cfg(all(feature = "artifact-flash", feature = "abi-current"))]
pub(super) use cartridge::try_load_flash;
