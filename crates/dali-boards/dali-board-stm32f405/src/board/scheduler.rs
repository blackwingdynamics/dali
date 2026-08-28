//! Board-local SysTick scheduler transition.

use super::super::drivers::F405TimerDriver;
use super::{Board, SYSTEM_CLOCK_HZ, TimerMode};
use dali_driver_api::{Duration, TimerDriver};
#[cfg(feature = "driver-hardware-test")]
use dali_targets::TARGET_F405;

/// Lowest reload value accepted by the SysTick peripheral.
const SYSTICK_MIN_RELOAD: u32 = 1;
/// Highest reload value representable by the SysTick peripheral.
const SYSTICK_MAX_RELOAD: u32 = 0x00FF_FFFF;

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
    if !run_timeout_probe(
        &mut counter,
        tick_hz,
        TARGET_F405.driver_probe.timer_timeout_divisor,
        TARGET_F405.driver_probe.timer_evidence_poll_limit,
    ) {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][TIMER] Hardware timer timeout probe failed"),
        );
        return false;
    }
    #[cfg(feature = "driver-hardware-test")]
    if !counter.wait_for_tick(TARGET_F405.driver_probe.timer_evidence_poll_limit) {
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
fn run_timeout_probe(
    counter: &mut F405TimerDriver,
    tick_hz: u32,
    timeout_divisor: u32,
    evidence_poll_limit: u32,
) -> bool {
    let timeout_ticks = tick_hz.checked_div(timeout_divisor);
    let Some(timeout_ticks) = timeout_ticks.filter(|ticks| *ticks != 0) else {
        return false;
    };
    if counter.start(Duration::from_ticks(timeout_ticks)).is_err() {
        return false;
    }
    let expired = counter.wait_for_timeout(evidence_poll_limit);
    let stopped = counter.stop().is_ok();
    if expired && stopped {
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][TIMER] Hardware timer timeout enforced"),
        );
    }
    stopped && expired && counter.start_frequency(tick_hz).is_ok()
}
