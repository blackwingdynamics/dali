mod mock;

use core::sync::atomic::{AtomicU8, Ordering};

use dali_driver_api::{
    BaudRate, BoundedTimeout, CountDown, DataBits, DriverError, Duration, GpioMode, InputPin,
    InterruptPin, InterruptTrigger, OutputPin, Parity, PinMode, SerialConfig, SerialConfigure,
    SerialOwnership, SerialRead, SerialWrite, SpiBusOwnership, SpiDeviceId, SpiDeviceSelect,
    SpiTransfer, StopBits, TimerDriver,
};
use mock::{MockGpio, MockSerial, MockSpi, MockTimer};

const TIMER_LIMIT: Duration = Duration::from_ticks(100);
const VALID_TIMEOUT: Duration = Duration::from_ticks(10);

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
