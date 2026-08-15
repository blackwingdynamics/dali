# Dali OS Roadmap

Tasks are intentionally small. A task is complete only when its stated evidence exists. Later tasks must not silently expand the MVP.

## Current Status — 2026-08-15

### Completed and evidenced

- The repository is a Rust workspace with kernel, SDK, CLI, and demo-application boundaries.
- The STM32F411 BlackPill backend exists for the original MVP target.
- The WeAct STM32F405RGT6 Core Board backend exists with its 8 MHz HSE, 168 MHz system clock, PB2 LED, and SDIO pin mapping.
- The F405 kernel builds, checks, and passes strict target Clippy.
- DFU flashing of the F405 firmware completes successfully.
- The F405 USB CDC device has enumerated as `1209:da11` and created `/dev/ttyACM0` during hardware testing.
- Read-only FAT filesystem integration, root-directory enumeration, AMRN extension filtering, and package discovery logging are implemented.
- Cross-platform setup scripts, Just recipes, the USB console helper, and development documentation exist.
- A fixed-capacity USB log queue exists in kernel RAM and records overflow instead of silently hiding it.
- The hardware-neutral `dali-usb` crate provides bounded delivery, link state, partial-write handling, and host tests without owning USB hardware.
- The production USB backend services the `usb-device` state machine from the F405/F411 `OTG_FS` interrupt while main-context logging only appends to the bounded queue.
- Renode simulation explicitly disables USB CDC because its STM32F4 reference model does not implement the OTG_FS global registers used by the production backend.
- Pico 2 SWD evidence shows the F405 firmware reaches blocking SDIO block-read code while the OTG_FS interrupt services USB polling; RTT boot logs are visible through the probe.
- The host successfully enumerated the runtime CDC device as `1209:da11` and created `/dev/ttyACM1` during SWD debugging.
- SWD showed `Configured` USB state and successful drain reports, but no `SerialPort::write()` call after boot logs were queued; the queue therefore needs an explicit software-pended OTG_FS service trigger.
- Hardware console testing showed `picocom` opening after enumeration while boot logs had already drained; CDC delivery must therefore wait for the host-open DTR signal.
- On 2026-08-15, Pico 2 SWD programming and `/dev/ttyACM1` CDC testing produced `Terminal ready` and delivered boot logs through `[STORAGE] SDIO card initialized`.
- SWD SDIO register evidence at the block-0 read showed `RXDAVL=1`, `RXACT=1`, `RXFIFOHF=0`, `RXOVERR=1`, 371 bytes remaining, and 94 FIFO words; the HAL read loop waits on half-full FIFO state and does not drain this tail.
- A follow-up SWD stop on the raw path showed `RXOVERR=1` with `TXUNDERR=0`; the reader now drains each 512-byte block in one bounded critical section using the SDIO eight-word threshold and its final FIFO tail.
- Conditional SWD capture confirmed the overrun at `STA=0x0022a060` while `FIFOCNT=0x60`; the raw reader no longer waits for CMD17 completion before starting the bounded data drain.
- A hardware test with `CLKCR.HWFC_EN` enabled changed the failure to `DataCorruption`; the STM32F4 HAL and F40x errata document that hardware flow control causes clock glitches and CRC errors, so it was removed again.
- The polling reader was replaced with an aligned DMA2 Stream 3, Channel 4 receive path; hardware confirmed block reads and root scanning on the STM32F405.
- SWD showed SDIO DMA enabled with active receive data while DMA2 Stream 3 registers were zero; DMA2 reset-before-setup and a bounded inactive-transfer failure path were added.
- SWD then showed DMA2 Stream 3 at `NDTR=4` with `HTIF3=1`, so the DMA FIFO tail was the remaining failure; full FIFO mode with four-word bursts and a bounded tail drain are now hardware-confirmed.
- The DMA receive path now handles the observed `DCOUNT=0` and `NDTR=4` terminal tail with a bounded direct FIFO drain.
- A FAT32 hardware scan on the reformatted 128GB SD card reached the root directory; the scan no longer attempts the library's FSInfo write-back on the read-only block device.
- FAT long-file-name enumeration now discovers host-created packages with the four-character `.amrn` extension without a package-name assumption.
- The `dali-amrn` crate decodes the fixed header, validates payload bounds and entry metadata, and verifies CRC32 with 15 host tests.
- The AMRN parser exposes separate header and incremental payload validation for a bounded streaming loader, with host coverage for the split path.
- The read-only filesystem boundary now exposes one selected root AMRN file as a bounded read/rewind stream and rejects ambiguous package selection.
- The kernel loader validates the selected package header, exact file length, and payload CRC using bounded block-sized reads; successful validation is logged.
- The `dali-amrn` crate encodes contract-valid packages, and the `dali package` command wraps a raw payload with the documented header and CRC.
- The `dali-app-hello` validation payload now has a dedicated 64 KiB SRAM linker layout and repeatable build/package recipes.
- The validation payload is intentionally limited to package construction and loader validation plus the documented native execution proof.
- The AMRN v1 execution target is STM32F405 (`0x02`); F411 remains a separate board profile until its target ID is specified.
- The loader performs a second bounded read pass to copy a validated payload into the reserved SRAM region and provides the validated ABI entry transfer; F405 hardware execution has been observed.
- The validation payload defines the documented three-flash/long-pause F405 active-high PB2 LED pattern with host coverage and physical observation.
- ABI v2 now passes a bounded kernel service table to applications and exposes the first logging service; host evidence is passing.
- On 2026-08-15, the F405 board accepted the rebuilt `hello.amrn` package and logged successful AMRN validation; the earlier observed LED behavior was continuous slow blinking and did not satisfy the documented three-flash/long-pause pattern.
- On 2026-08-15, the F405 board delivered three `[INFO][APP] Hello World from AMRN` records through USB CDC after AMRN validation. This is hardware evidence for the application logging service.
- F405 hardware testing confirmed that boot logs and all three application records reappear after a board reset with the console already open, and after closing and reopening the USB CDC connection.
- On 2026-08-15, after reseating the SD card following an SDIO timeout, the F405 board completed the full SDIO, AMRN, application logging, and corrected three-flash/long-pause LED path. The recovery indicates sensitivity in the physical SD-card connection or power path; the responsible component is not isolated.

