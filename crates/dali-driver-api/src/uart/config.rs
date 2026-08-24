//! Hardware-neutral UART configuration types.

use crate::{DriverError, DriverResult};

/// A positive UART baud rate expressed in bits per second.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BaudRate(u32);

impl BaudRate {
    /// Creates a baud rate from a positive bits-per-second value.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::InvalidState`] when `bits_per_second` is zero.
    pub const fn from_bits_per_second(bits_per_second: u32) -> DriverResult<Self> {
        if bits_per_second == 0 {
            Err(DriverError::InvalidState)
        } else {
            Ok(Self(bits_per_second))
        }
    }

    /// Returns the configured bits-per-second value.
    pub const fn bits_per_second(self) -> u32 {
        self.0
    }
}

/// Number of data bits transmitted by a UART frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataBits {
    /// Five data bits.
    Five,
    /// Six data bits.
    Six,
    /// Seven data bits.
    Seven,
    /// Eight data bits.
    Eight,
    /// Nine data bits.
    Nine,
}

/// Optional parity bit policy for a UART frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Parity {
    /// Do not transmit a parity bit.
    None,
    /// Use even parity.
    Even,
    /// Use odd parity.
    Odd,
}

/// Number of stop bits transmitted by a UART frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopBits {
    /// One stop bit.
    One,
    /// Two stop bits.
    Two,
}

/// Complete UART framing configuration owned by the adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerialConfig {
    /// Configured line rate.
    pub baud_rate: BaudRate,
    /// Configured data width.
    pub data_bits: DataBits,
    /// Configured parity mode.
    pub parity: Parity,
    /// Configured stop-bit count.
    pub stop_bits: StopBits,
}

impl SerialConfig {
    /// Creates a validated UART framing configuration.
    pub const fn new(
        baud_rate: BaudRate,
        data_bits: DataBits,
        parity: Parity,
        stop_bits: StopBits,
    ) -> Self {
        Self {
            baud_rate,
            data_bits,
            parity,
            stop_bits,
        }
    }
}

/// Configures UART framing without exposing a platform register map.
pub trait SerialConfigure {
    /// Applies a complete UART framing configuration.
    ///
    /// # Errors
    ///
    /// Returns an adapter-specific [`DriverError`] when the port is owned by
    /// another client or the configuration is unsupported.
    fn configure(&mut self, config: SerialConfig) -> DriverResult<()>;
}
