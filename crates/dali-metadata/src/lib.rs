#![no_std]

//! Bounded, hardware-neutral types for Dali repository metadata.
//!
//! This crate intentionally contains contract types and validation only. It
//! does not parse unbounded JSON, access storage, or perform cryptography.
//! The canonical metadata codec will build on these types after its wire
//! behavior is implemented and tested.

mod codec;
mod limits;
mod model;
mod parser;
mod validation;

pub use codec::*;
pub use limits::*;
pub use model::*;
pub use parser::*;
pub use validation::*;
