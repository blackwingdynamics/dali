# Dali OS Roadmap

Tasks are intentionally small. A task is complete only when its stated evidence exists. Later tasks must not silently expand the MVP.

## Current Status — 2026-08-15

### Completed and evidenced

- The repository is a Rust workspace with kernel, SDK, CLI, and demo-application boundaries.
- The STM32F411 BlackPill backend has been removed; its board facts now live in
  a generator-only `targets/f411.toml` profile.
- The WeAct STM32F405RGT6 Core Board backend exists with its 8 MHz HSE, 168 MHz system clock, PB2 LED, and SDIO pin mapping.
- The F405 kernel builds, checks, and passes strict target Clippy.
- DFU flashing of the F405 firmware completes successfully.
- The F405 USB CDC device has enumerated as `1209:da11` and created `/dev/ttyACM0` during hardware testing.
- Read-only FAT filesystem integration, root-directory enumeration, AMRN extension filtering, and package discovery logging are implemented.
- Cross-platform setup scripts, Just recipes, the USB console helper, and development documentation exist.
- A fixed-capacity USB log queue exists in kernel RAM and records overflow instead of silently hiding it.
- The hardware-neutral `dali-usb` crate provides bounded delivery, link state, partial-write handling, and host tests without owning USB hardware.
- The production USB backend services the `usb-device` state machine from the
  F405 `OTG_FS` interrupt while main-context logging only appends to the bounded queue.
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
- The AMRN v1 execution target is STM32F405 (`0x02`); F411 is a generator-only
  board profile without an AMRN target ID.
- The loader performs a second bounded read pass to copy a validated payload into the reserved SRAM region and provides the validated ABI entry transfer; F405 hardware execution has been observed.
- The validation payload defines the documented three-flash/long-pause F405 active-high PB2 LED pattern with host coverage and physical observation.
- Board metadata is generated from typed registry data: F405 is application-supported, while the F411 manifest is a generator-only board profile without storage or AMRN compatibility claims.
- The target manifest schema is documented with field rules and a complete F405 example.
- The F411 kernel backend and its build/flash routes were removed; F405 is now the only kernel backend and hardware recipe.
- ABI v2 now passes a bounded kernel service table to applications and exposes the first logging service; host evidence is passing.
- On 2026-08-15, the F405 board accepted the rebuilt `hello.amrn` package and logged successful AMRN validation; the earlier observed LED behavior was continuous slow blinking and did not satisfy the documented three-flash/long-pause pattern.
- On 2026-08-15, the F405 board delivered three `[INFO][APP] Hello World from AMRN` records through USB CDC after AMRN validation. This is hardware evidence for the application logging service.
- F405 hardware testing confirmed that boot logs and all three application records reappear after a board reset with the console already open, and after closing and reopening the USB CDC connection.
- On 2026-08-15, after reseating the SD card following an SDIO timeout, the F405 board completed the full SDIO, AMRN, application logging, and corrected three-flash/long-pause LED path. The recovery indicates sensitivity in the physical SD-card connection or power path; the responsible component is not isolated.
- On 2026-08-16, a Raspberry Pi Pico 2 running CMSIS-DAP successfully programmed
  the F405 through SWD. The firmware started immediately after probe flashing
  and the application logs were delivered through USB CDC.
- On 2026-08-16, the F405 storage-status and application LED distinction was
  physically verified: no SD card produced the slow storage-status blink, while
  the inserted card produced three short application flashes followed by a
  long pause. The STM32 CDC console also repeated the boot and application
  logs after reset and reconnect.
- On 2026-08-16, the complete F405 MVP acceptance record was closed as PASS.
  The exact `hello.amrn` package used by the SD-card run was inspected from
  the FAT32 card: 3999 bytes total, 3967-byte payload, CRC32
  `0xBFDB25F5`, format version 1, target ID `0x02`, and ABI version 2.

### Historical evidence and remaining limitations

- On 2026-08-13, an F405 DFU write completed, but the flashed runtime image did not answer the host's USB descriptor requests: Linux reported repeated `device descriptor read/64, error -110`, followed by `device not accepting address, error -71`. Later firmware and hardware testing resolved the documented MVP path; this remains historical evidence, not an open acceptance failure.
- SDK application APIs remain intentionally limited; the F405 loader and native LED execution path are the accepted MVP boundary.
- The F411 profile is intentionally not part of current kernel execution work.

### Current priority

**Current priority — Build the F405 single-application isolation foundation.**

