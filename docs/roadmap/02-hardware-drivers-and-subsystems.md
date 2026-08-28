# Hardware Drivers and System Subsystems

Status: **Active — repository quality work; I2C device validation pending**.

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
[`docs/testing/README.md`](../testing/README.md). Driver claims must remain specific to the
observed backend and board configuration.

## Current execution order

### Phase 1 — F405 bounded-timeout acceptance — Completed

- [x] Capture Timer timeout integration and expiration evidence on the F405.
- [x] Capture external-SPI or intentionally stalled-SPI recovery evidence on the
  F405.
- [x] Record the available firmware, transport, expected trace, observed trace,
  result, and unrecorded board metadata in `docs/testing/README.md`.

### Phase 2 — Communication drivers — Completed 2026-08-25

- [x] Define a hardware-neutral bounded I2C Write, Read, and Write-Read
  (repeated-start) contract.
- [x] Implement the F405 I2C backend with bounded bus-lockup recovery.
- [x] Add fixed-capacity I2C mocks and host tests for ownership, repeated-start,
  timeout, and bus errors.
- [x] Confirm the F405 I2C timeout and bus-recovery path with the 2026-08-25
  Silicon Trace recorded in `docs/testing/README.md`.

### Phase 3 — Small-form-factor I2C display and diagnostics console

- [x] Specify an SSD1306 or SH1106 128x64 I2C display backend.
- [x] Define a hardware-neutral OLED contract and bounded framebuffer or
  command-stream policy in `crates/dali-driver-api/`.
- [x] Extend target manifests with display controller, address, dimensions,
  reset policy, and update-time limits; generate typed display metadata.
- [x] Implement the F405 I2C OLED adapter only behind the display capability.
- [x] Define and implement a bounded text-mode diagnostics console for boot
  status and diagnostic stream records.
- [x] Record target rendering, unavailable-display, and recovery evidence.

The Phase 3 implementation is complete, but physical OLED acceptance is still
pending. The available F405 trace confirms boot progress and bounded I2C
failure/recovery handling; it does not confirm a connected device ACK or
physical OLED rendering. The SPI driver, ILI9341 experiments, USB CDC core
servicing, SDIO, and Storage remain frozen.

#### Phase 3 action plan

1. **Freeze the display contract.** Select SSD1306 or SH1106 after confirming
   the module's I2C command and addressing mode. Define typed dimensions,
   bounded initialization, bounded page/frame updates, reset behavior, and
   `Unavailable`, `Timeout`, `BusError`, and `InvalidState` errors. The
   contract must accept caller-owned text or framebuffer data and must not
   expose F405 PAC/HAL types.
2. **Add hardware-neutral API and host coverage.** Place the display trait and
   bounded text/flush policy in `crates/dali-driver-api/`. Add fixed-capacity
   host mocks only for contract tests covering dimensions, clipping, command
   ordering, update bounds, timeout propagation, and unavailable-display
   recovery. Host tests are not hardware evidence.
3. **Extend target metadata.** Add a manifest-owned display capability with
   controller, I2C address, width, height, addressing mode, reset policy, and
   update timeout/buffer limits. Generate a typed display profile through
   `crates/dali-targets/`; reject incomplete or unsupported combinations at
   manifest validation time. No display value may be duplicated in kernel
   policy code.
4. **Implement the F405 backend.** Add the SSD1306/SH1106 adapter behind the
   existing hardware-neutral I2C boundary. Keep all PAC/HAL ownership in
   `kernel/src/platform/f405/` and use the already accepted bounded I2C
   timeout and recovery path. A missing or failed OLED must return a bounded
   error and must not stall boot or storage recovery.
5. **Build the diagnostics console engine.** Define a bounded text-mode sink
   with fixed rows, columns, line clipping, and deterministic cursor/update
   behavior. Feed it boot status and diagnostic records through a small
   hardware-neutral interface; do not couple it to USB CDC servicing or alter
   the existing logging transport. Define overflow and unavailable-display
   behavior explicitly.
6. **Integrate and accept on hardware.** Add the display capability to the
   F405 acceptance image, verify initialization, text rendering, bounded
   update behavior, and unplugged/bus-fault recovery with Silicon Trace and
   the physical OLED. Record board, display controller, I2C wiring, power,
   firmware revision, expected output, observed output, and result in
   `docs/testing/README.md` before closing Phase 3.

The implementation order was contract and host tests, manifest/profile
generation, F405 backend, console engine, then physical acceptance. The code
steps are complete; physical I2C device validation awaits laboratory equipment
and remains open. SPI display code and ILI9341 experiments remain out of scope.

### Current work — documentation and codebase organization

