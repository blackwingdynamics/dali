//! Bounded, allocation-free serial contracts.

mod config;
mod ownership;
mod read;
mod write;

pub use config::{BaudRate, DataBits, Parity, SerialConfig, SerialConfigure, StopBits};
pub use ownership::SerialOwnership;
pub use read::SerialRead;
pub use write::SerialWrite;
