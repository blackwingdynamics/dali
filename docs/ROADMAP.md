# Dali OS Roadmap

Tasks are intentionally small. A task is complete only when its stated evidence exists. Later tasks must not silently expand the MVP.

## Current Status — 2026-08-13

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

### Incomplete or not yet accepted

- Boot logs are not yet reliably observable through the USB CDC terminal after reset.
- The current polling-based USB implementation can be starved by blocking SDIO/storage operations.
- The attempted interrupt-driven USB implementation was not accepted and must be redesigned and tested independently before reuse.
- The F405 SDIO path has not completed the documented hardware acceptance evidence.
- AMRN parsing, CRC32 validation, RAM loading, application entry, SDK packaging, and CLI assembly remain incomplete.

### Current priority

**P0 — Make USB CDC boot logging deterministic under blocking storage bring-up.**

The next implementation must establish a tested USB lifecycle contract that can
service enumeration, CDC control requests, and log delivery while storage is
initializing. It must not rely on arbitrary sleeps, terminal-specific behavior,
or unverified interrupt code. Hardware testing resumes only after host-side or
Renode-level evidence demonstrates the lifecycle behavior.

### Next atomic tasks

- [ ] Isolate the USB CDC backend behind a testable lifecycle/state interface.
- [ ] Add host-side tests for USB log queue ordering, partial writes, reconnects, and overflow reporting.
- [ ] Model blocking storage and USB control traffic in a deterministic simulation test.
- [ ] Implement one reviewed USB servicing strategy for blocking boot phases.
- [ ] Verify the strategy with F405 target checks and strict Clippy.
- [ ] Verify USB enumeration and `Terminal ready` on F405 hardware.
- [ ] Verify that all boot logs appear after a reset with the terminal already connected.
- [ ] Record the hardware evidence before marking USB boot logging complete.

## Phase 0 — Documentation baseline

- [ ] Confirm the reference board is WeAct BlackPill with STM32F411CEU6.
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
- [ ] Record STM32F405 SDIO wiring and hardware bring-up evidence.

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

- [ ] Define the fixed header as a Rust representation.
- [ ] Add parser tests for a valid header.
- [ ] Reject an invalid magic value.
- [ ] Reject an unsupported format version.
- [ ] Reject an unsupported target.
- [ ] Reject truncated headers.
- [ ] Reject integer-overflowing sizes and offsets.
- [ ] Reject payloads outside the package.
- [ ] Reject payloads larger than the reserved SRAM region.
- [ ] Implement CRC32 for the payload.
- [ ] Reject CRC32 mismatches.
- [ ] Log successful header validation.

## Phase 4 — RAM loading and execution

- [ ] Reserve the application SRAM region in `kernel/memory.x`.
- [ ] Expose the region only to the loader.
- [ ] Read a validated payload into a bounded buffer.
- [ ] Copy the payload to `0x20008000`.
- [ ] Validate the calculated entry address.
- [ ] Set the Cortex-M Thumb bit on the entry address.
- [ ] Disable application-owned interrupts for the MVP.
- [ ] Define the application reset and return behavior.
- [ ] Jump to the application entry point.
- [ ] Produce the deterministic LED pattern from the demo application.
- [ ] Record the first end-to-end hardware acceptance result.

## Phase 5 — Developer workflow

- [ ] Create the `dali-app-hello` package.
- [ ] Add its target configuration.
- [ ] Link the demo payload for the reserved SRAM address.
- [ ] Add a repeatable package assembly step.
- [ ] Add a package inspection command.
- [ ] Add a package CRC32 command.
- [ ] Document the SD-card installation procedure.
- [ ] Document the complete MVP demo procedure.

## Phase 6 — Post-MVP platform work

- [ ] Specify task and scheduler semantics.
- [ ] Specify fixed-size IPC primitives.
- [ ] Specify service discovery and capabilities.
- [ ] Add application lifecycle management.
- [ ] Add fault and watchdog policy.
- [ ] Add `dali-sdk`.
- [ ] Add `dali-cli`.
- [ ] Add signed package verification.
- [ ] Add kernel secure boot.
- [ ] Add version compatibility and anti-rollback.
- [ ] Add MPU-backed isolation where supported.
- [ ] Add update and rollback support.
