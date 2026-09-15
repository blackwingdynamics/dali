//! F405 USB FS resource ownership and host re-enumeration.

#[cfg(feature = "usb-install")]
use super::installer::InstallerBulkClass;
use core::cell::RefCell;
use critical_section::Mutex;
use dali_kernel_api::{UsbResetDelay, UsbResources as UsbResourceContract};
use stm32f4xx_hal::{gpio, pac, rcc::Clocks};
use usb_device::{bus::UsbBusAllocator, class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

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

const USB_VENDOR_ID: u16 = 0x1209;
const USB_PRODUCT_ID: u16 = 0xDA11;
const ENDPOINT_MEMORY_WORDS: usize = 1_024;
#[cfg(feature = "usb-install")]
const INSTALLATION_READ_BUFFER_SIZE: usize = 64;

#[unsafe(link_section = ".dma_buffer")]
static mut ENDPOINT_MEMORY: [u32; ENDPOINT_MEMORY_WORDS] = [0; ENDPOINT_MEMORY_WORDS];
static mut USB_BUS: Option<UsbBusAllocator<stm32f4xx_hal::otg_fs::UsbBusType>> = None;
static USB_SERIAL: Mutex<RefCell<Option<SerialPort<'static, stm32f4xx_hal::otg_fs::UsbBusType>>>> =
    Mutex::new(RefCell::new(None));
#[cfg(feature = "usb-install")]
static USB_INSTALL_SERIAL: Mutex<
    RefCell<Option<InstallerBulkClass<'static, stm32f4xx_hal::otg_fs::UsbBusType>>>,
> = Mutex::new(RefCell::new(None));
static USB_DEVICE: Mutex<RefCell<Option<UsbDevice<'static, stm32f4xx_hal::otg_fs::UsbBusType>>>> =
    Mutex::new(RefCell::new(None));

/// Initializes USB state owned entirely by the F405 backend.
pub(crate) fn initialize(board: &mut super::Board, force_reenumeration: bool) -> bool {
    let Some(resources) = board.take_usb_resources() else {
        return false;
    };
    struct Delay<'a>(&'a mut super::Board);
    impl dali_kernel_api::UsbResetDelay for Delay<'_> {
        fn delay_ms(&mut self, milliseconds: u32) {
            super::services::delay_ms(self.0, milliseconds);
        }
    }
    let bus = unsafe {
        let memory = core::slice::from_raw_parts_mut(
            core::ptr::addr_of_mut!(ENDPOINT_MEMORY).cast::<u32>(),
            ENDPOINT_MEMORY_WORDS,
        );
        resources.into_bus(memory)
    };
    unsafe {
        (*core::ptr::addr_of_mut!(USB_BUS)) = Some(bus);
        let Some(bus) = (*core::ptr::addr_of!(USB_BUS)).as_ref() else {
            return false;
        };
        let serial = SerialPort::new(bus);
        #[cfg(feature = "usb-install")]
        let installation_serial = InstallerBulkClass::new(bus);
        #[cfg(feature = "usb-install")]
        let device = UsbDeviceBuilder::new(bus, UsbVidPid(USB_VENDOR_ID, USB_PRODUCT_ID))
            .composite_with_iads()
            .build();
        #[cfg(not(feature = "usb-install"))]
        let device = UsbDeviceBuilder::new(bus, UsbVidPid(USB_VENDOR_ID, USB_PRODUCT_ID))
            .device_class(usbd_serial::USB_CLASS_CDC)
            .build();
        if force_reenumeration {
            let mut delay = Delay(board);
            struct DelayAdapter<'a>(&'a mut Delay<'a>);
            impl stm32f4xx_hal::hal_02::blocking::delay::DelayMs<u32> for DelayAdapter<'_> {
                fn delay_ms(&mut self, milliseconds: u32) {
                    self.0.delay_ms(milliseconds);
                }
            }
            let mut adapter = DelayAdapter(&mut delay);
            let _ = device.bus().force_reset(&mut adapter);
        }
        critical_section::with(|cs| {
            *USB_SERIAL.borrow(cs).borrow_mut() = Some(serial);
            *USB_DEVICE.borrow(cs).borrow_mut() = Some(device);
        });
        #[cfg(feature = "usb-install")]
        critical_section::with(|cs| {
            *USB_INSTALL_SERIAL.borrow(cs).borrow_mut() = Some(installation_serial);
        });
    }
    unmask_usb_irq();
    pend_usb_irq();
    true
}

