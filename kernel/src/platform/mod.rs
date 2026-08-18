//! Platform selection boundary for hardware-specific kernel entry points.

#[cfg(feature = "board-stm32f405-sd")]
mod f405;

/// Stable operations supplied by a selected platform backend.
pub(crate) trait Backend: Sized {
    /// Human-readable system clock value exposed by the boot status path.
    const SYSTEM_CLOCK_MHZ: u32;

    #[cfg(feature = "sdio")]
    /// Generic block reader supplied by the platform SDIO transport.
    type SdioReader: crate::drivers::BlockReader;

    #[cfg(feature = "usb-cdc")]
    /// Board-owned resources used to construct the USB bus.
    type UsbResources: crate::logging::usb_cdc::UsbResources;

    /// Takes the platform singletons and initializes the backend.
    fn initialize() -> Self;

    /// Sets the board status indicator.
    fn set_status_led(&mut self, on: bool);

    /// Delays for a bounded number of milliseconds.
    fn delay_ms(&mut self, milliseconds: u32);

    /// Enables the target scheduler tick after application activation.
    #[cfg(feature = "abi-context-switch")]
    fn enable_scheduler_tick(&mut self, tick_hz: u32) -> bool;

    #[cfg(feature = "sdio")]
    /// Transfers the SDIO reader to the storage policy.
    fn take_sdio_reader(&mut self) -> Option<Self::SdioReader>;

    #[cfg(feature = "usb-cdc")]
    /// Transfers USB resources to the logging backend.
    fn take_usb_resources(&mut self) -> Option<Self::UsbResources>;

    /// Enables the backend's USB interrupt after initialization.
    #[cfg(feature = "usb-cdc")]
    fn unmask_usb_irq();

    /// Pends the backend's USB interrupt after a log enqueue.
    #[cfg(feature = "usb-cdc")]
    fn pend_usb_irq();
}

/// Stable kernel-facing facade over the selected platform backend.
pub(crate) struct Platform(f405::Board);

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) const SYSTEM_CLOCK_MHZ: u32 = <f405::Board as Backend>::SYSTEM_CLOCK_MHZ;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) fn initialize() -> Platform {
    Platform(<f405::Board as Backend>::initialize())
}

#[cfg(feature = "board-stm32f405-sd")]
impl Platform {
    pub(crate) fn set_status_led(&mut self, on: bool) {
        self.0.set_status_led(on);
    }

    pub(crate) fn delay_ms(&mut self, milliseconds: u32) {
        self.0.delay_ms(milliseconds);
    }

    #[cfg(feature = "abi-context-switch")]
    pub(crate) fn enable_scheduler_tick(&mut self, tick_hz: u32) -> bool {
        self.0.enable_scheduler_tick(tick_hz)
    }

    #[cfg(feature = "sdio")]
    pub(crate) fn take_sdio_reader(&mut self) -> Option<<f405::Board as Backend>::SdioReader> {
        self.0.take_sdio_reader()
    }

    #[cfg(feature = "usb-cdc")]
    pub(crate) fn take_usb_resources(&mut self) -> Option<UsbResources> {
        <f405::Board as Backend>::take_usb_resources(&mut self.0)
    }
}

#[cfg(feature = "usb-cdc")]
pub(crate) type UsbResources = <f405::Board as Backend>::UsbResources;

#[cfg(feature = "usb-cdc")]
pub(crate) fn unmask_usb_irq() {
    <f405::Board as Backend>::unmask_usb_irq();
}

#[cfg(feature = "usb-cdc")]
pub(crate) fn pend_usb_irq() {
    <f405::Board as Backend>::pend_usb_irq();
}

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use f405::MEMORY_PROFILE;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use f405::{ISOLATION_LAYOUT, activate_application_regions};

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use crate::security::mpu;

#[cfg(all(feature = "board-stm32f405-sd", not(feature = "abi-current")))]
pub(crate) use f405::APPLICATION_EXECUTION_SUPPORTED;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use f405::TARGET_PROFILE;

#[cfg(all(
    feature = "board-stm32f405-sd",
    feature = "abi-context-switch",
    feature = "abi-current"
))]
pub(crate) use f405::{CONTEXT_CAPACITY, SCHEDULER_PROFILE};

#[cfg(not(feature = "board-stm32f405-sd"))]
compile_error!("Select a supported Dali OS platform feature");
