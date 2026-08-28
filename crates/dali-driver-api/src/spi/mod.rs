//! Bounded, allocation-free SPI contracts.

mod ownership;
mod transfer;

pub use ownership::{SpiBusOwnership, SpiDeviceId, SpiDeviceSelect};
pub use transfer::SpiTransfer;
