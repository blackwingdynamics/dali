mod mock;

use core::sync::atomic::{AtomicU8, Ordering};

use dali_driver_api::{
    BoundedTimeout, CountDown, DriverError, Duration, GpioMode, InputPin, InterruptPin,
    InterruptTrigger, OutputPin, PinMode, SerialRead, SerialWrite, SpiTransfer, TimerDriver,
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
