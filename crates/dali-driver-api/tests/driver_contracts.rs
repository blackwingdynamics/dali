mod mock;

use dali_driver_api::{
    BoundedTimeout, CountDown, DriverError, Duration, GpioMode, InputPin, InterruptPin,
    InterruptTrigger, OutputPin, PinMode, TimerDriver,
};
use mock::{MockGpio, MockTimer};

const TIMER_LIMIT: Duration = Duration::from_ticks(100);
const VALID_TIMEOUT: Duration = Duration::from_ticks(10);

#[test]
fn gpio_contracts_cover_mode_level_and_interrupt_lifecycle() {
    let mut gpio = MockGpio::new();

    gpio.set_mode(GpioMode::Output).unwrap();
    gpio.set_high().unwrap();
    assert!(gpio.is_high().unwrap());
    gpio.toggle().unwrap();
    assert!(!gpio.is_high().unwrap());

    gpio.enable_interrupt(InterruptTrigger::RisingEdge).unwrap();
    gpio.pending = true;
    assert!(gpio.take_pending().unwrap());
    assert!(!gpio.take_pending().unwrap());
    gpio.disable_interrupt().unwrap();

    gpio.interrupt_supported = false;
    assert_eq!(
        gpio.enable_interrupt(InterruptTrigger::BothEdges),
        Err(DriverError::Unsupported)
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
