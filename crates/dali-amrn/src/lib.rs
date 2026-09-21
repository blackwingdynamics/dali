#![no_std]

//! Hardware-independent AMRN cartridge formats and validation.

pub mod compatibility;
mod legacy;
pub mod signature;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

pub use legacy::*;