/// Services the board-owned USB state and invokes the kernel queue callback.
pub(crate) fn service_irq(
    drain: fn(dali_usb::LinkState, &mut dyn dali_usb::ByteSink),
    receive: dali_kernel_api::UsbInstallationReceive,
) {
    static mut SERIAL: Option<SerialPort<'static, stm32f4xx_hal::otg_fs::UsbBusType>> = None;
    #[cfg(feature = "usb-install")]
    static mut INSTALL_SERIAL: Option<
        InstallerBulkClass<'static, stm32f4xx_hal::otg_fs::UsbBusType>,
    > = None;
    static mut DEVICE: Option<UsbDevice<'static, stm32f4xx_hal::otg_fs::UsbBusType>> = None;
    unsafe {
        if (*core::ptr::addr_of!(DEVICE)).is_none() {
            (*core::ptr::addr_of_mut!(DEVICE)) =
                critical_section::with(|cs| USB_DEVICE.borrow(cs).replace(None));
        }
        if (*core::ptr::addr_of!(SERIAL)).is_none() {
            (*core::ptr::addr_of_mut!(SERIAL)) =
                critical_section::with(|cs| USB_SERIAL.borrow(cs).replace(None));
        }
        #[cfg(feature = "usb-install")]
        if (*core::ptr::addr_of!(INSTALL_SERIAL)).is_none() {
            (*core::ptr::addr_of_mut!(INSTALL_SERIAL)) =
                critical_section::with(|cs| USB_INSTALL_SERIAL.borrow(cs).replace(None));
        }
        #[cfg(feature = "usb-install")]
        let (Some(device), Some(serial), Some(installation_serial)) = (
            (*core::ptr::addr_of_mut!(DEVICE)).as_mut(),
            (*core::ptr::addr_of_mut!(SERIAL)).as_mut(),
            (*core::ptr::addr_of_mut!(INSTALL_SERIAL)).as_mut(),
        ) else {
            return;
        };
        #[cfg(not(feature = "usb-install"))]
        let (Some(device), Some(serial)) = (
            (*core::ptr::addr_of_mut!(DEVICE)).as_mut(),
            (*core::ptr::addr_of_mut!(SERIAL)).as_mut(),
        ) else {
            return;
        };
        #[cfg(feature = "usb-install")]
        let _ = device.poll(&mut [serial, installation_serial]);
        #[cfg(not(feature = "usb-install"))]
        let _ = device.poll(&mut [serial]);
        #[cfg(feature = "usb-install")]
        receive_installation_bytes(installation_serial, receive);
        #[cfg(not(feature = "usb-install"))]
        let _ = receive;
        let link = dali_usb::LinkState::from_configured_and_open(
            device.state() == UsbDeviceState::Configured,
            serial.dtr(),
        );
        drain(link, &mut Sink { serial });
    }
}

#[cfg(feature = "usb-install")]
fn receive_installation_bytes(
    serial: &mut InstallerBulkClass<'static, stm32f4xx_hal::otg_fs::UsbBusType>,
    receive: fn(&[u8]),
) {
    let mut buffer = [0; INSTALLATION_READ_BUFFER_SIZE];
    if let Ok(length) = serial.read(&mut buffer)
        && length > 0
    {
        receive(&buffer[..length]);
    }
}

struct Sink<'a> {
    serial: &'a mut SerialPort<'static, stm32f4xx_hal::otg_fs::UsbBusType>,
}

impl dali_usb::ByteSink for Sink<'_> {
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

/// Enables the board's USB interrupt after the CDC backend is initialized.
pub(crate) fn unmask_usb_irq() {
    // SAFETY: The backend initializes USB state before unmasking its sole IRQ.
    unsafe { cortex_m::peripheral::NVIC::unmask(pac::Interrupt::OTG_FS) };
}

/// Wakes the board's USB backend after a main-context log enqueue.
pub(crate) fn pend_usb_irq() {
    // SAFETY: PENDING is a software wake-up for the initialized USB owner.
    cortex_m::peripheral::NVIC::pend(pac::Interrupt::OTG_FS);
}
