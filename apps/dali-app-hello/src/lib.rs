#![no_std]

//! Shared constants for the Dali OS AMRN demo application.

/// Number of application LED flashes in one execution-pattern cycle.
pub const LED_FLASH_COUNT: usize = 3;

/// Number of busy-loop iterations used for each short LED phase.
pub const LED_SHORT_PHASE_ITERATIONS: u32 = 250_000;

/// Multiplier used to make the inter-cycle pause longer than a short phase.
pub const LED_LONG_PAUSE_MULTIPLIER: u32 = 4;

/// Number of phases in one complete LED pattern cycle.
pub const LED_PATTERN_PHASE_COUNT: usize = LED_FLASH_COUNT * 2 + 1;

/// One logical phase of the application execution indicator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedPatternStep {
    /// Whether the logical LED state is on during this phase.
    pub on: bool,
    /// Number of bounded busy-loop iterations for this phase.
    pub duration_iterations: u32,
}

/// Three short flashes followed by a longer pause.
pub const LED_PATTERN: [LedPatternStep; LED_PATTERN_PHASE_COUNT] = [
    LedPatternStep {
        on: true,
        duration_iterations: LED_SHORT_PHASE_ITERATIONS,
    },
    LedPatternStep {
        on: false,
        duration_iterations: LED_SHORT_PHASE_ITERATIONS,
    },
    LedPatternStep {
        on: true,
        duration_iterations: LED_SHORT_PHASE_ITERATIONS,
    },
    LedPatternStep {
        on: false,
        duration_iterations: LED_SHORT_PHASE_ITERATIONS,
    },
    LedPatternStep {
        on: true,
        duration_iterations: LED_SHORT_PHASE_ITERATIONS,
    },
    LedPatternStep {
        on: false,
        duration_iterations: LED_SHORT_PHASE_ITERATIONS,
    },
    LedPatternStep {
        on: false,
        duration_iterations: LED_SHORT_PHASE_ITERATIONS * LED_LONG_PAUSE_MULTIPLIER,
    },
];

#[cfg(test)]
mod tests {
    use super::{
        LED_FLASH_COUNT, LED_PATTERN, LED_PATTERN_PHASE_COUNT, LED_SHORT_PHASE_ITERATIONS,
    };

    #[test]
    fn pattern_contains_three_flashes_and_a_long_pause() {
        assert_eq!(LED_PATTERN.len(), LED_PATTERN_PHASE_COUNT);
        assert_eq!(
            LED_PATTERN.iter().filter(|step| step.on).count(),
            LED_FLASH_COUNT
        );
        assert!(
            LED_PATTERN[..LED_PATTERN.len() - 1]
                .iter()
                .all(|step| step.duration_iterations == LED_SHORT_PHASE_ITERATIONS)
        );
        assert!(!LED_PATTERN[LED_PATTERN.len() - 1].on);
        assert!(
            LED_PATTERN[LED_PATTERN.len() - 1].duration_iterations > LED_SHORT_PHASE_ITERATIONS
        );
    }
}