### Incomplete or not yet accepted

- USB CDC boot logs are observable through the terminal after reset, and the targeted reset/reconnect test has passed. Formal MVP acceptance recording remains incomplete.
- The interrupt-driven USB servicing strategy has completed the targeted F405 enumeration, reset, reconnect, and boot-log test; broader MVP acceptance remains a separate gate.
- On 2026-08-13, an F405 DFU write completed, but the flashed runtime image did not answer the host's USB descriptor requests: Linux reported repeated `device descriptor read/64, error -110`, followed by `device not accepting address, error -71`. This evidence is pre-CDC and does not establish a queue or terminal fault.
- The F405 SDIO path has not completed the documented hardware acceptance evidence.
- The corrected three-flash/long-pause application LED pattern has been observed on F405 hardware; formal MVP acceptance recording remains a separate gate.
- SDK application APIs remain incomplete; the F405 loader and native LED execution path now have first physical evidence.
- Full MVP acceptance remains incomplete despite hardware evidence for application logging, reset-to-application execution, and USB reconnect behavior; the formal acceptance record still requires the complete documented procedure.
- A separate AMRN target profile for STM32F411 remains unspecified and is not part of the current execution work.

### Current priority

**P0 — Make USB CDC boot logging deterministic under blocking storage bring-up.**

The implementation phase resumed after Pico 2 SWD access became available.
The first targeted fix addresses the CDC TX delivery boundary: the backend
must flush the `usbd-serial` software buffer and preserve pending transport
progress across USB service events. Queue insertion also software-pends the
OTG_FS interrupt so configured hosts are serviced without arbitrary delays or
terminal-specific workarounds.

The next implementation must establish a tested USB lifecycle contract that can
service enumeration, CDC control requests, and log delivery while storage is
initializing. It must not rely on arbitrary sleeps, terminal-specific behavior,
or unverified interrupt code. Initial F405 CDC delivery is now hardware-
observed; the targeted reset and reconnect behavior is now evidenced.

### P0 handoff boundary

- Software architecture: implemented and target-checked. USB control/state
  servicing is owned by the `OTG_FS` interrupt, while main-context logging only
  appends to the bounded queue.
- Host evidence: implemented and passing. The `dali-usb` crate tests FIFO
  ordering, partial writes, disconnect/reconnect retention, and explicit
  overflow accounting. A deterministic lifecycle test also interleaves
  blocking storage steps with host configuration changes and bounded USB
  service budgets.
- Simulation evidence: limited. Renode cannot exercise the production USB
  backend because its STM32F4 model lacks the required OTG_FS global registers.
- F405 hardware evidence: direct USB DFU flashing produced a runtime CDC
  device; `picocom` reported `Terminal ready` and received the complete boot,
  AMRN validation, and three application records. A reset with the console
  open repeated the sequence, and a USB disconnect/reconnect repeated it again.
