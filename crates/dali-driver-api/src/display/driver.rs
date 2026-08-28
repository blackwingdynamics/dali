//! Bounded display operation contract.

use crate::{BoundedTimeout, DriverResult, Duration};

use super::{DisplayDimensions, TextModeProperties, TextPosition};

/// Provides bounded lifecycle, text, and flush operations for one display.
pub trait DisplayDriver<
    const WIDTH: usize,
    const HEIGHT: usize,
    const COLUMNS: usize,
    const ROWS: usize,
>: BoundedTimeout
{
    /// Initializes the display controller before the timeout expires.
    fn initialize(&mut self, timeout: Duration) -> DriverResult<()>;

    /// Flushes the adapter-owned display state before the timeout expires.
    fn flush(&mut self, timeout: Duration) -> DriverResult<()>;

    /// Clears the display state before the timeout expires.
    fn clear(&mut self, timeout: Duration) -> DriverResult<()>;

    /// Writes bounded text beginning at a text-cell position.
    fn write_text(
        &mut self,
        position: TextPosition,
        text: &[u8],
        timeout: Duration,
    ) -> DriverResult<()>;

    /// Resets the display controller before the timeout expires.
    fn reset(&mut self, timeout: Duration) -> DriverResult<()>;

    /// Returns the compile-time pixel dimensions of this adapter.
    fn dimensions(&self) -> DisplayDimensions<WIDTH, HEIGHT> {
        DisplayDimensions
    }

    /// Returns the compile-time text-grid properties of this adapter.
    fn text_properties(&self) -> TextModeProperties<COLUMNS, ROWS> {
        TextModeProperties
    }
}
