//! Bounded, allocation-free I2C contracts.

mod address;
mod driver;

pub use address::I2cAddress;
pub use driver::I2cDriver;
