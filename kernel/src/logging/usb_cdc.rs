//! Board-neutral USB CDC serial logging backend.

use usb_device::{bus::UsbBusAllocator, class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

/// Hardware resources required to construct a USB CDC bus.
pub(crate) trait UsbResources {
    /// Concrete USB bus supplied by the selected board backend.
    type Bus: UsbBus;

    /// Builds the bus using the backend-owned USB peripheral and pins.
    fn into_bus(self, endpoint_memory: &'static mut [u32]) -> UsbBusAllocator<Self::Bus>;
}

type ActiveBus = <crate::board::UsbResources as UsbResources>::Bus;

const USB_VENDOR_ID: u16 = 0x1209;
const USB_PRODUCT_ID: u16 = 0xDA11;
const ENDPOINT_MEMORY_WORDS: usize = 1_024;

static mut ENDPOINT_MEMORY: [u32; ENDPOINT_MEMORY_WORDS] = [0; ENDPOINT_MEMORY_WORDS];
static mut USB_BUS: Option<UsbBusAllocator<ActiveBus>> = None;
static mut USB_SERIAL: Option<SerialPort<'static, ActiveBus>> = None;
static mut USB_DEVICE: Option<UsbDevice<'static, ActiveBus>> = None;

/// Initializes the USB CDC-ACM console.
pub(super) fn initialize(resources: crate::board::UsbResources) {
    // SAFETY: USB endpoint memory is initialized once during reset bootstrap and
    // remains exclusively owned by the single kernel execution context.
    let bus = unsafe {
        let endpoint_memory = core::slice::from_raw_parts_mut(
            core::ptr::addr_of_mut!(ENDPOINT_MEMORY).cast::<u32>(),
            ENDPOINT_MEMORY_WORDS,
        );
        resources.into_bus(endpoint_memory)
    };

    // SAFETY: USB resources are consumed once during reset bootstrap. The static
    // objects are only accessed by the single kernel execution context.
    unsafe {
        (*core::ptr::addr_of_mut!(USB_BUS)) = Some(bus);
        let bus = match (*core::ptr::addr_of!(USB_BUS)).as_ref() {
            Some(bus) => bus,
            None => return,
        };
        USB_SERIAL = Some(SerialPort::new(bus));
        USB_DEVICE = Some(
            UsbDeviceBuilder::new(bus, UsbVidPid(USB_VENDOR_ID, USB_PRODUCT_ID))
                .device_class(usbd_serial::USB_CLASS_CDC)
                .build(),
        );
    }
}

/// Polls USB control and CDC endpoint state.
pub(super) fn poll() {
    // SAFETY: The USB state is initialized once and accessed only by the kernel
    // main context; no interrupt-driven USB access is enabled in the MVP.
    unsafe {
        let device = match (*core::ptr::addr_of_mut!(USB_DEVICE)).as_mut() {
            Some(device) => device,
            None => return,
        };
        let serial = match (*core::ptr::addr_of_mut!(USB_SERIAL)).as_mut() {
            Some(serial) => serial,
            None => return,
        };
        let _ = device.poll(&mut [serial]);
    }
}

/// Writes bytes to the USB CDC endpoint when it accepts them.
pub(super) fn write_bytes(bytes: &[u8]) -> usize {
    // SAFETY: USB state ownership is restricted to the kernel main context.
    unsafe {
        let serial = match (*core::ptr::addr_of_mut!(USB_SERIAL)).as_mut() {
            Some(serial) => serial,
            None => return 0,
        };
        serial.write(bytes).map_or(0, |written| written)
    }
}

/// Returns whether the USB host has completed device configuration.
pub(super) fn host_ready() -> bool {
    // SAFETY: USB state ownership is restricted to the kernel main context.
    unsafe {
        (*core::ptr::addr_of!(USB_DEVICE))
            .as_ref()
            .is_some_and(|device| device.state() == UsbDeviceState::Configured)
    }
}