The USB implementation phase, formal F405 MVP acceptance, and 0.1.0-alpha.1
release boundary are complete. RP2350/Pico kernel support remains deferred;
the Pico is currently used only as an external SWD probe. The next work is the
ABI v3 implementation and its host/SWD evidence.

### USB CDC handoff boundary

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
- Current fault boundary: the targeted CDC reset/reconnect behavior and formal
  F405 MVP acceptance are complete.
- Required next input: release validation and post-MVP platform work.

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
- [x] Confirm `.amrn` as the single MVP package extension.
- [x] Record the target triple as `thumbv7em-none-eabihf`.
- [x] Define the MVP application entry ABI.
- [x] Reserve kernel RAM from `0x20000000` to `0x20007FFF`.
- [x] Reserve application RAM from `0x20008000` to `0x20017FFF`.
- [x] Reserve kernel runtime RAM from `0x20018000` to `0x2001FFFF`.
- [x] Define the exact 32-byte `.amrn` header byte layout.
- [x] Define the fixed application load address as `0x20008000`.
- [x] Define CRC32 as the MVP integrity algorithm.
- [x] Define the maximum application payload as 64 KiB.
- [x] Define that MVP applications do not own interrupts.
- [x] Define the LED pattern used as the application execution proof.
- [x] Adopt `CODING_STANDARDS.md` as mandatory for all implementation work.
- [x] Require an immediate `SAFETY` comment for every unsafe block.
- [x] Require English comments, logs, and error messages.
- [x] Require tests or hardware evidence for every implementation module.
- [x] Document the STM32F405 SDIO pin mapping.
- [x] Document the RTT and USB CDC logging channels used for acceptance testing.
- [x] Review all MVP claims for unsupported security language.
- [x] Add the compile-time STM32F405 SDIO board backend.
- [x] Complete the formal STM32F405 wiring and MVP acceptance evidence record.

## Phase 1 — Kernel bootstrap

- [x] Confirm the project builds for the embedded target.
- [x] Confirm the linker script matches the F405 reference memory.
- [x] Initialize core and device peripherals.
- [x] Configure the 168 MHz F405 system clock from the 8 MHz HSE.
- [x] Configure the active-high PB2 status LED.
- [x] Configure the SysTick-backed blocking delay source.
- [x] Initialize RTT and USB CDC logging boundaries.
- [x] Emit a deterministic boot banner.
- [x] Blink the storage-status LED at documented intervals.
- [x] Implement and observe the F405 boot path; formal acceptance recording is tracked in Phase 0.

## Phase 2 — SD and filesystem read path

- [x] Add the F405 board-specific SDIO transport configuration.
- [x] Configure the F405 SDIO command, clock, and four data pins.
- [x] Implement SD card initialization through the board SDIO backend.
- [x] Implement one raw block read with the bounded DMA receive path.
- [x] Verify a known block-zero read on F405 hardware.
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
- [x] Connect the F405 backend clock and AMRN compatibility checks to its generated target profile.
- [x] Add target scaffold generation for reviewable board backend templates.
- [x] Document the complete `targets/*.toml` manifest contract and generation workflow.
- [ ] Add typed kernel backends for additional target profiles after the F405 0.1.0 scope is complete.
- [x] Add `dali target info <target>` for board, MCU, ABI, AMRN, memory, clock, and transport metadata, with a machine-readable probe-chip field.
- [x] Add target-registry lookup by AMRN identifier and host-side `dali inspect`
  validation for the documented AMRN v2 package contract.
- [x] Generate an ABI v3 linker layout from target isolation metadata and
  produce host-inspectable code/data AMRN v2 packages; kernel execution remains
  disabled until the launch path is implemented.

#### Device operations

- [x] Define a transport-neutral device discovery contract.
- [x] Add the hardware-neutral normalized device record model and ordering tests.
- [x] Add `dali device list` for probe, DFU, and Linux CDC discovery; CDC records without serials remain explicitly unidentified.
- [x] Add `dali device info` for selected device and target metadata.
- [x] Add manifest-driven DFU flashing with explicit flags and the short `dali device flash <target>` artifact form.
- [x] Add manifest-driven probe flashing with the short `dali device flash <target> --transport probe` form.
- [x] Add `dali device console [--port <path>]` for the runtime CDC console.
- [x] Add `dali device attach --target <target>` for debug attachment.
- [x] Record F405 hardware evidence for device discovery, USB CDC console,
  DFU flashing, and Pico CMSIS-DAP probe flashing.

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
- [ ] Add update and rollback support.

### Phase 6A — F405 application isolation foundation