While I2C laboratory validation is pending, the active track is repository
quality work:

- [ ] Reconcile architecture, roadmap, testing, and file-structure documents
  with the current implementation and evidence boundary.
- [ ] Inventory large files, mixed responsibilities, misplaced modules, and
  obsolete or duplicated code.
- [ ] Split large implementation files only at explicit ownership boundaries;
  preserve behavior, public APIs, generated output, and boot order.
- [ ] Keep refactoring separate from behavioral changes and frozen subsystem
  paths.
- [ ] Validate every atomic refactoring with formatting, focused tests,
  workspace checks, and `git diff --check`.
- [ ] Close I2C device acceptance only after laboratory measurement confirms
  a real F405 device ACK and OLED rendering.

### Phase 4 — Power and diagnostics

- [ ] Complete accurate RCC reset-reason mapping.
- [ ] Implement Ustari-based wired read-only `sysinfo`, `mem`, `ps`, and
  `dmesg` diagnostics with target evidence.

## Frozen boundaries

- SDIO and Storage are frozen at the Known-Good baseline. Do not modify
  `kernel/src/platform/f405/sdio_raw/`, `kernel/src/storage/`, or SDIO manifest
  limits while this roadmap is active.
- USB CDC core servicing and polling loops are frozen.
- ILI9341 and SPI display experiments are frozen.

## Slice 1 — GPIO and timers contract

Status: **Future**.

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
- [x] Record F405 GPIO input and interrupt evidence: grounding the PC13 input
  produced repeated `[DRIVER][EXTI] PC13 interrupt triggered` traces both with
  the SD card absent and during a successful SDIO/package boot.

### Timers

- [x] Define `TimerDriver` and `CountDown` contracts.
- [x] Define typed duration, maximum timeout, cancellation, and expiration rules.
- [x] Add host tests for bounds, restart, cancellation, and repeated expiry.
- [x] Implement a target timer adapter from manifest-owned timing metadata.
- [x] Record F405 timer elapsed-tick evidence with the F405 acceptance firmware.
- [ ] Record F405 bounded-timeout evidence.

### Slice 3 — EXTI and serial driver contracts

- [x] Extend `InterruptPin` with edge selection, an allocation-free callback,
  and a polling handle contract.
- [x] Add bounded `SerialRead` and `SerialWrite` contracts for caller-owned
  byte slices.
- [x] Add bounded `SpiTransfer` contract for caller-owned full-duplex buffers.
- [x] Add typed non-blocking, framing, overrun, buffer, NACK, and arbitration
  errors without exposing target-specific types.
- [x] Add fixed-size host mocks for GPIO callbacks, serial partial transfers,
  and SPI failures.
- [x] Add host tests for callback delivery, partial I/O, timeout bounds,
  disconnects, buffer limits, and protocol failures.
- [x] Implement the F405 EXTI adapter inside the platform driver boundary.
- [x] Bind an F405 EXTI input resource to board-owned EXTI and SYSCFG state.
- [x] Record F405 EXTI edge and pending-bit evidence: grounding the PC13 input
  produced repeated `[DRIVER][EXTI] PC13 interrupt triggered` traces both with
  the SD card absent and during a successful SDIO/package boot.
- [x] Run Slice 3 host tests, strict Clippy, and the production F405 build.

### UART

- [x] Define bounded UART read/write and framing-error contracts.
- [x] Define hardware-neutral UART ownership, baud configuration, and
  backpressure error contracts.
- [x] Add host tests for exclusive ownership and invalid zero baud rates.
- [x] Add host tests for partial transfers, overflow, timeout, and disconnect
  across UART read, write, and flush operations.
- [x] Implement the F405 UART adapter behind the accepted driver contract.
- [x] Record F405 target UART timeout evidence with the acceptance firmware.

### SPI / I2C

- [x] Define bounded transaction, bus ownership, and device-selection contracts.
- [x] Define timeout, arbitration, NACK, and protocol-error variants.
- [x] Add host tests for transaction bounds and typed failure propagation.
- [x] Add host tests for balanced bus ownership and chip-selection lifecycle.
- [x] Implement the F405 SPI adapter with board-owned pin and clock configuration.
- [ ] Record physical bus and recovery evidence for each supported backend.

### F405 bounded-I/O acceptance

- [x] Run the real F405 UART no-data probe and observe
  `[DRIVER][UART] Bounded timeout enforced`.
- [x] Run the real F405 SPI transfer probe and observe
  `[DRIVER][SPI] Bounded transfer completed`.
- [ ] Exercise an external SPI device or an intentionally stalled SPI state;
  a no-peer master transfer does not by itself prove the SPI timeout branch.

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
