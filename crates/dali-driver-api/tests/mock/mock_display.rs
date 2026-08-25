use dali_driver_api::{
    BoundedTimeout, DisplayDriver, DriverError, DriverResult, Duration, TextPosition,
};

/// Recorded display operation used to verify bounded command sequencing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayCommand {
    /// Display initialization was requested.
    Initialize,
    /// Display state was cleared.
    Clear,
    /// Text was written to the display state.
    WriteText,
    /// Display state was flushed.
    Flush,
    /// Display reset was requested.
    Reset,
}

/// Fixed-capacity host mock for the hardware-neutral display contract.
pub struct MockDisplay<
    const WIDTH: usize,
    const HEIGHT: usize,
    const COLUMNS: usize,
    const ROWS: usize,
    const BUFFER_CAPACITY: usize,
    const COMMAND_CAPACITY: usize,
> {
    /// Maximum accepted operation timeout.
    pub max_timeout: Duration,
    /// Fixed text/framebuffer storage owned by the mock.
    pub buffer: [u8; BUFFER_CAPACITY],
    /// Recorded command sequence.
    pub commands: [Option<DisplayCommand>; COMMAND_CAPACITY],
    /// Number of recorded commands.
    pub command_count: usize,
    /// Whether the display can be addressed.
    pub connected: bool,
    /// Whether the next operation should time out.
    pub timed_out: bool,
    /// Whether the next operation should report a bus error.
    pub bus_error: bool,
    initialized: bool,
}

impl<
    const WIDTH: usize,
    const HEIGHT: usize,
    const COLUMNS: usize,
    const ROWS: usize,
    const BUFFER_CAPACITY: usize,
    const COMMAND_CAPACITY: usize,
> MockDisplay<WIDTH, HEIGHT, COLUMNS, ROWS, BUFFER_CAPACITY, COMMAND_CAPACITY>
{
    /// Creates an available, empty display mock.
    pub const fn new(max_timeout: Duration) -> Self {
        Self {
            max_timeout,
            buffer: [0; BUFFER_CAPACITY],
            commands: [None; COMMAND_CAPACITY],
            command_count: 0,
            connected: true,
            timed_out: false,
            bus_error: false,
            initialized: false,
        }
    }

    fn begin(&self, timeout: Duration, require_initialized: bool) -> DriverResult<()> {
        self.validate_timeout(timeout)?;
        if !self.connected {
            return Err(DriverError::DisplayUnavailable);
        }
        if self.timed_out {
            return Err(DriverError::Timeout);
        }
        if self.bus_error {
            return Err(DriverError::BusError);
        }
        if require_initialized && !self.initialized {
            return Err(DriverError::InvalidState);
        }
        Ok(())
    }

    fn record(&mut self, command: DisplayCommand) -> DriverResult<()> {
        if self.command_count == COMMAND_CAPACITY {
            return Err(DriverError::InvalidBuffer);
        }
        self.commands[self.command_count] = Some(command);
        self.command_count += 1;
        Ok(())
    }

    fn required_capacity() -> Option<usize> {
        COLUMNS.checked_mul(ROWS)
    }
}

impl<
    const WIDTH: usize,
    const HEIGHT: usize,
    const COLUMNS: usize,
    const ROWS: usize,
    const BUFFER_CAPACITY: usize,
    const COMMAND_CAPACITY: usize,
> BoundedTimeout for MockDisplay<WIDTH, HEIGHT, COLUMNS, ROWS, BUFFER_CAPACITY, COMMAND_CAPACITY>
{
    fn max_timeout(&self) -> Duration {
        self.max_timeout
    }
}

impl<
    const WIDTH: usize,
    const HEIGHT: usize,
    const COLUMNS: usize,
    const ROWS: usize,
    const BUFFER_CAPACITY: usize,
    const COMMAND_CAPACITY: usize,
> DisplayDriver<WIDTH, HEIGHT, COLUMNS, ROWS>
    for MockDisplay<WIDTH, HEIGHT, COLUMNS, ROWS, BUFFER_CAPACITY, COMMAND_CAPACITY>
{
    fn initialize(&mut self, timeout: Duration) -> DriverResult<()> {
        self.begin(timeout, false)?;
        if Self::required_capacity().is_none_or(|capacity| capacity > BUFFER_CAPACITY) {
            return Err(DriverError::InvalidBuffer);
        }
        self.record(DisplayCommand::Initialize)?;
        self.initialized = true;
        Ok(())
    }

    fn flush(&mut self, timeout: Duration) -> DriverResult<()> {
        self.begin(timeout, true)?;
        self.record(DisplayCommand::Flush)
    }

    fn clear(&mut self, timeout: Duration) -> DriverResult<()> {
        self.begin(timeout, true)?;
        self.buffer.fill(0);
        self.record(DisplayCommand::Clear)
    }

    fn write_text(
        &mut self,
        position: TextPosition,
        text: &[u8],
        timeout: Duration,
    ) -> DriverResult<()> {
        self.begin(timeout, true)?;
        if Self::required_capacity().is_none_or(|capacity| capacity > BUFFER_CAPACITY) {
            return Err(DriverError::InvalidBuffer);
        }
        let mut column = position.column;
        let mut row = position.row;
        for byte in text {
            if row >= ROWS {
                break;
            }
            if column >= COLUMNS {
                column = 0;
                row += 1;
                if row >= ROWS {
                    break;
                }
            }
            let index = row * COLUMNS + column;
            self.buffer[index] = *byte;
            column += 1;
        }
        self.record(DisplayCommand::WriteText)
    }

    fn reset(&mut self, timeout: Duration) -> DriverResult<()> {
        self.begin(timeout, false)?;
        self.initialized = false;
        self.record(DisplayCommand::Reset)
    }
}
