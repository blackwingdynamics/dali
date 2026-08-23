# Hardware Drivers and System Subsystems

Status: **Active**.

The current implementation target is the WeAct STM32F405RGT6 board. Hardware
specific facts remain in the F405 platform boundary and target manifest;
generic contracts must remain hardware-agnostic.

## Existing driver baseline

- Board-owned clock, GPIO, SDIO, USB CDC, watchdog, and interrupt ownership.
- Read-only FAT16/FAT32 access and bounded repository streaming.
- Kernel-owned SDIO transfer buffers in DMA-accessible SRAM.
- Bounded storage lifecycle state machine:
  `Unavailable -> Present -> Ready -> Removed/Fault`.
- Safe recovery heartbeat with bounded SDIO reinitialization after reinsertion.
- Kernel-owned watchdog servicing around opaque storage and verification work.

F405 hardware records for these paths are maintained in
[`docs/TESTING.md`](../TESTING.md). Driver claims must remain specific to the
observed backend and board configuration.

## Slice 1 — GPIO and timers contract

Status: **Planned**.

This slice defines hardware-neutral driver contracts only. It does not select
an MCU register map, move board facts out of the F405 platform boundary, or
replace an existing HAL backend.

### 1. `dali-driver-api` crate foundation

- Create a `no_std` crate containing only bounded, allocation-free driver
  contracts and error types.
- Keep trait methods synchronous and bounded unless a contract explicitly
  documents interrupt-driven behavior.
- Represent ownership, capabilities, and timeout units with typed values;
  avoid raw pins, addresses, frequencies, or platform identifiers.
- Define a common `DriverError` taxonomy that preserves unsupported,
  invalid-state, timeout, disconnected, and hardware-fault causes.
- Keep all host mocks behind test modules or a test-support boundary; mocks
  must exercise contracts and must not simulate hardware acceptance.

### 2. GPIO contract

Define the smallest reusable traits needed by board-owned GPIO adapters:

- `InputPin`: bounded level read and optional edge observation;
- `OutputPin`: bounded level set/toggle operation;
- `PinMode`: typed input/output/alternate-mode transition;
- `InterruptPin`: explicit enable, disable, and pending-event handling.

The contract must define invalid transitions, unavailable interrupt support,
level semantics, and ownership errors. Interrupt APIs must not expose a raw
vector or permit an application to claim kernel-owned pins.

### 3. Timer contract

- Define `TimerDriver` for start, stop, and bounded expiration observation.
- Define `CountDown` for one-shot deadline progression using a typed duration.
- Require explicit maximum timeout and reject zero/overflowing durations.
- Separate monotonic time measurement from timer interrupt delivery.
- Define restart, expiration, cancellation, and already-running behavior.
- Keep frequency and timer selection in target metadata or the platform
  adapter; generic code must not contain board timing values.

### 4. Error and host-test slice

- Add typed GPIO and timer error enums with stable, documented variants.
- Add host tests for valid transitions, invalid ownership, unsupported modes,
  timeout bounds, cancellation, repeated expiration, and error propagation.
- Add deterministic contract-test doubles that record calls and return
  configured typed outcomes without pretending to be target evidence.
- Add documentation examples that compile without heap allocation.

### 5. Slice 1 acceptance gate

- `cargo test --workspace --exclude dali-kernel` passes with the new contract
  tests.
- Host strict Clippy and formatting pass.
- The F405 kernel builds with the existing production feature set unchanged.
- No generic API contains F405 pins, MMIO addresses, timer frequencies, or
  deployment-specific values.
- A later F405 adapter slice must provide the physical GPIO/timer evidence;
  Slice 1 host tests alone cannot close hardware acceptance.

## Interactive subsystem checklist

Update this checklist after each implementation, test, and commit. Keep items
atomic and mark an item `[x]` only when its stated evidence exists.

### Slice 1 core API

