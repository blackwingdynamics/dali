# Driver Architecture

Dali driver contracts are `no_std`, allocation-free interfaces between kernel
policy and board-owned hardware adapters. Generic code depends on traits and
typed values from `dali-driver-api`; PAC and HAL types remain inside the
selected platform backend.

## Core rules

- Driver operations use caller-owned slices or fixed-size buffers. They do not
  allocate from a heap and do not return owned dynamic collections.
- Every operation has a bounded completion rule. A driver returns
  `DriverError::WouldBlock` when progress is not currently possible and
  `DriverError::Timeout` when the caller's declared duration is invalid or
  exhausted.
- Resource ownership is explicit. A driver must report `ResourceBusy`,
  `InvalidState`, or `Disconnected` instead of silently sharing hardware.
- Protocol and peripheral failures use typed errors such as `Framing`,
  `Overrun`, `Nack`, and `ArbitrationLost`.
- Callbacks are function pointers with no captured allocation. Polling handles
  must remain bounded and must acknowledge an event exactly once.

## Boundary ownership

`dali-driver-api` contains only hardware-neutral traits, durations, and error
categories. It must not import a PAC, HAL, register address, pin type, clock
value, or board identifier. The F405 backend adapts those concrete resources
to the contracts inside `kernel/src/platform/f405/`.

Host mocks exercise contract behavior with fixed-size storage only. They are
not substitutes for target validation. Hardware acceptance requires the
selected board, firmware image, physical peripheral, and target-visible trace.

## Validation policy

Each driver slice requires contract tests, strict Clippy, and the embedded
build before review. A backend is not accepted until its required hardware
trace is recorded separately from host-test and compilation evidence.
