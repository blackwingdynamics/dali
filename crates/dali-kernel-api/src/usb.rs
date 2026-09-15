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

/// Opaque USB interrupt operations supplied by a board backend.
#[derive(Clone, Copy)]
pub struct UsbOperations {
    /// Services the board-owned USB device through the kernel sink callback.
    pub service_irq: fn(fn(dali_usb::LinkState, &mut dyn dali_usb::ByteSink)),
    /// Copies one queued installation frame into caller-owned storage.
    pub poll_installation_frame:
        fn(&mut [u8; dali_usb::installation::MAX_FRAME_SIZE]) -> Option<usize>,
    /// Pends the board-owned USB interrupt.
    pub pend_irq: fn(),
}
