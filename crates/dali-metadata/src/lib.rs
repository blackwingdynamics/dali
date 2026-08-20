#![no_std]

//! Bounded, hardware-neutral types for Dali repository metadata.
//!
//! This crate contains bounded contract types, canonical JSON codecs, and
//! hardware-neutral policy primitives. It does not access storage or provide
//! a concrete cryptographic backend.

mod authorization;
mod binary;
mod binary_roles;
mod chain;
mod codec;
mod crypto;
mod limits;
mod model;
mod parser;
mod trust_store;
mod validation;
mod verification;

pub use authorization::*;
pub use binary::*;
pub use binary_roles::*;
pub use chain::*;
pub use codec::*;
pub use crypto::*;
pub use limits::*;
pub use model::*;
pub use parser::*;
pub use trust_store::*;
pub use validation::*;
pub use verification::*;
