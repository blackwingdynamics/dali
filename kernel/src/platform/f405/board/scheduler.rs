//! Board-local SysTick scheduler transition.

use super::super::drivers::F405TimerDriver;
use super::{Board, SYSTEM_CLOCK_HZ, TimerMode};
use dali_driver_api::Duration;

const SYSTICK_MIN_RELOAD: u32 = 1;
const SYSTICK_MAX_RELOAD: u32 = 0x00FF_FFFF;
#[cfg(feature = "driver-hardware-test")]
const DRIVER_EVIDENCE_POLL_LIMIT: u32 = SYSTICK_MAX_RELOAD;

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
