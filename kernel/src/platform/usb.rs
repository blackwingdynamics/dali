//! Registered board USB operations behind the kernel logging boundary.

use dali_kernel_api::BoardBackend;

/// USB operations registered by the selected firmware composition.
static USB_OPERATIONS: critical_section::Mutex<
    core::cell::RefCell<Option<dali_kernel_api::UsbOperations>>,
> = critical_section::Mutex::new(core::cell::RefCell::new(None));

/// Registers the selected board USB operations once.
pub(super) fn register<B: BoardBackend>() {
    critical_section::with(|cs| {
        let mut operations = USB_OPERATIONS.borrow(cs).borrow_mut();
        if operations.is_none() {
            *operations = B::usb_operations();
        }
    });
}

/// Services the registered board USB backend through a kernel-owned callback.
pub(crate) fn service_irq(
    drain: fn(dali_usb::LinkState, &mut dyn dali_usb::ByteSink),
    receive: dali_kernel_api::UsbInstallationReceive,
) {
    if let Some(operations) = critical_section::with(|cs| *USB_OPERATIONS.borrow(cs).borrow()) {
        (operations.service_irq)(drain, receive);
    }
}

/// Pends the registered board USB interrupt.
pub(crate) fn pend_irq() {
    if let Some(operations) = critical_section::with(|cs| *USB_OPERATIONS.borrow(cs).borrow()) {
        (operations.pend_irq)();
    }
}