- Current fault boundary: the targeted CDC reset/reconnect behavior is working.
  Formal MVP acceptance remains the remaining evidence gate.
- Required next input: record the complete MVP acceptance result using the
  documented procedure.

### Next atomic tasks

- [x] Isolate the USB CDC backend behind a testable lifecycle/state interface.
- [x] Add host-side tests for USB log queue ordering, partial writes, reconnects, and overflow reporting.
- [x] Model blocking storage and USB control traffic in a deterministic host test.
- [x] Implement one reviewed USB servicing strategy for blocking boot phases.
- [x] Verify the strategy with F405 target checks and strict Clippy.
- [x] Record the first failed F405 enumeration evidence and keep it separate from CDC queue conclusions.
- [x] Verify USB enumeration and `Terminal ready` on F405 hardware.
- [x] Verify that all boot logs appear after a reset with the terminal already connected.
- [x] Record the hardware evidence before marking USB boot logging complete.
- [x] Use Pico 2 SWD to identify that USB polling runs while storage is blocked and separate enumeration from CDC TX delivery.
- [x] Preserve CDC TX data across software-buffer flush backpressure.
- [x] Trigger USB service after queue insertion when the host is already configured.
- [x] Retain boot logs until the CDC host-open signal is asserted.
- [x] Verify initial CDC TX delivery after the targeted flush and host-open fixes.
- [x] Verify CDC TX reconnect behavior and the remaining boot sequence.

## Phase 0 — Documentation baseline

- [x] Confirm the reference board is WeAct Studio STM32F405RGT6 Core Board.
- [ ] Confirm `.amrn` as the single MVP package extension.
- [ ] Record the target triple as `thumbv7em-none-eabihf`.
- [ ] Define the MVP application entry ABI.
- [ ] Reserve kernel RAM from `0x20000000` to `0x20007FFF`.
- [ ] Reserve application RAM from `0x20008000` to `0x20017FFF`.
- [ ] Reserve kernel runtime RAM from `0x20018000` to `0x2001FFFF`.
- [ ] Define the exact 32-byte `.amrn` header byte layout.
- [ ] Define the fixed application load address as `0x20008000`.
- [ ] Define CRC32 as the MVP integrity algorithm.
- [ ] Define the maximum application payload as 64 KiB.
- [ ] Define that MVP applications do not own interrupts.
- [ ] Define the LED pattern used as the application execution proof.
- [ ] Adopt `CODING_STANDARDS.md` as mandatory for all implementation work.
- [ ] Require an immediate `SAFETY` comment for every unsafe block.
- [ ] Require English comments, logs, and error messages.
- [ ] Require tests or hardware evidence for every implementation module.
- [ ] Document the SPI1 pin mapping.
- [x] Document the RTT and USB CDC logging channels used for acceptance testing.
- [ ] Review all MVP claims for unsupported security language.
- [x] Add the compile-time STM32F405 SDIO board backend.
- [ ] Record STM32F405 SDIO wiring and hardware acceptance evidence.

## Phase 1 — Kernel bootstrap

- [ ] Confirm the project builds for the embedded target.
- [ ] Confirm the linker script matches STM32F411 memory.
- [ ] Initialize core and device peripherals.
- [ ] Configure the 100 MHz system clock.
- [ ] Configure the PC13 status LED.
- [ ] Configure the 1 ms SysTick source.
- [ ] Initialize RTT logging.
- [ ] Emit a deterministic boot banner.
- [ ] Blink the status LED at a fixed interval.
- [ ] Record a hardware boot acceptance result.

## Phase 2 — SD and filesystem read path

- [ ] Add board-specific SD transport configuration for SPI1 or SDIO.
- [ ] Configure the SD chip-select pin.
- [ ] Implement SD card low-speed initialization.
- [ ] Implement one raw block read.
- [ ] Verify a known SD block read on hardware.
- [x] Integrate a read-only FAT16/FAT32 filesystem layer.
- [x] Mount the filesystem read-only.
- [x] Enumerate the root directory.
- [x] Filter files by the `.amrn` extension.
- [x] Log the discovered package path.

## Phase 3 — AMRN parser and integrity

- [x] Define the fixed header as a Rust representation.
- [x] Add parser tests for a valid header.
- [x] Reject an invalid magic value.
- [x] Reject an unsupported format version.
- [x] Reject an unsupported target.
- [x] Reject truncated headers.
- [x] Reject integer-overflowing sizes and offsets.
- [x] Reject payloads outside the package.
- [x] Reject payloads larger than the reserved SRAM region.
- [x] Implement CRC32 for the payload.
- [x] Reject CRC32 mismatches.
- [x] Expose separate header and payload validation for bounded loader reads.
- [x] Expose incremental payload validation before the future SRAM copy pass.
- [x] Define the single-root-package selection policy for the loader boundary.
- [x] Log successful header and payload validation.

