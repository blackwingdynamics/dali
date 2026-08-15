//! WeAct Studio STM32F405RGT6 Core Board support.

use dali_targets::TARGET_F405;
use stm32f4xx_hal::{gpio, pac, prelude::*, rcc::Clocks, time::Hertz, timer::SysDelay};

/// The current MVP board supports AMRN native application execution.
pub const APPLICATION_EXECUTION_SUPPORTED: bool = true;

/// System clock target derived from the declarative F405 target profile.
pub const SYSTEM_CLOCK_HZ: u32 = TARGET_F405.clock.system_hz;
/// Unit conversion used by the boot log's human-readable clock value.
const HZ_PER_MHZ: u32 = 1_000_000;
/// System clock in megahertz for the common board facade.
pub const SYSTEM_CLOCK_MHZ: u32 = SYSTEM_CLOCK_HZ / HZ_PER_MHZ;

const _: () = assert!(TARGET_F405.amrn_target_id == dali_amrn::TARGET_ID);
const _: () = assert!(TARGET_F405.abi_version == dali_amrn::ABI_VERSION);
const _: () = assert!(
    TARGET_F405.memory.application_origin == dali_amrn::LOAD_ADDRESS
        && TARGET_F405.memory.application_length == dali_amrn::MAX_PAYLOAD_SIZE as u32
);

/// Status LED output pin on the active-high PB2 LED.
pub type StatusLed = gpio::gpiob::PB2<gpio::Output<gpio::PushPull>>;

/// USB FS resources connected to the board's USB-C data pins.
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
    pub clocks: Clocks,
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

/// SDIO pins owned by the kernel after board initialization.
pub type SdioPins = (
    gpio::gpioc::PC12,
    gpio::gpiod::PD2,
    gpio::gpioc::PC8,
    gpio::gpioc::PC9,
    gpio::gpioc::PC10,
    gpio::gpioc::PC11,
);

/// Peripherals owned by the kernel after board initialization.
pub struct Board {
    /// Blocking delay driven by the initialized SysTick timer.
    pub delay: SysDelay,
    /// WeAct board status LED on active-high PB2.
    pub status_led: StatusLed,
    /// Hardware SDIO 4-bit pins for the on-board microSD socket.
    sdio_pins: Option<SdioPins>,
    /// SDIO peripheral reserved for the storage driver.
    sdio: Option<pac::SDIO>,
    /// Frozen clock configuration required to initialize SDIO.
    clocks: Clocks,
    /// USB FS resources reserved for the CDC logging backend.
    #[cfg(feature = "usb-cdc")]
    usb: Option<UsbResources>,
}

impl Board {
    /// Transfers the SDIO resources to the storage driver.
    pub fn take_sdio_resources(&mut self) -> Option<(pac::SDIO, SdioPins, &Clocks)> {
        let peripheral = self.sdio.take()?;
        let pins = self.sdio_pins.take()?;
        Some((peripheral, pins, &self.clocks))
    }

    /// Transfers USB FS resources to the logging backend.
    #[cfg(feature = "usb-cdc")]
    pub fn take_usb_resources(&mut self) -> Option<UsbResources> {
        self.usb.take()
    }
}

/// Takes singleton peripherals and initializes the STM32F405 board hardware.
pub fn initialize(device: pac::Peripherals, core: cortex_m::Peripherals) -> Board {
    let rcc = device.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(Hertz::from_raw(TARGET_F405.clock.hse_hz))
        .sysclk(Hertz::from_raw(TARGET_F405.clock.system_hz))
        .hclk(Hertz::from_raw(TARGET_F405.clock.system_hz))
        .pclk1(Hertz::from_raw(TARGET_F405.clock.pclk1_hz))
        .pclk2(Hertz::from_raw(TARGET_F405.clock.pclk2_hz));
    #[cfg(feature = "usb-cdc")]
    let clocks = clocks.require_pll48clk();
    let clocks = clocks.freeze();
    let delay = core.SYST.delay(&clocks);

    let gpiob = device.GPIOB.split();
    #[cfg(feature = "usb-cdc")]
    let gpioa = device.GPIOA.split();
    let gpioc = device.GPIOC.split();
    let gpiod = device.GPIOD.split();
    let mut status_led = gpiob.pb2.into_push_pull_output();
    status_led.set_low();

    let sdio_pins = (
        gpioc.pc12,
        gpiod.pd2.internal_pull_up(true),
        gpioc.pc8.internal_pull_up(true),
        gpioc.pc9.internal_pull_up(true),
        gpioc.pc10.internal_pull_up(true),
        gpioc.pc11.internal_pull_up(true),
    );

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
        sdio_pins: Some(sdio_pins),
        sdio: Some(device.SDIO),
        clocks,
        #[cfg(feature = "usb-cdc")]
        usb,
    }
}

/// Sets the active-high F405 board LED to the requested logical state.
pub fn set_status_led(board: &mut Board, on: bool) {
    if on {
        board.status_led.set_high();
    } else {
        board.status_led.set_low();
    }
}
