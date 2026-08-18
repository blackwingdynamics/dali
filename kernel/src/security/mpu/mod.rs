//! Memory-protection contracts and privileged activation boundary.

mod descriptor;
mod layout;

#[cfg(feature = "abi-mpu")]
mod hardware;

#[cfg(test)]
mod tests;

pub use layout::IsolationLayout;

#[cfg(feature = "abi-mpu")]
pub use hardware::{activate_application_regions, configure_hardware};