## Phase 4 — RAM loading and execution

- [x] Reserve the application SRAM region in `kernel/memory.x`.
- [x] Expose the region only to the loader.
- [x] Read a validated payload into a bounded buffer.
- [x] Copy the payload to `0x20008000`.
- [x] Validate the calculated entry address.
- [x] Set the Cortex-M Thumb bit on the entry address.
- [x] Keep application-owned interrupts disabled for the MVP.
- [x] Define the application reset and return behavior.
- [x] Provide the application entry-point transfer.
- [x] Produce and physically verify the documented three-flash/long-pause LED pattern from the demo application.
- [x] Record the first end-to-end F405 loader and native LED execution result.

## Phase 5 — Developer workflow

- [x] Create the `dali-app-hello` package scaffold.
- [x] Add its target configuration.
- [x] Link the validation payload for the reserved SRAM address.
- [x] Add a repeatable package assembly step for a raw payload.
- [x] Add a package inspection command with contract and exact-length validation.
- [x] Add package CRC32 generation through the package command.
- [x] Document the SD-card installation procedure.
- [x] Document the complete MVP demo procedure and link the CLI workflow.

### CLI platform expansion

The `dali` executable is the product-facing application and device CLI. The
`just` recipes remain the repository's developer task runner for builds, tests,
formatting, CI, simulation, and local hardware workflows. New CLI commands
must have an explicit contract, command-specific documentation, stable output,
typed failures, host tests, and a documented hardware boundary where relevant.

#### Application creation and packaging

- [x] Define the application project manifest and scaffold contract.
- [x] Add `dali app new <name>` to create a new application scaffold.
- [x] Add `dali app init` to initialize an existing directory as a Dali application.
- [x] Add reproducible application template assets without embedding board-specific values in command logic.
- [x] Add host tests for application-name validation, template rendering, and scaffold file creation.
- [x] Document the `dali app new` command and generated project contract.
- [x] Support standalone scaffolding outside a Dali workspace through an explicit SDK path.
- [x] Add `dali app build` for the documented native target and profile selection.
- [x] Add `dali app package` as the application-oriented wrapper around AMRN package creation.
- [x] Decide and document compatibility between the existing top-level `dali package`/`dali inspect` commands and the application command group.

#### Target and host diagnostics

- [x] Add `dali doctor` for toolchain, target, host-permission, and required-tool diagnostics.
- [x] Add `dali target list` for supported target profiles.
- [x] Define declarative target manifests for manufacturer and compatibility metadata.
- [x] Generate and validate a typed target registry from `targets/*.toml`.
- [x] Centralize target profile metadata outside individual CLI commands.
- [ ] Map every declarative target manifest to a typed kernel board backend.
- [ ] Add `dali target info <target>` for board, MCU, ABI, AMRN, and transport metadata.

#### Device operations

- [ ] Define a transport-neutral device discovery contract.
- [ ] Add `dali device list` for probe, DFU, and runtime CDC discovery.
- [ ] Add `dali device info` for selected device and target metadata.
- [ ] Add `dali device flash --transport dfu --target <target>`.
- [ ] Add `dali device flash --transport probe --target <target>`.
- [ ] Add `dali device console [--port <path>]` for the runtime CDC console.
- [ ] Add `dali device attach --target <target>` for debug attachment.
- [ ] Add hardware tests and evidence before marking device commands accepted.

#### Application lifecycle

- [ ] Define package installation, selection, and removal semantics for the read-only MVP filesystem boundary.
- [ ] Add `dali app install` only after writable package-management semantics are specified.
- [ ] Add `dali app list` only after multi-package selection policy is specified.
- [ ] Add `dali app remove` only after safe write and recovery semantics are specified.
- [ ] Add `dali app run` only after application lifecycle and reset semantics are specified.
- [ ] Add end-to-end device tests for application lifecycle commands.

## Phase 6 — Post-MVP platform work

- [ ] Specify task and scheduler semantics.
- [ ] Specify fixed-size IPC primitives.
- [ ] Specify service discovery and capabilities.
- [ ] Add application lifecycle management.
- [ ] Add fault and watchdog policy.
- [ ] Expand the `dali` application SDK.
- [x] Add the `dali` package and device CLI command.
- [ ] Add signed package verification.
- [ ] Add kernel secure boot.
- [ ] Add version compatibility and anti-rollback.
- [ ] Add MPU-backed isolation where supported.
- [ ] Add update and rollback support.
