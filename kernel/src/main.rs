#![no_std]
#![no_main]

pub(crate) mod board;
mod bootstrap;
pub mod drivers;
pub mod loader;
pub mod logging;
pub mod storage;

use cortex_m_rt::entry;
use panic_halt as _;

#[cfg(feature = "usb-cdc")]
use stm32f4xx_hal::pac::interrupt;

#[cfg(feature = "usb-cdc")]
#[interrupt]
fn OTG_FS() {
    logging::service_usb_irq();
}

#[entry]
fn main() -> ! {
    bootstrap::run()
}
