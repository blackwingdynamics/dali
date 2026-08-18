#![cfg_attr(not(test), no_std)]

//! Hardware-independent kernel contracts exposed for host-side testing.

pub mod drivers;
pub mod runtime;
pub mod storage;
