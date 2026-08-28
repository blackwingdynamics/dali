//! WeAct Studio STM32F405RGT6 Core Board hardware backend facade.

#[cfg(feature = "driver-hardware-test")]
#[path = "board/acceptance.rs"]
mod acceptance;
#[path = "board/config.rs"]
mod config;
#[path = "board/initialization.rs"]
mod initialization;
#[path = "board/resources.rs"]
mod resources;
#[path = "board/services.rs"]
mod services;

#[path = "board/input.rs"]
mod input;
#[cfg(feature = "abi-context-switch")]
#[path = "board/scheduler.rs"]
mod scheduler;
#[cfg(feature = "usb-cdc")]
#[path = "board/usb.rs"]
mod usb;

#[cfg(feature = "abi-context-switch")]
pub(crate) use scheduler::enable_scheduler_tick;
#[cfg(feature = "usb-cdc")]
pub use usb::UsbResources;
#[cfg(feature = "usb-cdc")]
pub(crate) use usb::{pend_usb_irq, unmask_usb_irq};

/// System clock target derived from the declarative F405 target profile.
pub const SYSTEM_CLOCK_HZ: u32 = config::SYSTEM_CLOCK_HZ;
/// MPU layout derived from the target memory contract.
/// System clock in megahertz for the common platform facade.
pub const SYSTEM_CLOCK_MHZ: u32 = config::SYSTEM_CLOCK_MHZ;
/// Status LED output pin selected by the board configuration.
pub type StatusLed = config::StatusLed;
/// User-key input selected by the board configuration.
pub type UserKey = config::UserKey;
/// SDIO pins selected by the board configuration.
pub type SdioPins = config::SdioPins;
#[cfg(feature = "abi-mpu")]
pub use config::{activate_application_regions, configure_memory_protection};
pub use initialization::initialize;
pub use resources::Board;
pub use services::{delay_ms, set_status_led};

#[cfg(feature = "abi-context-switch")]
pub(super) use resources::TimerMode;
