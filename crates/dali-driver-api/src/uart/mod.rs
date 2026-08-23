//! Bounded, allocation-free serial contracts.

mod read;
mod write;

pub use read::SerialRead;
pub use write::SerialWrite;
