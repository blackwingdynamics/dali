mod mock;

use core::sync::atomic::{AtomicU8, Ordering};

use dali_driver_api::{
    BaudRate, BoundedTimeout, CountDown, DataBits, DiagnosticsConsole, DisplayDriver, DriverError,
    Duration, GpioMode, I2cAddress, I2cDriver, InputPin, InterruptPin, InterruptTrigger, OutputPin,
    Parity, PinMode, SerialConfig, SerialConfigure, SerialOwnership, SerialRead, SerialWrite,
    SpiBusOwnership, SpiDeviceId, SpiDeviceSelect, SpiTransfer, StopBits, TextPosition,
    TimerDriver,
};
use mock::{
    DisplayCommand, I2cOperation, MockDisplay, MockGpio, MockI2c, MockSerial, MockSpi, MockTimer,
};

const TIMER_LIMIT: Duration = Duration::from_ticks(100);
const VALID_TIMEOUT: Duration = Duration::from_ticks(10);
const DISPLAY_WIDTH: usize = 128;
const DISPLAY_HEIGHT: usize = 64;
const DISPLAY_COLUMNS: usize = 4;
const DISPLAY_ROWS: usize = 2;
const DISPLAY_BUFFER_CAPACITY: usize = DISPLAY_COLUMNS * DISPLAY_ROWS;
const DISPLAY_COMMAND_CAPACITY: usize = 8;

type TestDisplay = MockDisplay<
    DISPLAY_WIDTH,
    DISPLAY_HEIGHT,
    DISPLAY_COLUMNS,
    DISPLAY_ROWS,
    DISPLAY_BUFFER_CAPACITY,
    DISPLAY_COMMAND_CAPACITY,
>;

static CALLBACK_COUNT: AtomicU8 = AtomicU8::new(0);

