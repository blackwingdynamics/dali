# GPIO and EXTI

The GPIO contract separates level access, output control, pin-mode changes,
and interrupt observation. `InputPin`, `OutputPin`, and `PinMode` expose only
bounded operations and typed `DriverError` results.

## Pin modes

Adapters own the concrete transition from input, output, or alternate mode.
Generic code requests a contract-defined `GpioMode`; it does not name a PAC
register, port, pin number, or alternate-function value. Invalid transitions
and unavailable capabilities return `InvalidState` or `Unsupported`.

## EXTI model

`InterruptPin::enable_interrupt` accepts an `InterruptTrigger` and an optional
allocation-free `InterruptCallback` function pointer. Supported edge triggers
are rising, falling, and both edges. Level triggers may be rejected by a
backend when the peripheral provides edge-only EXTI lines.

`take_pending` is the polling handle. It reports and clears one pending event;
when a callback is registered, the adapter invokes it at that same bounded
acknowledgment boundary. `disable_interrupt` releases the line and callback.
Applications do not claim kernel-owned vectors or pins.

## F405 boundary

The F405 adapter keeps `stm32f4xx-hal` GPIO, EXTI, SYSCFG, and PAC values inside
the platform module. The board binds the documented PC13 user-key input to an
F405 EXTI line, while the kernel heartbeat performs bounded polling. Physical
edge and pending-bit behavior still requires F405 hardware evidence.
