//! Hardware-neutral USB resource contracts.

use usb_device::bus::{UsbBus, UsbBusAllocator};

/// USB resources supplied by a board backend.
pub trait UsbResources {
    /// Concrete USB bus implementation owned by the backend.
    type Bus: UsbBus;

    /// Builds the bus using caller-owned endpoint memory.
    fn into_bus(self, endpoint_memory: &'static mut [u32]) -> UsbBusAllocator<Self::Bus>;
}

/// Board operation used to force host-visible USB re-enumeration.
pub trait UsbBusReset {
    /// Performs the bounded disconnect/reconnect sequence.
    fn force_reenumeration(&self, delay: &mut dyn UsbResetDelay);
}

/// Delay capability used by a USB reset sequence.
pub trait UsbResetDelay {
    /// Delays for the backend-defined interval.
    fn delay_ms(&mut self, milliseconds: u32);
}
