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

### Incomplete or not yet accepted

- USB CDC boot logs are now observable through the terminal after reset through the SDIO initialization milestone. Full boot-log and reconnect acceptance remains incomplete.
- The interrupt-driven USB servicing strategy has target-build evidence but has not completed physical enumeration, reconnect, and boot-log acceptance.
- On 2026-08-13, an F405 DFU write completed, but the flashed runtime image did not answer the host's USB descriptor requests: Linux reported repeated `device descriptor read/64, error -110`, followed by `device not accepting address, error -71`. This evidence is pre-CDC and does not establish a queue or terminal fault.
- The F405 SDIO path has not completed the documented hardware acceptance evidence.
- AMRN parsing, CRC32 validation, RAM loading, application entry, SDK packaging, and CLI assembly remain incomplete.

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
observed; reconnect and the remaining boot sequence are still pending.

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
- F405 hardware evidence: Pico 2 SWD reaches the firmware, RTT shows boot logs
  through SDIO initialization, and the host creates `/dev/ttyACM1` for the
  runtime CDC device; `picocom` reports `Terminal ready` and receives the boot
  logs through `[STORAGE] SDIO card initialized`.
- Current fault boundary: initial CDC delivery is working. Full boot completion
  and reconnect behavior remain unverified.
- Required next input: one targeted reconnect test and evidence for the
  remaining boot sequence.

### Next atomic tasks

- [x] Isolate the USB CDC backend behind a testable lifecycle/state interface.
- [x] Add host-side tests for USB log queue ordering, partial writes, reconnects, and overflow reporting.
- [x] Model blocking storage and USB control traffic in a deterministic host test.
- [x] Implement one reviewed USB servicing strategy for blocking boot phases.
- [x] Verify the strategy with F405 target checks and strict Clippy.
- [x] Record the first failed F405 enumeration evidence and keep it separate from CDC queue conclusions.
- [x] Verify USB enumeration and `Terminal ready` on F405 hardware.
- [ ] Verify that all boot logs appear after a reset with the terminal already connected.
- [ ] Record the hardware evidence before marking USB boot logging complete.
- [x] Use Pico 2 SWD to identify that USB polling runs while storage is blocked and separate enumeration from CDC TX delivery.
- [x] Preserve CDC TX data across software-buffer flush backpressure.
- [x] Trigger USB service after queue insertion when the host is already configured.
- [x] Retain boot logs until the CDC host-open signal is asserted.
- [x] Verify initial CDC TX delivery after the targeted flush and host-open fixes.
- [ ] Verify CDC TX reconnect behavior and the remaining boot sequence.

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
