//! Board-neutral USB CDC serial logging backend.

use core::cell::RefCell;
use cortex_m::interrupt::{Mutex, free};
use usb_device::{bus::UsbBusAllocator, class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

/// Hardware resources required to construct a USB CDC bus.
pub(crate) trait UsbResources {
    /// Concrete USB bus supplied by the selected board backend.
    type Bus: UsbBus;

    /// Builds the bus using the backend-owned USB peripheral and pins.
    fn into_bus(self, endpoint_memory: &'static mut [u32]) -> UsbBusAllocator<Self::Bus>;
}

type ActiveBus = <crate::platform::UsbResources as UsbResources>::Bus;

const USB_VENDOR_ID: u16 = 0x1209;
const USB_PRODUCT_ID: u16 = 0xDA11;
const ENDPOINT_MEMORY_WORDS: usize = 1_024;

// The endpoint arena is kept in the board's DMA-visible buffer section so a
// future CCM migration cannot move USB transport memory into CCM by accident.
#[unsafe(link_section = ".dma_buffer")]
static mut ENDPOINT_MEMORY: [u32; ENDPOINT_MEMORY_WORDS] = [0; ENDPOINT_MEMORY_WORDS];
static mut USB_BUS: Option<UsbBusAllocator<ActiveBus>> = None;
static USB_SERIAL: Mutex<RefCell<Option<SerialPort<'static, ActiveBus>>>> =
    Mutex::new(RefCell::new(None));
static USB_DEVICE: Mutex<RefCell<Option<UsbDevice<'static, ActiveBus>>>> =
    Mutex::new(RefCell::new(None));

/// Initializes the USB CDC-ACM console.
pub(super) fn initialize(resources: crate::platform::UsbResources) {
    // SAFETY: USB endpoint memory is initialized once during reset bootstrap and
    // remains exclusively owned by the single kernel execution context.
    let bus = unsafe {
        let endpoint_memory = core::slice::from_raw_parts_mut(
            core::ptr::addr_of_mut!(ENDPOINT_MEMORY).cast::<u32>(),
            ENDPOINT_MEMORY_WORDS,
        );
        resources.into_bus(endpoint_memory)
    };

    // SAFETY: USB resources are consumed once during reset bootstrap. The bus
    // allocator remains alive for the static lifetime required by its endpoints.
    unsafe {
        (*core::ptr::addr_of_mut!(USB_BUS)) = Some(bus);
        let bus = match (*core::ptr::addr_of!(USB_BUS)).as_ref() {
            Some(bus) => bus,
            None => return,
        };
        free(|cs| {
            *USB_SERIAL.borrow(cs).borrow_mut() = Some(SerialPort::new(bus));
            *USB_DEVICE.borrow(cs).borrow_mut() = Some(
                UsbDeviceBuilder::new(bus, UsbVidPid(USB_VENDOR_ID, USB_PRODUCT_ID))
                    .device_class(usbd_serial::USB_CLASS_CDC)
                    .build(),
            );
        });

        // SAFETY: USB resources and shared state are initialized before the
        // platform backend unmasks the sole USB owner.
        crate::platform::unmask_usb_irq();
        // Boot messages may have been queued before USB resources existed, so
        // the pre-initialization pend cannot wake the newly installed owner.
        // Request one bounded service pass after ownership is transferred.
        crate::platform::pend_usb_irq();
    }
}

/// Services USB control and CDC endpoint state from the OTG_FS interrupt.
pub(super) fn service_irq<F>(drain: F)
where
    F: FnOnce(dali_usb::LinkState, &mut dyn dali_usb::ByteSink),
{
    static mut USB_SERIAL_ISR: Option<SerialPort<'static, ActiveBus>> = None;
    static mut USB_DEVICE_ISR: Option<UsbDevice<'static, ActiveBus>> = None;

    // SAFETY: OTG_FS is the only caller after initialization. The bootstrap
    // critical section transfers each object here exactly once, after which
    // these ISR-local statics are the sole USB owner.
    unsafe {
        if (*core::ptr::addr_of!(USB_DEVICE_ISR)).is_none() {
            let Some(device) = free(|cs| USB_DEVICE.borrow(cs).replace(None)) else {
                return;
            };
            (*core::ptr::addr_of_mut!(USB_DEVICE_ISR)) = Some(device);
        }
        if (*core::ptr::addr_of!(USB_SERIAL_ISR)).is_none() {
            let Some(serial) = free(|cs| USB_SERIAL.borrow(cs).replace(None)) else {
                return;
            };
            (*core::ptr::addr_of_mut!(USB_SERIAL_ISR)) = Some(serial);
        }

        let (Some(device), Some(serial)) = (
            (*core::ptr::addr_of_mut!(USB_DEVICE_ISR)).as_mut(),
            (*core::ptr::addr_of_mut!(USB_SERIAL_ISR)).as_mut(),
        ) else {
            return;
        };
        let _ = device.poll(&mut [serial]);
        let link = dali_usb::LinkState::from_configured_and_open(
            device.state() == UsbDeviceState::Configured,
            serial.dtr(),
        );
        drain(link, &mut UsbSink { serial });
    }
}

struct UsbSink<'a> {
    serial: &'a mut SerialPort<'static, ActiveBus>,
}

impl dali_usb::ByteSink for UsbSink<'_> {
    fn write(&mut self, bytes: &[u8]) -> usize {
        self.serial.write(bytes).map_or(0, |written| written)
    }

    fn flush(&mut self) -> dali_usb::FlushStatus {
        match self.serial.flush() {
            Ok(()) => dali_usb::FlushStatus::Complete,
            Err(UsbError::WouldBlock) => dali_usb::FlushStatus::Pending,
            Err(_) => dali_usb::FlushStatus::Failed,
        }
    }
}
