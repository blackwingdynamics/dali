//! Board-local SysTick scheduler transition.

use super::super::drivers::F405TimerDriver;
use super::{Board, SYSTEM_CLOCK_HZ, TimerMode};
use dali_driver_api::{Duration, TimerDriver};

const SYSTICK_MIN_RELOAD: u32 = 1;
const SYSTICK_MAX_RELOAD: u32 = 0x00FF_FFFF;
#[cfg(feature = "driver-hardware-test")]
const DRIVER_EVIDENCE_POLL_LIMIT: u32 = SYSTICK_MAX_RELOAD;
#[cfg(feature = "driver-hardware-test")]
const DRIVER_TIMEOUT_PROBE_DIVISOR: u32 = 10;

/// Enables SysTick for the scheduler after the application context is ready.
pub(crate) fn enable_scheduler_tick(board: &mut Board, tick_hz: u32) -> bool {
    let Some(core_ticks) = SYSTEM_CLOCK_HZ.checked_div(tick_hz) else {
        return false;
    };
    let Some(reload) = core_ticks.checked_sub(1) else {
        return false;
    };
    if !(SYSTICK_MIN_RELOAD..=SYSTICK_MAX_RELOAD).contains(&reload) {
        return false;
    }

    let Some(TimerMode::Delay(delay)) = board.delay.take() else {
        return false;
    };
    let maximum_timeout = Duration::from_ticks(tick_hz);
    let mut counter = F405TimerDriver::new(delay.release().counter_hz(), tick_hz, maximum_timeout);
    if counter.start_frequency(tick_hz).is_err() {
        return false;
    }
    #[cfg(feature = "driver-hardware-test")]
    if !run_timeout_probe(&mut counter, tick_hz) {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][TIMER] Hardware timer timeout probe failed"),
        );
        return false;
    }
    #[cfg(feature = "driver-hardware-test")]
    if !counter.wait_for_tick(DRIVER_EVIDENCE_POLL_LIMIT) {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][TIMER] Hardware timer tick probe failed"),
        );
        return false;
    }
    #[cfg(feature = "driver-hardware-test")]
    crate::logging::info(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[DRIVER][TIMER] Hardware timer tick elapsed"),
    );
    counter.listen_update();
    board.delay = Some(TimerMode::Scheduler(counter));
    true
}

#[cfg(feature = "driver-hardware-test")]
fn run_timeout_probe(counter: &mut F405TimerDriver, tick_hz: u32) -> bool {
    let timeout_ticks = tick_hz.checked_div(DRIVER_TIMEOUT_PROBE_DIVISOR);
    let Some(timeout_ticks) = timeout_ticks.filter(|ticks| *ticks != 0) else {
        return false;
    };
    if counter.start(Duration::from_ticks(timeout_ticks)).is_err() {
        return false;
    }
    let expired = counter.wait_for_timeout(DRIVER_EVIDENCE_POLL_LIMIT);
    let stopped = counter.stop().is_ok();
    if expired && stopped {
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][TIMER] Hardware timer timeout enforced"),
        );
    }
    stopped && expired && counter.start_frequency(tick_hz).is_ok()
}