- [x] Add `dali-driver-api` as a workspace crate with a grouped module layout.
- [x] Keep the production crate `no_std`, dependency-free, and zero-heap.
- [x] Define the bounded `DriverError` and `DriverResult` contract.
- [x] Add deterministic GPIO and timer contract-test doubles under `tests/`.
- [x] Run the `dali-driver-api` host test suite and strict Clippy.

### GPIO

- [x] Define `InputPin`, `OutputPin`, and `PinMode` contracts.
- [x] Define typed GPIO ownership and transition errors.
- [x] Define interrupt enable, disable, pending, and acknowledgment contract.
- [x] Add host tests for levels, modes, ownership, and unsupported interrupts.
- [x] Implement a target adapter without leaking F405 facts into the API crate.
- [x] Record F405 GPIO output-toggle evidence with the F405 acceptance firmware.
- [ ] Record F405 GPIO input and interrupt evidence.

### Timers

- [x] Define `TimerDriver` and `CountDown` contracts.
- [x] Define typed duration, maximum timeout, cancellation, and expiration rules.
- [x] Add host tests for bounds, restart, cancellation, and repeated expiry.
- [x] Implement a target timer adapter from manifest-owned timing metadata.
- [x] Record F405 timer elapsed-tick evidence with the F405 acceptance firmware.
- [ ] Record F405 bounded-timeout evidence.

### UART

- [ ] Define bounded UART read/write and framing-error contracts.
- [ ] Define ownership, baud configuration, and backpressure errors.
- [ ] Add host tests for partial transfers, overflow, timeout, and disconnect.
- [ ] Implement a target adapter only after the contract is accepted.
- [ ] Record target console/UART evidence where applicable.

### SPI / I2C

- [ ] Define bounded transaction, bus ownership, and device-selection contracts.
- [ ] Define timeout, arbitration, NACK, and protocol-error variants.
- [ ] Add host tests for transaction bounds and typed failure propagation.
- [ ] Implement target adapters with board-owned pin and clock configuration.
- [ ] Record physical bus and recovery evidence for each supported backend.

### Display

- [ ] Define a bounded framebuffer or command-stream contract.
- [ ] Define initialization, flush, reset, and unavailable-display errors.
- [ ] Add host tests for dimensions, clipping, and bounded update behavior.
- [ ] Implement a target adapter only after a display capability is selected.
- [ ] Record target rendering and failure-recovery evidence.

### CAN

- [ ] Define bounded frame identifiers, payload limits, and filter ownership.
- [ ] Define bus-off, arbitration-loss, timeout, and controller-fault errors.
- [ ] Add host tests for frame validation, filters, and recovery transitions.
- [ ] Implement a target adapter behind explicit capability metadata.
- [ ] Record physical CAN acceptance evidence before marking the slice complete.

### Power / Services

- [ ] Define power-state, reset-reason, and service-health contracts.
- [ ] Define watchdog, brownout, and unavailable-service error semantics.
- [ ] Add host tests for bounded state transitions and ownership rules.
- [ ] Integrate only kernel-owned safety services with driver capabilities.
- [ ] Record target power/reset evidence for each supported board.

## Continuing backlog

- Add target-independent driver capability contracts without moving board
  facts into generic storage, loader, or policy modules.
- Verify removal and reinsertion while an application is already running;
  current recovery does not automatically relaunch an application.
- Add bounded fault telemetry for SDIO, USB, and watchdog transitions.
- Review every future DMA-capable backend under the same ownership policy;
  arbitrary DMA-controller isolation is not complete.
- Add cross-target watchdog semantics only when another watchdog backend is
  introduced.
- Define power-loss-safe storage update acceptance before writable production
  installation is enabled.

## Acceptance gate

Every driver slice requires host tests for hardware-neutral policy, the
appropriate target build and Clippy checks, and F405 evidence when behavior
depends on physical peripherals. No simulated device or fake storage is an
acceptance substitute.
