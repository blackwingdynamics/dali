//! Cartridge validation and application launch handoff.

mod cartridge;

#[cfg(feature = "sdio")]
pub(super) use cartridge::load;
