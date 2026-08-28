//! Bounded text-mode diagnostics console for display adapters.

use crate::{DisplayDriver, DriverError, DriverResult, Duration, TextPosition};

const LINE_FEED: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';
const SPACE: u8 = b' ';

/// Fixed-capacity text console backed by a compile-time grid.
pub struct DiagnosticsConsole<const COLUMNS: usize, const ROWS: usize> {
    cells: [[u8; COLUMNS]; ROWS],
    cursor: TextPosition,
    line_clipped: bool,
    overflowed: bool,
    headless: bool,
}

impl<const COLUMNS: usize, const ROWS: usize> DiagnosticsConsole<COLUMNS, ROWS> {
    /// Creates an empty console with its cursor at the first cell.
    pub const fn new() -> Self {
        Self {
            cells: [[SPACE; COLUMNS]; ROWS],
            cursor: TextPosition::new(0, 0),
            line_clipped: false,
            overflowed: false,
            headless: false,
        }
    }

    /// Appends bytes using deterministic newline, clipping, and scrolling rules.
    pub fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            match byte {
                LINE_FEED => self.new_line(),
                CARRIAGE_RETURN => self.cursor.column = 0,
                _ if self.cursor.row >= ROWS || self.cursor.column >= COLUMNS => {
                    self.line_clipped = true;
                    self.overflowed = true;
                }
                _ => {
                    self.cells[self.cursor.row][self.cursor.column] = byte;
                    self.cursor.column += 1;
                }
            }
        }
    }

    /// Renders the bounded grid and silently enters headless mode if unavailable.
    pub fn render<D, const WIDTH: usize, const HEIGHT: usize>(
        &mut self,
        display: &mut D,
        timeout: Duration,
    ) -> DriverResult<()>
    where
        D: DisplayDriver<WIDTH, HEIGHT, COLUMNS, ROWS>,
    {
        if self.headless {
            return Ok(());
        }
        let result = self.render_grid(display, timeout);
        if matches!(result, Err(DriverError::DisplayUnavailable)) {
            self.headless = true;
            return Ok(());
        }
        result
    }

    /// Returns the current cursor position.
    pub const fn cursor(&self) -> TextPosition {
        self.cursor
    }

    /// Returns whether input exceeded the current line or grid capacity.
    pub const fn overflowed(&self) -> bool {
        self.overflowed
    }

    /// Returns whether the console has permanently selected headless operation.
    pub const fn is_headless(&self) -> bool {
        self.headless
    }

    /// Returns one text row for bounded inspection or rendering support.
    pub fn row(&self, row: usize) -> Option<&[u8; COLUMNS]> {
        self.cells.get(row)
    }

    fn new_line(&mut self) {
        self.cursor.column = 0;
        if self.cursor.row + 1 < ROWS {
            self.cursor.row += 1;
            self.line_clipped = false;
            return;
        }
        self.scroll();
        self.cursor.row = ROWS.saturating_sub(1);
        self.line_clipped = false;
    }

    fn scroll(&mut self) {
        if ROWS == 0 {
            self.overflowed = true;
            return;
        }
        let mut row = 1;
        while row < ROWS {
            self.cells[row - 1] = self.cells[row];
            row += 1;
        }
        self.cells[ROWS - 1] = [SPACE; COLUMNS];
        self.overflowed = true;
    }

    fn render_grid<D, const WIDTH: usize, const HEIGHT: usize>(
        &self,
        display: &mut D,
        timeout: Duration,
    ) -> DriverResult<()>
    where
        D: DisplayDriver<WIDTH, HEIGHT, COLUMNS, ROWS>,
    {
        display.clear(timeout)?;
        let mut row = 0;
        while row < ROWS {
            display.write_text(TextPosition::new(0, row), &self.cells[row], timeout)?;
            row += 1;
        }
        display.flush(timeout)
    }
}

impl<const COLUMNS: usize, const ROWS: usize> Default for DiagnosticsConsole<COLUMNS, ROWS> {
    fn default() -> Self {
        Self::new()
    }
}