fn record_interrupt() {
    CALLBACK_COUNT.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn gpio_contracts_cover_mode_level_and_interrupt_lifecycle() {
    let mut gpio = MockGpio::new();

    gpio.set_mode(GpioMode::Output).unwrap();
    gpio.set_high().unwrap();
    assert!(gpio.is_high().unwrap());
    gpio.toggle().unwrap();
    assert!(!gpio.is_high().unwrap());

    gpio.enable_interrupt(InterruptTrigger::RisingEdge, Some(record_interrupt))
        .unwrap();
    gpio.pending = true;
    assert!(gpio.take_pending().unwrap());
    assert!(!gpio.take_pending().unwrap());
    assert_eq!(CALLBACK_COUNT.load(Ordering::Relaxed), 1);
    gpio.disable_interrupt().unwrap();

    gpio.interrupt_supported = false;
    assert_eq!(
        gpio.enable_interrupt(InterruptTrigger::BothEdges, None),
        Err(DriverError::Unsupported)
    );
}

#[test]
fn serial_contracts_bound_partial_transfers_and_failures() {
    let mut serial = MockSerial::<4>::new(TIMER_LIMIT);
    serial.queue_rx(&[1, 2, 3]).unwrap();
    let mut received = [0; 2];

    assert_eq!(serial.read(&mut received, VALID_TIMEOUT), Ok(2));
    assert_eq!(received, [1, 2]);
    assert_eq!(serial.write(&[4, 5], VALID_TIMEOUT), Ok(2));
    assert_eq!(serial.tx[..2], [4, 5]);
    assert_eq!(
        serial.write(&[6, 7, 8], VALID_TIMEOUT),
        Err(DriverError::InvalidBuffer)
    );
    serial.would_block = true;
    assert_eq!(serial.flush(VALID_TIMEOUT), Err(DriverError::WouldBlock));
    serial.would_block = false;
    serial.parity_error = true;
    assert_eq!(
        serial.read(&mut received, VALID_TIMEOUT),
        Err(DriverError::Parity)
    );
    serial.parity_error = false;
    serial.overrun = true;
    assert_eq!(
        serial.read(&mut received, VALID_TIMEOUT),
        Err(DriverError::Overrun)
    );
    serial.overrun = false;
    serial.connected = false;
    assert_eq!(
        serial.read(&mut received, VALID_TIMEOUT),
        Err(DriverError::Disconnected)
    );
    assert_eq!(
        serial.write(&[9], VALID_TIMEOUT),
        Err(DriverError::Disconnected)
    );
    assert_eq!(serial.flush(VALID_TIMEOUT), Err(DriverError::Disconnected));
    serial.connected = true;
    serial.timed_out = true;
    assert_eq!(
        serial.read(&mut received, VALID_TIMEOUT),
        Err(DriverError::Timeout)
    );
    assert_eq!(serial.write(&[9], VALID_TIMEOUT), Err(DriverError::Timeout));
    assert_eq!(serial.flush(VALID_TIMEOUT), Err(DriverError::Timeout));
}

#[test]
fn spi_contract_bounds_transactions_and_propagates_nack() {
    let mut spi = MockSpi::<4>::new(TIMER_LIMIT);
    spi.set_response(&[9, 8]).unwrap();
    let mut transfer = [0; 2];

    SpiTransfer::transfer(&mut spi, &mut transfer, VALID_TIMEOUT).unwrap();
    assert_eq!(transfer, [9, 8]);
    let mut short = [0; 1];
    assert_eq!(
        spi.transfer(&mut short, VALID_TIMEOUT),
        Err(DriverError::InvalidBuffer)
    );
    spi.nack = true;
    assert_eq!(
        spi.transfer(&mut transfer, VALID_TIMEOUT),
        Err(DriverError::Nack)
    );
    spi.nack = false;
    spi.arbitration_lost = true;
    assert_eq!(
        spi.transfer(&mut transfer, VALID_TIMEOUT),
        Err(DriverError::ArbitrationLost)
    );
}

#[test]
fn timer_timeout_and_spi_stall_paths_remain_bounded_and_recoverable() {
    let mut timer = MockTimer::new(TIMER_LIMIT);
    assert_eq!(
        TimerDriver::start(&mut timer, Duration::from_ticks(0)),
        Err(DriverError::Timeout)
    );
    assert!(!timer.is_running().unwrap());

    let mut spi = MockSpi::<1>::new(TIMER_LIMIT);
    let mut transfer = [0; 1];
    spi.acquire().unwrap();
    spi.select(SpiDeviceId::new(1)).unwrap();

    spi.timed_out = true;
    assert_eq!(
        spi.transfer(&mut transfer, VALID_TIMEOUT),
        Err(DriverError::Timeout)
    );
    spi.timed_out = false;
    spi.would_block = true;
    assert_eq!(
        spi.transfer(&mut transfer, VALID_TIMEOUT),
        Err(DriverError::WouldBlock)
    );

    spi.would_block = false;
    spi.deselect().unwrap();
    spi.release().unwrap();
    assert!(!spi.is_owned());
}

#[test]
fn i2c_write_read_preserves_repeated_start_transaction() {
    let mut i2c = MockI2c::<4>::new(TIMER_LIMIT);
    let address = I2cAddress::new(0x42);
    let mut response = [0; 2];
    i2c.set_response(&[7, 8]).unwrap();
    i2c.acquire().unwrap();

    i2c.write_read(address, &[1, 2], &mut response, VALID_TIMEOUT)
        .unwrap();

    assert_eq!(response, [7, 8]);
    assert_eq!(i2c.last_address, Some(address));
    assert_eq!(i2c.last_write[..2], [1, 2]);
    assert_eq!(i2c.last_write_len, 2);
    assert_eq!(i2c.last_operation, Some(I2cOperation::WriteRead));
    assert_eq!(i2c.write_read_calls, 1);
    i2c.release().unwrap();
}

#[test]
fn i2c_bounded_timeout_and_typed_bus_errors_propagate() {
    let mut i2c = MockI2c::<2>::new(TIMER_LIMIT);
    let address = I2cAddress::new(0x18);
    let mut buffer = [0; 1];
    i2c.acquire().unwrap();

    assert_eq!(
        i2c.read(address, &mut buffer, Duration::from_ticks(0)),
        Err(DriverError::Timeout)
    );
    i2c.timed_out = true;
    assert_eq!(
        i2c.read(address, &mut buffer, VALID_TIMEOUT),
        Err(DriverError::Timeout)
    );
    i2c.timed_out = false;
    i2c.bus_error = true;
    assert_eq!(
        i2c.write(address, &[1], VALID_TIMEOUT),
        Err(DriverError::BusError)
    );
    i2c.bus_error = false;
    i2c.nack = true;
    assert_eq!(
        i2c.write(address, &[1], VALID_TIMEOUT),
        Err(DriverError::Nack)
    );
    i2c.nack = false;
    i2c.arbitration_lost = true;
    assert_eq!(
        i2c.write(address, &[1], VALID_TIMEOUT),
        Err(DriverError::ArbitrationLost)
    );
    i2c.release().unwrap();
}

#[test]
fn i2c_ownership_lifecycle_is_exclusive_and_balanced() {
    let mut i2c = MockI2c::<2>::new(TIMER_LIMIT);

    assert!(!i2c.is_owned());
    assert_eq!(i2c.release(), Err(DriverError::InvalidState));
    i2c.acquire().unwrap();
    assert_eq!(i2c.acquire(), Err(DriverError::ResourceBusy));
    assert!(i2c.is_owned());
    i2c.release().unwrap();
    assert!(!i2c.is_owned());
}

#[test]
fn serial_ownership_and_configuration_are_exclusive_and_bounded() {
    let mut serial = MockSerial::<4>::new(TIMER_LIMIT);
    let baud_rate = BaudRate::from_bits_per_second(115_200).unwrap();
    let config = SerialConfig::new(baud_rate, DataBits::Eight, Parity::None, StopBits::One);

    assert!(!serial.is_owned());
    assert_eq!(serial.release(), Err(DriverError::InvalidState));
    assert_eq!(serial.configure(config), Err(DriverError::InvalidState));
    serial.acquire().unwrap();
    assert_eq!(serial.acquire(), Err(DriverError::ResourceBusy));
    serial.configure(config).unwrap();
    assert_eq!(serial.config, Some(config));
    serial.release().unwrap();
    assert!(!serial.is_owned());
}

#[test]
fn spi_bus_ownership_requires_balanced_device_selection() {
    let mut spi = MockSpi::<4>::new(TIMER_LIMIT);
    let device = SpiDeviceId::new(3);

    assert_eq!(spi.select(device), Err(DriverError::InvalidState));
    spi.acquire().unwrap();
    assert_eq!(spi.acquire(), Err(DriverError::ResourceBusy));
    spi.select(device).unwrap();
    assert_eq!(spi.selected(), Some(device));
    assert_eq!(spi.select(device), Err(DriverError::ResourceBusy));
    spi.deselect().unwrap();
    assert_eq!(spi.deselect(), Err(DriverError::InvalidState));
    spi.release().unwrap();
    assert!(!spi.is_owned());
}

#[test]
fn baud_rate_rejects_zero_without_platform_values() {
    assert_eq!(
        BaudRate::from_bits_per_second(0),
        Err(DriverError::InvalidState)
    );
}

#[test]
fn gpio_rejects_busy_ownership() {
    let mut gpio = MockGpio::new();
    gpio.busy = true;

    assert_eq!(gpio.set_low(), Err(DriverError::ResourceBusy));
    assert_eq!(
        gpio.set_mode(GpioMode::Input),
        Err(DriverError::ResourceBusy)
    );
}

#[test]
fn countdown_enforces_bounded_timeout_and_cancellation() {
    let mut timer = MockTimer::new(TIMER_LIMIT);

    assert_eq!(
        timer.validate_timeout(Duration::from_ticks(0)),
        Err(DriverError::Timeout)
    );
    assert_eq!(
        timer.validate_timeout(Duration::from_ticks(101)),
        Err(DriverError::Timeout)
    );
    CountDown::start(&mut timer, VALID_TIMEOUT).unwrap();
    assert!(timer.is_running().unwrap());
    assert!(!timer.wait().unwrap());
    timer.cancel().unwrap();
    assert_eq!(timer.wait(), Err(DriverError::InvalidState));
}

#[test]
fn timer_driver_reports_lifecycle_state() {
    let mut timer = MockTimer::new(TIMER_LIMIT);

    assert!(!timer.is_running().unwrap());
    assert_eq!(timer.is_expired(), Err(DriverError::InvalidState));
    TimerDriver::start(&mut timer, VALID_TIMEOUT).unwrap();
    timer.expired = true;
    assert!(timer.is_expired().unwrap());
    TimerDriver::start(&mut timer, VALID_TIMEOUT).unwrap();
    assert!(!timer.is_expired().unwrap());
    timer.expired = true;
    assert!(timer.is_expired().unwrap());
    assert!(timer.is_expired().unwrap());
    timer.stop().unwrap();
    assert!(!timer.is_running().unwrap());
}

#[test]
fn display_text_is_clipped_to_the_typed_text_grid() {
    let mut display = TestDisplay::new(TIMER_LIMIT);
    display.initialize(VALID_TIMEOUT).unwrap();

    display
        .write_text(
            TextPosition::new(DISPLAY_COLUMNS - 1, DISPLAY_ROWS - 1),
            b"AB",
            VALID_TIMEOUT,
        )
        .unwrap();

    assert_eq!(display.buffer[DISPLAY_BUFFER_CAPACITY - 1], b'A');
    assert_eq!(
        display.buffer[..DISPLAY_BUFFER_CAPACITY - 1],
        [0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(display.dimensions().width(), DISPLAY_WIDTH);
    assert_eq!(display.dimensions().height(), DISPLAY_HEIGHT);
    assert_eq!(display.text_properties().columns(), DISPLAY_COLUMNS);
    assert_eq!(display.text_properties().rows(), DISPLAY_ROWS);
}

#[test]
fn display_records_the_bounded_command_sequence() {
    let mut display = TestDisplay::new(TIMER_LIMIT);
    display.initialize(VALID_TIMEOUT).unwrap();
    display.clear(VALID_TIMEOUT).unwrap();
    display
        .write_text(TextPosition::new(0, 0), b"OK", VALID_TIMEOUT)
        .unwrap();
    display.flush(VALID_TIMEOUT).unwrap();

    assert_eq!(
        display.commands[..display.command_count],
        [
            Some(DisplayCommand::Initialize),
            Some(DisplayCommand::Clear),
            Some(DisplayCommand::WriteText),
            Some(DisplayCommand::Flush),
        ]
    );
}

#[test]
fn display_flush_propagates_bounded_timeout_and_bus_errors() {
    let mut display = TestDisplay::new(TIMER_LIMIT);
    display.initialize(VALID_TIMEOUT).unwrap();

    display.timed_out = true;
    assert_eq!(display.flush(VALID_TIMEOUT), Err(DriverError::Timeout));
    display.timed_out = false;
    display.bus_error = true;
    assert_eq!(display.flush(VALID_TIMEOUT), Err(DriverError::BusError));
}

#[test]
fn display_unavailable_state_can_be_recovered_with_reset_and_reinitialization() {
    let mut display = TestDisplay::new(TIMER_LIMIT);
    display.connected = false;
    assert_eq!(
        display.initialize(VALID_TIMEOUT),
        Err(DriverError::DisplayUnavailable)
    );

    display.connected = true;
    display.reset(VALID_TIMEOUT).unwrap();
    display.initialize(VALID_TIMEOUT).unwrap();
    assert_eq!(display.command_count, 2);
}

#[test]
fn diagnostics_console_clips_lines_and_tracks_cursor() {
    let mut console = DiagnosticsConsole::<DISPLAY_COLUMNS, DISPLAY_ROWS>::new();
    console.write(b"12345");

    assert_eq!(console.cursor(), TextPosition::new(DISPLAY_COLUMNS, 0));
    assert!(console.overflowed());
    assert_eq!(console.row(0), Some(b"1234"));
}

#[test]
fn diagnostics_console_scrolls_deterministically() {
    let mut console = DiagnosticsConsole::<DISPLAY_COLUMNS, DISPLAY_ROWS>::new();
    console.write(b"1234\n5678\n9");

    assert_eq!(console.row(0), Some(b"5678"));
    assert_eq!(console.row(1), Some(b"9   "));
    assert_eq!(console.cursor(), TextPosition::new(1, 1));
}

#[test]
fn diagnostics_console_renders_a_bounded_command_sequence() {
    let mut console = DiagnosticsConsole::<DISPLAY_COLUMNS, DISPLAY_ROWS>::new();
    let mut display = TestDisplay::new(TIMER_LIMIT);
    display.initialize(VALID_TIMEOUT).unwrap();
    console.write(b"boot");

    console.render(&mut display, VALID_TIMEOUT).unwrap();

    assert_eq!(
        &display.commands[..display.command_count],
        &[
            Some(DisplayCommand::Initialize),
            Some(DisplayCommand::Clear),
            Some(DisplayCommand::WriteText),
            Some(DisplayCommand::WriteText),
            Some(DisplayCommand::Flush),
        ]
    );
}

#[test]
fn diagnostics_console_enters_headless_mode_on_display_loss() {
    let mut console = DiagnosticsConsole::<DISPLAY_COLUMNS, DISPLAY_ROWS>::new();
    let mut display = TestDisplay::new(TIMER_LIMIT);
    display.initialize(VALID_TIMEOUT).unwrap();
    display.connected = false;

    console.write(b"ignored when unavailable");
    console.render(&mut display, VALID_TIMEOUT).unwrap();

    assert!(console.is_headless());
    console.write(b"still bounded");
    console.render(&mut display, VALID_TIMEOUT).unwrap();
}
