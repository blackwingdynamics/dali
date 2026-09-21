//! Bounded SSD1306/SH1106 adapter at the F405 platform boundary.

use super::F405I2c;
use dali_driver_api::{
    BoundedTimeout, DisplayDriver, DriverError, DriverResult, Duration, I2cAddress, I2cDriver,
    TextPosition,
};
use dali_targets::DisplayProfile;

/// SSD1306/SH1106 command byte selecting the command stream.
const CONTROL_COMMAND: u8 = 0x00;
/// SSD1306/SH1106 command byte selecting display data.
const CONTROL_DATA: u8 = 0x40;
/// SSD1306/SH1106 command disabling the panel while it is configured.
const DISPLAY_OFF: u8 = 0xAE;
/// SSD1306/SH1106 command enabling the panel after configuration.
const DISPLAY_ON: u8 = 0xAF;
/// SSD1306/SH1106 command selecting page addressing mode.
const PAGE_ADDRESSING_MODE: u8 = 0x20;
/// SSD1306/SH1106 page-address command base.
const PAGE_ADDRESS_BASE: u8 = 0xB0;
/// SSD1306/SH1106 lower-column command base.
const LOWER_COLUMN_BASE: u8 = 0x00;
/// SSD1306/SH1106 upper-column command base.
const UPPER_COLUMN_BASE: u8 = 0x10;
/// Accepted SSD1306/SH1106 controller identifiers.
const SUPPORTED_CONTROLLERS: &[&str] = &["SSD1306", "SH1106"];
/// Mask selecting the lower column command nibble.
const COLUMN_NIBBLE_MASK: u8 = 0x0F;
/// Shift selecting the upper column command nibble.
const COLUMN_NIBBLE_SHIFT: u8 = 4;

/// F405 OLED adapter using the existing kernel-owned I2C1 bus.
pub struct F405Ssd1306<
    const WIDTH: usize,
    const HEIGHT: usize,
    const COLUMNS: usize,
    const ROWS: usize,
> {
    bus: F405I2c,
    profile: DisplayProfile,
    initialized: bool,
    available: bool,
}

impl<const WIDTH: usize, const HEIGHT: usize, const COLUMNS: usize, const ROWS: usize>
    F405Ssd1306<WIDTH, HEIGHT, COLUMNS, ROWS>
{
    /// Creates an uninitialized OLED adapter from manifest-owned metadata.
    pub(crate) const fn new(bus: F405I2c, profile: DisplayProfile) -> Self {
        Self {
            bus,
            profile,
            initialized: false,
            available: true,
        }
    }

    /// Borrows the single board-owned I2C bus for an acceptance operation.
    #[cfg(feature = "driver-hardware-test")]
    pub(crate) fn bus_mut(&mut self) -> &mut F405I2c {
        &mut self.bus
    }

    pub fn address(&self) -> I2cAddress {
        I2cAddress::new(self.profile.i2c_address)
    }

    pub fn controller_supported(&self) -> bool {
        SUPPORTED_CONTROLLERS.contains(&self.profile.controller)
    }

    pub fn transfer(&mut self, bytes: &[u8], timeout: Duration) -> DriverResult<()> {
        if !self.available {
            return Err(DriverError::DisplayUnavailable);
        }
        self.bus.acquire()?;
        let result = self.bus.write(self.address(), bytes, timeout);
        let release = self.bus.release();
        match result {
            Err(DriverError::Nack) => Err(DriverError::DisplayUnavailable),
            Err(error) => Err(error),
            Ok(()) => release,
        }
    }

    pub fn probe_address(&mut self, timeout: Duration) -> DriverResult<()> {
        match self.transfer(&[], timeout) {
            Err(DriverError::Nack) | Err(DriverError::DisplayUnavailable) => {
                Err(DriverError::DisplayUnavailable)
            }
            result => result,
        }
    }

    pub fn set_cursor(&mut self, position: TextPosition, timeout: Duration) -> DriverResult<()> {
        if position.column >= COLUMNS || position.row >= ROWS {
            return Err(DriverError::InvalidState);
        }
        let column = position
            .column
            .checked_mul(self.profile.text_cell_width)
            .ok_or(DriverError::InvalidState)?;
        let lower = LOWER_COLUMN_BASE | (column as u8 & COLUMN_NIBBLE_MASK);
        let upper =
            UPPER_COLUMN_BASE | ((column as u8 >> COLUMN_NIBBLE_SHIFT) & COLUMN_NIBBLE_MASK);
        let commands = [
            CONTROL_COMMAND,
            PAGE_ADDRESS_BASE | (position.row as u8),
            lower,
            upper,
        ];
        self.transfer(&commands, timeout)
    }

    pub fn write_byte(&mut self, byte: u8, timeout: Duration) -> DriverResult<()> {
        self.transfer(&[CONTROL_DATA, byte], timeout)
    }
}

impl<const WIDTH: usize, const HEIGHT: usize, const COLUMNS: usize, const ROWS: usize>
    BoundedTimeout for F405Ssd1306<WIDTH, HEIGHT, COLUMNS, ROWS>
{
    fn max_timeout(&self) -> Duration {
        Duration::from_ticks(self.profile.timeout_ticks)
    }
}

impl<const WIDTH: usize, const HEIGHT: usize, const COLUMNS: usize, const ROWS: usize>
    DisplayDriver<WIDTH, HEIGHT, COLUMNS, ROWS> for F405Ssd1306<WIDTH, HEIGHT, COLUMNS, ROWS>
{
    fn initialize(&mut self, timeout: Duration) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        if self.profile.width != WIDTH
            || self.profile.height != HEIGHT
            || !self.controller_supported()
        {
            return Err(DriverError::DisplayUnavailable);
        }
        if let Err(error) = self.probe_address(timeout) {
            if matches!(error, DriverError::DisplayUnavailable) {
                self.available = false;
            }
            return Err(error);
        }
        self.transfer(
            &[
                CONTROL_COMMAND,
                DISPLAY_OFF,
                PAGE_ADDRESSING_MODE,
                DISPLAY_ON,
            ],
            timeout,
        )?;
        self.initialized = true;
        self.available = true;
        Ok(())
    }

    fn flush(&mut self, timeout: Duration) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        if !self.initialized {
            return if self.available {
                Err(DriverError::InvalidState)
            } else {
                Err(DriverError::DisplayUnavailable)
            };
        }
        Ok(())
    }

    fn clear(&mut self, timeout: Duration) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        if !self.initialized {
            return if self.available {
                Err(DriverError::InvalidState)
            } else {
                Err(DriverError::DisplayUnavailable)
            };
        }
        self.transfer(&[CONTROL_COMMAND, DISPLAY_OFF, DISPLAY_ON], timeout)
    }

    fn write_text(
        &mut self,
        position: TextPosition,
        text: &[u8],
        timeout: Duration,
    ) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        if !self.initialized {
            return if self.available {
                Err(DriverError::InvalidState)
            } else {
                Err(DriverError::DisplayUnavailable)
            };
        }
        let mut cursor = position;
        for byte in text {
            if cursor.row >= ROWS {
                break;
            }
            self.set_cursor(cursor, timeout)?;
            self.write_byte(*byte, timeout)?;
            cursor.column += 1;
            if cursor.column == COLUMNS {
                cursor.column = 0;
                cursor.row += 1;
            }
        }
        Ok(())
    }

    fn reset(&mut self, timeout: Duration) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        self.transfer(&[CONTROL_COMMAND, DISPLAY_OFF], timeout)?;
        self.initialized = false;
        Ok(())
    }
}
