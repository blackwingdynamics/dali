//! Board-local LED and delay services.

use super::resources::{Board, TimerMode};
use dali_driver_api::OutputPin;
use stm32f4xx_hal::prelude::*;

/// Sets the active-high F405 board LED to the requested logical state.
pub fn set_status_led(board: &mut Board, on: bool) {
    let result = if on {
        OutputPin::set_high(&mut board.status_led)
    } else {
        OutputPin::set_low(&mut board.status_led)
    };
    if result.is_err() {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[GPIO] Status LED operation failed"),
        );
    }
    #[cfg(feature = "driver-hardware-test")]
    if result.is_ok() && board.status_led_on != on {
        board.status_led_on = on;
        if !board.gpio_driver_log_emitted {
            board.gpio_driver_log_emitted = true;
            crate::logging::info(
                crate::logging::BOOT_SUBSYSTEM,
                format_args!("[DRIVER][GPIO] Hardware pin toggled"),
            );
        }
    }
}

/// Delays through the bootstrap-owned SysTick mode.
pub fn delay_ms(board: &mut Board, milliseconds: u32) {
    let Some(mode) = board.delay.take() else {
        return;
    };
    let delay = match mode {
        TimerMode::Delay(mut delay) => {
            delay.delay_ms(milliseconds);
            delay
        }
        #[cfg(feature = "abi-context-switch")]
        TimerMode::Scheduler(counter) => {
            board.delay = Some(TimerMode::Scheduler(counter));
            return;
        }
    };
    board.delay = Some(TimerMode::Delay(delay));
}