This phase is intentionally limited to the existing STM32F405 target. It does
not add another board, relocation, or multitasking until the single-application
protection boundary is implemented and accepted.

#### Design constraints

- [x] Update `docs/ABI.md`, `docs/ARCHITECTURE.md`, `docs/HARDWARE.md`,
  `docs/SECURITY.md`, and `docs/VERSIONING.md` before implementation.
- [x] Define ABI v3 around an SVC-based service gateway; direct calls into
  privileged kernel functions are not an isolation boundary.
- [x] Define the privileged kernel Thread-mode bootstrap, unprivileged
  application Thread mode, MSP ownership, and application PSP ownership.
- [x] Define the MPU region budget for kernel RAM, runtime stack, application
  code, application data/stack, peripherals, and future shared memory.
- [x] Reconcile MPU power-of-two alignment with the current application region;
  do not assume `0x20008000` can represent one 64 KiB MPU region.
- [x] Evaluate the STM32F405 CCM RAM (`0x10000000`) for kernel stack/runtime
  use, while keeping SDIO and USB DMA buffers in DMA-accessible SRAM.
- [x] Define whether application code may read or execute from kernel Flash;
  `Read-Only` is not equivalent to confidentiality or complete kernel
  protection.
- [x] Define the fault policy for MemManage, BusFault, UsageFault, and
  exception-return failures without claiming automatic recovery.

#### Implementation and evidence order

- [x] Add a board-owned typed MPU layout descriptor sourced from the F405
  target manifest; keep hardware activation deferred until the ABI boundary is
  complete.
- [x] Add host-testable ABI v3 SVC identifiers and Cortex-M exception-frame
  types without enabling the new ABI in the kernel.
- [x] Add manifest-backed aligned application code/data boundaries for the
  planned ABI v3 memory contract.
- [x] Add a feature-gated kernel SVC frame validator and bounded log dispatch;
  keep it disabled in the default ABI v2 MVP.
- [x] Define a bounded kernel-owned fault record for invalid exception-return
  rejection without installing handlers or changing ABI v2 behavior.
- [x] Add feature-gated diagnostic MemManage, BusFault, and UsageFault handlers
  that report bounded SCB status and halt without changing ABI v2 behavior.
- [x] Add a feature-gated descriptor-backed MPU register map with privileged
  default access; keep the unprivileged transition disabled.
- [x] Define ABI v3 AMRN compatibility, linker regions, and launch-frame
  validation before changing application entry or stack semantics. ABI v3
  packages use AMRN format version 2 with separate code and initialized-data
  segments, explicit zero-data and PSP stack reservations, and a kernel-built
  launch frame; the v1 parser and builder remain unchanged.
- [x] Add a host-side, no-std-compatible AMRN v2 parser and builder with
  target-contract validation; keep kernel loading on ABI v2 while the CLI can
  emit host-inspectable ABI v3 packages.
- [x] Add the feature-gated SDK-side ABI v3 SVC logging call; keep generated
  ABI v2 applications on the direct service-table path.
- [x] Add a feature-gated ABI v3 streaming loader with separate CRC validation,
  code/data copying, and BSS initialization; keep privilege transition deferred.
- [x] Prepare and materialize a kernel-generated ABI v3 basic exception frame
  with validated PSP bounds and a kernel-owned exception-return selector; do
  not enter it yet.
- [x] Add an explicitly disabled-by-default PendSV transition primitive for
  the prepared PSP frame; keep fault recovery and acceptance evidence pending.
- [ ] Implement MPU and privilege transition for one application only.
- [ ] Implement the SVC gateway and versioned service dispatch.
- [ ] Implement a privileged fault boundary that records the fault context and
  terminates the application without corrupting the kernel context.
- [ ] Add host tests for ABI encoding, service identifiers, and rejected calls.
- [ ] Add F405 SWD fault-injection tests for kernel RAM, peripherals, invalid
  execution, PSP bounds, and application service calls.
- [ ] Record hardware evidence before claiming application isolation.

#### Deferred until isolation foundation is accepted

- [ ] Define a true position-independent application contract; compiler PIC
  flags alone are not sufficient for raw AMRN images with writable data.
- [ ] Choose between a documented RWPI/PIC model and explicit relocation
  metadata in a new AMRN format revision.
- [ ] Design a slot manager only after the relocation contract and memory map
  are stable.
- [ ] Add PSP/PendSV context switching only after one isolated application is
  stable and its fault boundary is tested.
- [ ] Add multiple application slots and concurrent application execution.
