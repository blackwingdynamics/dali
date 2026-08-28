//! Board-local diagnostic output used during backend bring-up.

use core::fmt::Arguments;

#[cfg(feature = "usb-cdc")]
pub mod usb_cdc {
    use usb_device::{bus::UsbBus, bus::UsbBusAllocator};

    /// USB resources required to construct a board-owned USB bus.
    pub trait UsbResources {
        /// Concrete USB bus supplied by the board backend.
        type Bus: UsbBus;

        /// Builds the USB bus using backend-owned resources.
        fn into_bus(self, endpoint_memory: &'static mut [u32]) -> UsbBusAllocator<Self::Bus>;
    }

    /// Board-specific USB disconnect and reconnect operation.
    pub trait UsbBusReset {
        /// Requests host-visible USB re-enumeration.
        fn force_reenumeration(&self, delay: &mut dyn UsbResetDelay);
    }

    /// Bounded delay used during USB re-enumeration.
    pub trait UsbResetDelay {
        /// Delays for the backend-defined reset interval.
        fn delay_ms(&mut self, milliseconds: u32);
    }
}

/// Subsystem label for board bootstrap diagnostics.
pub const BOOT_SUBSYSTEM: &str = "BOOT";

/// Writes an informational board diagnostic.
#[cfg(feature = "driver-hardware-test")]
pub fn info(subsystem: &str, arguments: Arguments<'_>) {
    rtt_target::rprintln!("[INFO][{}] {}", subsystem, arguments);
}

/// Writes an error board diagnostic.
pub fn error(subsystem: &str, arguments: Arguments<'_>) {
    rtt_target::rprintln!("[ERROR][{}] {}", subsystem, arguments);
}
