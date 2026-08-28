//! F405 board metadata, type aliases, and compile-time contract checks.

#[cfg(feature = "display-oled")]
use super::super::drivers::ssd1306::F405Ssd1306;
use super::super::drivers::{F405ExtiPin, F405GpioPin};
#[cfg(feature = "abi-current")]
use dali_targets::MemoryProfile;
use dali_targets::TARGET_F405;
use stm32f4xx_hal::gpio;

/// First planned single-application F405 isolation layout.
pub const ISOLATION_LAYOUT: Option<crate::security::mpu::IsolationLayout> =
    crate::security::mpu::IsolationLayout::from_memory(TARGET_F405.memory);
const _: () = assert!(ISOLATION_LAYOUT.is_some());

/// Activates unprivileged application permissions after the loader finishes.
#[cfg(feature = "abi-mpu")]
pub fn activate_application_regions(slot: dali_targets::IsolationSlot) -> bool {
    let Some(layout) =
        crate::security::mpu::IsolationLayout::from_memory_for_slot(TARGET_F405.memory, slot)
    else {
        return false;
    };
    crate::security::mpu::activate_application_regions(layout);
    true
}

/// The current MVP board supports AMRN native application execution.
#[cfg(not(feature = "abi-current"))]
pub const APPLICATION_EXECUTION_SUPPORTED: bool = true;

/// System clock target derived from the declarative F405 target profile.
pub const SYSTEM_CLOCK_HZ: u32 = TARGET_F405.clock.system_hz;
#[cfg(feature = "abi-current")]
/// Memory layout selected by the F405 target profile.
pub const MEMORY_PROFILE: MemoryProfile = TARGET_F405.memory;
/// Unit conversion used by the boot log's human-readable clock value.
const HZ_PER_MHZ: u32 = 1_000_000;
/// System clock in megahertz for the common platform facade.
pub const SYSTEM_CLOCK_MHZ: u32 = SYSTEM_CLOCK_HZ / HZ_PER_MHZ;
const _: () = assert!(TARGET_F405.amrn_target_id == dali_amrn::TARGET_ID);
#[cfg(feature = "abi-current")]
const _: () = assert!(crate::abi::CURRENT_VERSION == dali_amrn::v2::ABI_VERSION);
#[cfg(not(feature = "abi-current"))]
const _: () = assert!(TARGET_F405.abi_version == crate::abi::CURRENT_VERSION);
const _: () = assert!(
    TARGET_F405.memory.application_origin == dali_amrn::LOAD_ADDRESS
        && TARGET_F405.memory.application_length == dali_amrn::MAX_PAYLOAD_SIZE as u32
);

/// Port selected by the board manifest for the active-high status LED.
const STATUS_LED_PORT: char = 'B';
/// Pin selected by the board manifest for the active-high status LED.
const STATUS_LED_PIN: u8 = 2;

/// Status LED output pin on the active-high PB2 LED.
pub type StatusLed = F405GpioPin<STATUS_LED_PORT, STATUS_LED_PIN>;

/// Port selected by the board manifest for the active-low user key.
const USER_KEY_PORT: char = TARGET_F405.user_key.port;
/// Pin selected by the board manifest for the active-low user key.
const USER_KEY_PIN: u8 = TARGET_F405.user_key.pin;

/// Board user-key input bound to the manifest-selected EXTI line.
pub type UserKey = F405ExtiPin<USER_KEY_PORT, USER_KEY_PIN>;

/// Optional F405 OLED type selected by the manifest-owned geometry.
#[cfg(feature = "display-oled")]
pub type OledDisplay = F405Ssd1306<
    { TARGET_F405.display.width },
    { TARGET_F405.display.height },
    { TARGET_F405.display.width / TARGET_F405.display.text_cell_width },
    { TARGET_F405.display.height / TARGET_F405.display.text_cell_height },
>;

/// Number of text columns derived from the manifest-owned OLED geometry.
#[cfg(feature = "display-oled")]
pub const DISPLAY_COLUMNS: usize = TARGET_F405.display.width / TARGET_F405.display.text_cell_width;
/// Number of text rows derived from the manifest-owned OLED geometry.
#[cfg(feature = "display-oled")]
pub const DISPLAY_ROWS: usize = TARGET_F405.display.height / TARGET_F405.display.text_cell_height;

/// SDIO pins owned by the kernel after board initialization.
pub type SdioPins = (
    gpio::gpioc::PC12,
    gpio::gpiod::PD2,
    gpio::gpioc::PC8,
    gpio::gpioc::PC9,
    gpio::gpioc::PC10,
    gpio::gpioc::PC11,
);
