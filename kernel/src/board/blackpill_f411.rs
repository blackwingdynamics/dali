//! WeAct BlackPill STM32F411 board support.

use stm32f4xx_hal::{gpio, pac, prelude::*, timer::SysDelay};

/// System clock target in megahertz for the STM32F411 MVP board.
pub const SYSTEM_CLOCK_MHZ: u32 = 100;

/// Status LED output pin after board initialization.
pub type StatusLed = gpio::gpioc::PC13<gpio::Output<gpio::PushPull>>;

/// USB FS resources connected to the BlackPill USB data pins.
#[cfg(feature = "usb-cdc")]
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
    pub clocks: stm32f4xx_hal::rcc::Clocks,
}

#[cfg(feature = "usb-cdc")]
impl crate::logging::usb_cdc::UsbResources for UsbResources {
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

/// Peripherals owned by the kernel after board initialization.
pub struct Board {
    /// Blocking delay driven by the initialized SysTick timer.
    pub delay: SysDelay,
    /// WeAct BlackPill status LED on PC13.
    pub status_led: StatusLed,
    /// USB FS resources reserved for the CDC logging backend.
    #[cfg(feature = "usb-cdc")]
    usb: Option<UsbResources>,
}

impl Board {
    /// Transfers USB FS resources to the logging backend.
    #[cfg(feature = "usb-cdc")]
    pub fn take_usb_resources(&mut self) -> Option<UsbResources> {
        self.usb.take()
    }
}

/// Takes singleton peripherals and initializes the BlackPill MVP hardware.
pub fn initialize(device: pac::Peripherals, core: cortex_m::Peripherals) -> Board {
    let rcc = device.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(SYSTEM_CLOCK_MHZ.MHz());
    #[cfg(feature = "usb-cdc")]
    let clocks = clocks.require_pll48clk();
    let clocks = clocks.freeze();
    let delay = core.SYST.delay(&clocks);
    #[cfg(feature = "usb-cdc")]
    let gpioa = device.GPIOA.split();
    let status_led = device.GPIOC.split().pc13.into_push_pull_output();

    #[cfg(feature = "usb-cdc")]
    let usb = Some(UsbResources {
        global: device.OTG_FS_GLOBAL,
        device: device.OTG_FS_DEVICE,
        power_clock: device.OTG_FS_PWRCLK,
        dm: gpioa.pa11.into_alternate::<10>(),
        dp: gpioa.pa12.into_alternate::<10>(),
        clocks,
    });

    Board {
        delay,
        status_led,
        #[cfg(feature = "usb-cdc")]
        usb,
    }
}

/// Sets the active-low BlackPill LED to the requested logical state.
pub fn set_status_led(board: &mut Board, on: bool) {
    if on {
        board.status_led.set_low();
    } else {
        board.status_led.set_high();
    }
}
