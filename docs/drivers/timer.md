# Timers

Timer contracts use an adapter-defined `Duration` and a declared maximum
timeout. `BoundedTimeout` rejects zero or oversized durations before hardware
configuration. `TimerDriver` covers start, stop, running state, and expiration
observation; `CountDown` covers one-shot start, bounded wait, and cancellation.

No timer contract permits an unbounded spin. A backend must return a typed
timeout, state, or hardware error when the requested operation cannot finish
within its declared bound.

## SysTick and peripheral policy

The F405 board initially owns SysTick for bootstrap and heartbeat delays. After
application activation, the board-local timer adapter may transition that same
owned resource to scheduler tick mode. The generic API does not contain the
F405 clock, reload range, or peripheral register map; those values come from
the target profile and platform adapter.

Timer interrupt delivery is separate from monotonic or countdown observation.
An adapter must not expose HAL timer types through `dali-driver-api`, and a
caller must not assume that a successful host mock represents physical timer
behavior.
