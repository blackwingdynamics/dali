//! F405 USB FS resource ownership and host re-enumeration.

use crate::logging::usb_cdc::{UsbBusReset, UsbResetDelay, UsbResources as UsbResourceContract};
use stm32f4xx_hal::{gpio, pac, rcc::Clocks};

/// USB FS resources connected to the board's USB-C data pins.
pub struct UsbResources {
    /// USB global registers.
    pub global: pac::OTG_FS_GLOBAL,
    /// USB device registers.
    pub device: pac::OTG_FS_DEVICE,
    /// USB power and clock registers.
    pub power_clock: pac::OTG_FS_PWRCLK,
    /// USB D- pin on PA11.
    pub dm: gpio::gpioa::PA11<gpio::Alternate<10>>,
    /// USB D+ pin on PA12.
    pub dp: gpio::gpioa::PA12<gpio::Alternate<10>>,
    /// Frozen clocks used to configure the USB peripheral.
    pub clocks: Clocks,
}

impl UsbResourceContract for UsbResources {
    type Bus = stm32f4xx_hal::otg_fs::UsbBusType;

    fn into_bus(
        self,
        endpoint_memory: &'static mut [u32],
    ) -> usb_device::bus::UsbBusAllocator<Self::Bus> {
        let usb = stm32f4xx_hal::otg_fs::USB::new(
            (self.global, self.device, self.power_clock),
            (self.dm, self.dp),
            &self.clocks,
        );
        stm32f4xx_hal::otg_fs::UsbBus::new(usb, endpoint_memory)
    }
}

impl UsbBusReset for stm32f4xx_hal::otg_fs::UsbBusType {
    fn force_reenumeration(&self, delay: &mut dyn UsbResetDelay) {
        struct DelayAdapter<'a> {
            delay: &'a mut dyn UsbResetDelay,
        }

        impl stm32f4xx_hal::hal_02::blocking::delay::DelayMs<u32> for DelayAdapter<'_> {
            fn delay_ms(&mut self, milliseconds: u32) {
                self.delay.delay_ms(milliseconds);
            }
        }

        let mut adapter = DelayAdapter { delay };
        let _ = self.force_reset(&mut adapter);
    }
}
