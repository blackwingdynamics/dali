//! Main-context handoff point for the bounded installer ingress.

#[cfg(feature = "usb-install")]
use dali_usb::installation_queue::{DEFAULT_QUEUE_CAPACITY, InstallationReceiveQueue};

#[cfg(feature = "usb-install")]
/// Bounded complete-frame queue awaiting main-context protocol dispatch.
static mut RECEIVE_QUEUE: InstallationReceiveQueue<DEFAULT_QUEUE_CAPACITY> =
    InstallationReceiveQueue::new();

/// Copies bytes received by the dedicated installer CDC into the bounded queue.
pub(crate) fn receive_bytes(bytes: &[u8]) {
    #[cfg(feature = "usb-install")]
    // SAFETY: The installer callback runs only from the board USB interrupt;
    // frame consumption will be added in the kernel main-context service.
    unsafe {
        (*core::ptr::addr_of_mut!(RECEIVE_QUEUE)).push_bytes(bytes);
    }
    #[cfg(not(feature = "usb-install"))]
    let _ = bytes;
}
