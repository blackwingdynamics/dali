//! WeAct Studio STM32F405RGT6 Core Board hardware backend facade.

#[cfg(feature = "driver-hardware-test")]
mod acceptance;
mod config;
mod initialization;
mod resources;
mod services;

mod input;
#[cfg(feature = "abi-context-switch")]
mod scheduler;
#[cfg(feature = "usb-cdc")]
mod usb;

#[cfg(feature = "abi-context-switch")]
pub(crate) use scheduler::enable_scheduler_tick;
#[cfg(feature = "usb-cdc")]
pub use usb::UsbResources;
#[cfg(feature = "usb-cdc")]
pub(crate) use usb::{pend_usb_irq, unmask_usb_irq};

/// First planned single-application F405 isolation layout.
pub const ISOLATION_LAYOUT: Option<crate::security::mpu::IsolationLayout> =
    config::ISOLATION_LAYOUT;
const _: () = assert!(ISOLATION_LAYOUT.is_some());
/// The current MVP board supports AMRN native application execution.
#[cfg(not(feature = "abi-current"))]
pub const APPLICATION_EXECUTION_SUPPORTED: bool = config::APPLICATION_EXECUTION_SUPPORTED;
/// System clock target derived from the declarative F405 target profile.
pub const SYSTEM_CLOCK_HZ: u32 = config::SYSTEM_CLOCK_HZ;
/// System clock in megahertz for the common platform facade.
pub const SYSTEM_CLOCK_MHZ: u32 = config::SYSTEM_CLOCK_MHZ;
/// Status LED output pin selected by the board configuration.
pub type StatusLed = config::StatusLed;
/// User-key input selected by the board configuration.
pub type UserKey = config::UserKey;
/// SDIO pins selected by the board configuration.
pub type SdioPins = config::SdioPins;
#[cfg(feature = "abi-current")]
pub const MEMORY_PROFILE: dali_targets::MemoryProfile = config::MEMORY_PROFILE;
#[cfg(feature = "abi-mpu")]
pub use config::activate_application_regions;
pub use initialization::initialize;
pub use resources::Board;
pub use services::{delay_ms, set_status_led};

#[cfg(feature = "abi-context-switch")]
pub(super) use resources::TimerMode;
