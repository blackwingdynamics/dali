//! Typed text-mode display properties.

/// A text-cell position on a bounded display.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextPosition {
    /// Zero-based text column.
    pub column: usize,
    /// Zero-based text row.
    pub row: usize,
}

impl TextPosition {
    /// Creates a text-cell position.
    pub const fn new(column: usize, row: usize) -> Self {
        Self { column, row }
    }
}

/// Compile-time text grid properties for a display adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextModeProperties<const COLUMNS: usize, const ROWS: usize>;

impl<const COLUMNS: usize, const ROWS: usize> TextModeProperties<COLUMNS, ROWS> {
    /// Returns the configured text-column count.
    pub const fn columns(self) -> usize {
        COLUMNS
    }

    /// Returns the configured text-row count.
    pub const fn rows(self) -> usize {
        ROWS
    }
}
