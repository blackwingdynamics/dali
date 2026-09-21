//! Hardware-neutral USB resource contracts.

use usb_device::bus::{UsbBus, UsbBusAllocator};

/// USB resources supplied by a board backend.
pub trait UsbResources {
    /// Concrete USB bus implementation owned by the backend.
    type Bus: UsbBus;

    /// Builds the bus using caller-owned endpoint memory.
    fn into_bus(self, endpoint_memory: &'static mut [u32]) -> UsbBusAllocator<Self::Bus>;
}

/// Kernel callback used to drain the log-only CDC channel.
pub type UsbLogDrain = fn(dali_usb::LinkState, &mut dyn dali_usb::ByteSink);

/// Kernel callback used to receive bounded bytes from the installer CDC.
pub type UsbInstallationReceive = fn(&[u8]);

/// Board callback used to service the log-only CDC channel.
#[cfg(not(feature = "usb-install"))]
pub type UsbService = fn(UsbLogDrain);

/// Board callback used to service both CDC channels when installation is enabled.
#[cfg(feature = "usb-install")]
pub type UsbService = fn(UsbLogDrain, UsbInstallationReceive);

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

/// Opaque USB interrupt operations supplied by a board backend.
#[derive(Clone, Copy)]
pub struct UsbOperations {
    /// Services the board-owned USB device and optional installer CDC input.
    pub service_irq: UsbService,
    /// Pends the board-owned USB interrupt.
    pub pend_irq: fn(),
}
