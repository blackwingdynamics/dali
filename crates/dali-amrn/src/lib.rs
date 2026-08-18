#![no_std]

//! Hardware-independent AMRN package formats and validation.

pub mod compatibility;
mod legacy;
pub mod v2;
pub mod v3;
pub mod v4;

pub use legacy::*;
