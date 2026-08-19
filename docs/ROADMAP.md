# Dali OS Roadmap

Tasks are intentionally small. A task is complete only when its stated evidence exists. Later tasks must not silently expand the MVP.

## Current Status — 2026-08-17

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

### Next session plan

The next implementation session resumes the post-v4 multi-application work in
this order:

1. [x] Add the hardware-neutral multi-package identity and slot catalog,
   including deterministic rejection of duplicate identities and occupied
   slots.
2. [x] Enumerate real root-directory packages and feed their validated v4
   metadata into the catalog; F405 discovery and catalog rejection are
   hardware-verified.
3. [x] Integrate the bounded two-slot load path without enabling concurrent
   execution or context switching; the runtime still enters only the first
   loaded context.
4. [x] Run host and target validation and verify bounded two-package loading
   on F405 hardware. Full multi-application isolation and context switching
   remain separate work.
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

**Current priority — Harden the post-MVP platform and security contracts.**

The USB implementation phase, formal F405 MVP acceptance, and 0.1.0-alpha.2
release boundary are complete. RP2350/Pico kernel support remains deferred;
the Pico is currently used only as an external SWD probe. ABI v3, relocation,
and the single-application isolation evidence are complete for the current
F405 scope. The scalable platform/backend extraction is complete; remaining
work is contract hardening, security evidence, and post-MVP lifecycle design.

### Post-alpha2 implementation order

Work on this sequence from a dedicated feature branch, merging each coherent
milestone into `main` only after its documented validation evidence exists.

1. **DMA isolation** — [in progress] the kernel-owned SDIO path now validates
   target-declared ranges before peripheral configuration, and F405 hardware
   confirms block reads after that check. Extend the policy to every
   DMA-capable backend before claiming general DMA isolation.
2. **Watchdog and reset recovery** — [in progress] target facts,
   reset-cause logging, platform integration, kernel-heartbeat feed ownership,
   and a real F405 IWDG timeout/reset-cause test are complete. Remaining work
   is feed-failure policy implementation and explicit safe-mode recovery
   evidence.
3. **Package authenticity** — [in progress] define a bounded signature
   envelope, trust-anchor identifier, and declarative development/release
   policy. The signed AMRN container contract now uses a new versioned
   extension without changing v4; CLI signing, hardware-neutral key-id
   lookup, and target-manifest development/release trust-anchor provisioning
   are implemented and tested. The `ed25519-dalek 3.0.0`
   backend passed the no_std thumb-target check and chunked standard-Ed25519
   verification tests through the `dali-crypto` facade. Kernel loader
   verification now has a feature-gated v5 streaming path with target-profile
   trust-anchor lookup, bounded CRC/relocation checks, and authentication
   before SRAM copy. The F405 development profile provisions only the RFC8032
   test anchor behind `abi-test-fixtures`; the release profile now contains a
   locally generated public anchor, while production key custody and release
   acceptance remain pending.
   v4/v5 identity metadata now rejects undeclared required-service bits before
   any SRAM copy.
   Valid signed-package hardware evidence now exists for the documented F405
   development profile, and the loader rejects an unknown trust anchor before
   SRAM copy. Modified-content rejection now has F405 evidence through the
   package CRC path, and a truncated DSIG trailer is rejected before loading.
   Secure Boot and production release acceptance remain separate work.
   The host CLI now generates Ed25519 seeds from OS CSPRNG output and exports
   only the public trust-anchor fragment for release provisioning.
4. **Multi-developer package trust and distribution** — design and implement a
   kernel-independent developer identity contract. The shipped kernel must
   contain a Dali root public key, while developers generate and retain their
   own private keys locally. A signed developer certificate or trust-store
   update must authorize developer public keys without requiring an end user
   or application developer to rebuild the kernel. Define package key
   ownership, certificate fields, enrollment authority, offline trust-store
   updates, revocation, expiry, rotation, roles, and recovery before exposing
   a public application ecosystem. Never distribute the Dali root private key
   or a shared developer signing key.
5. **Secure Boot** — verify kernel and package authenticity, compatibility
   metadata, and anti-rollback policy.
6. **Application lifecycle** — implement and test restart, rollback, package
   replacement, and slot recovery semantics.
7. **Storage hardening** — add retry policy, media health states,
   insertion/removal handling, and tests across SD cards and filesystems.
8. **CI and release hardening** — require reproducible builds, artifact hashes,
   target builds, host tests, Clippy, and release validation.
9. **Additional board families** — add family backends and generated target
   profiles only when a second MCU family is introduced.

### Platform scalability foundation

- [x] Define the platform/backend ownership boundary and contributor workflow
  before adding additional hardware targets.
- [x] Move the F405 board implementation fully behind the platform backend
  facade without changing its runtime behavior.
- [x] Add generated backend scaffolding and validation for new target profiles.
- [x] Expose stable platform operations without leaking board resource types to
  bootstrap or heartbeat policy.
- [x] Formalize the selected backend through a crate-private `Backend` contract
  and keep F405 resource ownership behind the platform facade.

#### Recorded architecture debt

- [x] Restore `kernel/src/drivers/` as the hardware-neutral driver layer after
  the platform refactor is complete.
- [x] Define the generic block-storage contract in `kernel/src/drivers/` and
  make the F405 SDIO adapter implement it without exposing PAC or HAL types to
  kernel policy.
- [x] Define a hardware-neutral SDIO transport contract in
  `kernel/src/drivers/sdio.rs`; keep the STM32F405 PAC/HAL implementation as a
  platform backend rather than coupling generic drivers to one board.
- [x] Add host tests for the generic driver contracts and target checks for the
  F405 adapter before marking the driver boundary complete.

#### Future family-level backend scaling

- [ ] Keep the kernel dependent only on a stable, hardware-neutral platform
  facade and typed backend contracts.
- [ ] When a second board from the STM32F4 family is added, extract the shared
  implementation into a reusable `dali-backend-stm32f4` family crate; board
  manifests must remain declarative and provide board-specific facts only.
- [ ] When a new MCU family is added, introduce a dedicated family backend
  crate such as `dali-backend-rp2040` or `dali-backend-nrf52` instead of adding
  family conditionals to kernel policy modules.
- [ ] Keep board selection, capabilities, memory layout, and peripheral
  metadata in generated target profiles rather than hardcoding them in kernel
  or CLI implementation code.
- [ ] Add a backend contract test suite that every family backend must satisfy,
  including clock, storage, console, reset, and memory-protection capabilities.
- [ ] Add one CI matrix entry per supported backend family and one target-level
  validation job per application-supported board profile.

This work starts only when a second board in an existing family or a first
board in a new family is introduced. Until then, the current F405 backend
facade is the intended minimum boundary and no empty family crate is needed.

### Post-MVP device lifecycle

These tasks are intentionally deferred until the current security hardware
tests and single-application isolation foundation are accepted. They improve
operator experience and runtime resilience without changing the boot-only
storage contract during the current MVP work.

- [ ] Add console-session auto-reconnect after target reset and USB CDC
  re-enumeration; preserve the existing bounded log delivery contract and
  report reconnect state explicitly.
- [ ] Define a hardware-neutral storage lifecycle contract with explicit
  `Unavailable`, `Present`, `Ready`, `Removed`, and `Fault` states.
- [ ] Add bounded storage health checks and safe SDIO reinitialization for
  card insertion/removal while the kernel is in an idle or recovery loop.
- [ ] Add hardware evidence for card removal, reinsertion, repeated reset, and
  recovery without treating a transient physical disconnect as a kernel panic.
- [ ] Extend the lifecycle contract to future storage backends such as eMMC,
  NVMe, or SSD without adding backend-specific conditions to kernel policy.

### ABI selector foundation

- [x] Centralize the active ABI selector and keep versioned Cargo feature names
  version-neutral across implementation modules.
- [x] Centralize ABI-family and AMRN-format compatibility validation for the
  kernel-facing build contract and CLI package commands.
- [ ] Define the ABI v4 contract and implementation before enabling a v4 alias.

### Enterprise architecture hardening

- [x] Separate direct PAC/HAL adapters from kernel policy modules.
- [x] Split bootstrap orchestration from storage initialization and package
  loading policy.
- [x] Declare target capabilities in TOML and validate them during builds.
- [x] Add build-time target/ABI/format consistency gates.
- [x] Run legacy and isolation embedded checks through a CI matrix.
- [x] Add hardware-independent kernel core contract tests.

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
  target manifest and activate it only through the feature-gated ABI v3 path.
- [x] Add host-testable ABI v3 SVC identifiers and Cortex-M exception-frame
  types while keeping ABI v2 as the default kernel configuration.
- [x] Add manifest-backed aligned application code/data boundaries for the
  ABI v3 memory contract.
- [x] Add a feature-gated kernel SVC frame validator and bounded log dispatch;
  keep it disabled in the default ABI v2 MVP.
- [x] Define a bounded kernel-owned fault record for invalid exception-return
  rejection without changing ABI v2 behavior.
- [x] Add feature-gated diagnostic MemManage, BusFault, and UsageFault handlers
  that report bounded SCB status and return to kernel recovery without changing
  ABI v2 behavior.
- [x] Add a feature-gated descriptor-backed MPU register map with privileged
  default access; keep the unprivileged transition disabled in ABI v2.
- [x] Define the two-phase MPU map required for privileged ABI v3 loading:
  application code/data remain kernel-only and non-executable during copying,
  then receive their unprivileged execution permissions immediately before
  the PSP transition.
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
  code/data copying, and BSS initialization before the privilege transition.
- [x] Prepare and materialize a kernel-generated ABI v3 basic exception frame
  with validated PSP bounds and a kernel-owned exception-return selector; do
  not enable it in the default ABI v2 path.
- [x] Add an explicitly disabled-by-default PendSV transition primitive for
  the prepared PSP frame; full concurrent scheduling remains disabled.
- [x] Add a feature-gated kernel-stack fault recovery return that terminates the
  application without reusing its PSP; hardware evidence is recorded below.
- [x] Add a feature-gated no-frame HardFault recovery path for exception-entry
  failures where an application stack frame is not valid; the handler boundary
  is SWD/GDB-tested, while full restart lifecycle semantics remain open.
- [x] Implement MPU and privilege transition for one application only; keep
  watchdog, DMA, and multi-application behavior pending; repeatability evidence
  is recorded below.
- [x] Implement the SVC gateway and versioned service dispatch; rejection
  evidence is recorded below.
- [x] Implement a privileged fault boundary that records the fault context and
  terminates the application without corrupting the kernel context; F405
  evidence is recorded below.
- [x] Add host tests for ABI encoding, service identifiers, and rejected calls.
- [x] Add a non-production F405 kernel-memory fault-injection application.
- [x] Add a non-production F405 kernel-memory write fault-injection application.
- [x] Add a non-production F405 peripheral-access fault-injection application.
- [x] Add a non-production F405 peripheral-write fault-injection application.
- [x] Add a non-production F405 BusFault-address fault-injection application.
- [x] Add a non-production F405 invalid-execution fault-injection application.
- [x] Add a non-production F405 invalid-PSP fault-injection application.
- [x] Add a non-production F405 no-frame HardFault fault-injection application.
- [x] Add a non-production F405 SVC rejection-matrix application.
- [x] Add F405 SWD fault-injection tests for kernel RAM, peripherals, invalid
  execution, PSP bounds, and application service calls.
- [x] Record first F405 hardware evidence for kernel-RAM read rejection and
  kernel recovery; broader isolation acceptance remains pending.
- [x] Record F405 hardware evidence for kernel-RAM write rejection and kernel
  recovery; broader isolation acceptance remains pending.
- [x] Record F405 hardware evidence for peripheral-MMIO read rejection and
  kernel recovery; broader isolation acceptance remains pending.
- [x] Record F405 hardware evidence for peripheral-MMIO write rejection and
  kernel recovery; broader isolation acceptance remains pending.
- [x] Record F405 hardware evidence for precise BusFault decoding at the
  reserved `0x00100000` code-region boundary, including `CFSR=0x00008200`,
  `BFAR=0x00100000`, stacked `PC=0x2000807A`, stacked `LR=0x2000803D`, and
  kernel recovery.
- [x] Record F405 hardware evidence for execute-never instruction rejection
  and kernel recovery.
- [x] Record F405 hardware evidence for invalid-PSP exception-entry rejection
  and kernel recovery.
- [x] Record F405 hardware evidence for the ABI v3 SVC rejection matrix.

Detailed hardware setup, commands, logs, decoded registers, and evidence
boundaries are maintained in [`docs/TESTING.md`](TESTING.md). This roadmap
records only implementation status and links to that canonical test record.

## Test Matrix

This matrix records the evidence boundary for the current F405 isolation work.
Compilation and host tests do not replace hardware evidence.

### Passed

- [x] Formatting, host workspace checks, host tests, and strict host Clippy.
- [x] STM32F405 target check, embedded build, and embedded Clippy.
- [x] F405 DFU flashing and Pico 2 CMSIS-DAP SWD flashing.
- [x] F405 clock, LED, SDIO initialization, and block-zero read.
- [x] FAT32 root scan and `.amrn` package discovery from the SD card.
- [x] AMRN header, bounds, entry-point, payload, and CRC32 validation.
- [x] Native application load, entry transfer, three-flash/long-pause LED proof,
  and ABI v2 application logging.
- [x] USB CDC boot/application logs after reset and reconnect.
- [x] ABI v3 application launch with unprivileged Thread mode and PSP.
- [x] Kernel-RAM read rejection with `MemManage` and kernel recovery.
- [x] Kernel-RAM write rejection with `MemManage` and kernel recovery.
- [x] Peripheral-MMIO read rejection with `MemManage` and kernel recovery.
- [x] Peripheral-MMIO write rejection with `MemManage` and kernel recovery.
- [x] Execute-never instruction rejection with `MemManage` and recovery.
- [x] Invalid-PSP exception-entry rejection with recovery.
- [x] SVC rejection matrix for unknown services, invalid pointers, oversized
  messages, and invalid UTF-8.
- [x] Precise F405 BusFault decoding with `CFSR`, `BFAR`, stacked `PC/LR`, and
  kernel recovery.
- [x] No-frame HardFault recovery verified through SWD/GDB boundary tracing.
- [x] Empty-storage and empty-package boot states remain informational and
  enter the heartbeat without reporting a user-facing transport error.

### Remaining single-application isolation tests

- [x] Verify valid SVC calls and the initial service authorization policy:
  `Log` is accepted and undeclared or malformed requests are rejected.
- [ ] Verify watchdog behavior after application termination and recovery.
- [x] Verify fault-status clearing and repeatability across three repeated
  invalid-PSP resets; each run produced a fresh `UsageFault 0x00040000` and
  returned to kernel recovery.
- [x] Revalidate slot0 application loading and invalid-PSP recovery after the
  manifest-owned 16 KiB code/data migration; F405 loaded code at `0x20008000`
  and data at `0x2000C000`.

### Future multi-application and memory tests

- [x] Define the initial explicit-relocation direction and safety contract in
  `docs/RELOCATION.md`; keep ABI v2/v3 fixed-address behavior unchanged.
- [x] Produce relocation-aware fixture artifacts with retained linker records
  for code and writable data; record the observed ARM kinds in
  `docs/RELOCATION.md`.
- [ ] Test relocation metadata or the selected RWPI/PIC implementation.
- [x] Implement and test the fixed-capacity SRAM slot manager on the host.
- [x] Integrate manifest-slot reservation into kernel-owned application loading
  before adding concurrent execution.
- [x] Define the bounded single-package selection result and reject ambiguous
  root packages before adding identity-aware multi-application loading.
- [x] Define the AMRN format v4 identity, compatibility, service-requirement,
  and explicit slot-selection fields without changing ABI v3.
- [x] Implement and host-test the hardware-neutral AMRN v4 codec before
  enabling v4 packages in the kernel.
- [x] Add manifest-driven v4 package generation and inspection without
  changing the existing ABI v2/v3 package paths.
- [x] Prepare a manifest-backed v4 relocation fixture for the hardware
  selection, relocation, and recovery run.
- [x] Add a kernel-side v4 selection and bounded streaming loader path without
  changing the ABI v2/v3 loaders.
- [x] Add a bounded v4 identity/slot catalog that rejects duplicate identities,
  duplicate or externally occupied slots, and discovery-order dependence.
- [x] Add hardware-neutral AMRN v4 codec tests for selection metadata, CRC, and
  relocation validation; streaming-loader behavior remains target-tested.
- [x] Validate v4 selection, relocation, and successful application execution
  on F405 hardware; the format 4 slot1 fixture passed AMRN validation and
  executed the relocation proof application. F405 duplicate-identity and
  occupied-slot rejection are also hardware-verified; broader recovery tests
  remain open. Detailed evidence is in `docs/TESTING.md`.
- [x] Add a host-tested slot ownership/range contract before enabling two
  application contexts.
- [x] Load two applications into independent slots and verify loader placement
  boundaries on F405; runtime memory isolation and concurrent execution remain
  separate work.
- [x] Define and host-test the bounded application lifecycle state machine from
  package discovery through terminal recovery without adding restart or
  context switching behavior.
- [x] Define and host-test single active-context ownership, including duplicate
  activation rejection and terminal-only retirement without a scheduler.
- [x] Integrate the v4 loader and MPU launch path with the lifecycle and active
  context owner; F405 hardware confirmed `Loaded -> Ready -> Running` during
  two-package boot and slot0 execution.
- [x] Connect the kernel fault boundary and recovery entry to the atomic active
  context state channel; F405 hardware verified `Faulted -> Recovering ->
  Terminated` with the v4 invalid-PSP application fixture.
- [x] Define the current restart and rollback policy: terminated applications
  require a manual reset and the read-only package boundary has no rollback.
- [x] Define the watchdog safety gate: do not arm hardware watchdogs until a
  bounded heartbeat/feed owner contract exists.
- [x] Define and host-test the bounded saved-context record and context-table
  selection contract; keep PendSV, SysTick, and MPU switching disabled.
- [x] Define and host-test the board-independent SysTick quantum and one-shot
  PendSV request contract without selecting a timer frequency in runtime code.
- [x] Add target-compiled, feature-gated ARM save/restore primitives for the
  kernel-owned context record; keep scheduler selection, interrupt enablement,
  and MPU switching separate.
- [x] Add a bounded scheduler facade that sequences tick requests, active-state
  capture, and next-ready selection without owning an interrupt vector.
- [x] Define and host-test the bounded PendSV preparation transition, including
  no-op behavior without a request and save-before-selection ordering.
- [x] Generate scheduler capacity from each target manifest's declared
  isolation slots before introducing kernel-owned scheduler storage.
- [x] Add one-time kernel-owned scheduler storage with explicit exclusive
  interrupt-access requirements; keep the storage disconnected from vectors.
- [x] Move the scheduler quantum into target-profile configuration so runtime
  code does not hardcode board timing values.
- [x] Initialize kernel-owned scheduler storage from the selected target
  profile without enabling PendSV or timer interrupts.
- [x] Bind each scheduler CPU record to its manifest-owned application slot so
  a future protected switch can select CPU state and MPU layout together.
- [x] Register validated loaded application contexts and activate the first
  scheduler context without enabling timer interrupts or concurrent execution.
- [x] Add a feature-gated SysTick exception hook that accounts for target ticks
  and requests PendSV only when a ready context exists; keep timer enablement,
  register transfer, and MPU switching separate.
- [x] Add a target-compiled privileged PendSV wrapper that captures the outgoing
  CPU record and routes the selected record to the restore primitive; keep
  lifecycle ownership separate until hardware evidence.
- [x] Declare target scheduler tick frequency and enable SysTick only after the
  first loaded context is active.
- [x] Obtain first F405 hardware evidence for a slot0-to-slot1 scheduler
  handoff using the two manifest-backed v4 fixtures.
- [x] Add per-slot progress markers to the real hardware fixtures for
  repeatable context-switch observation without changing production code.
- [x] Implement and test repeated PendSV/SysTick CPU context switching with
  independent progress markers on F405; MPU region switching is verified
  separately below.
- [x] Add and run a manifest-derived slot1-to-slot0 MPU fault fixture; F405
  hardware recorded a precise MemManage with the slot0 code origin.
- [x] Retire a faulted scheduler context before selecting the next ready
  context; host tests verify that terminated contexts cannot be selected again.
- [x] Verify MPU region switching during application context switches on F405;
  GDB observed slot0 code/data bases `0x20008000`/`0x2000C000` and slot1
  bases `0x20010000`/`0x20014000` at `restore_selected`.
- [x] Verify on F405 hardware that a faulted context is excluded and a ready
  application resumes; GDB observed one slot1 entry and continued slot0
  progress after recovery.
- [x] Verify bidirectional CPU-side application-memory isolation; both
  slot1-to-slot0 and slot0-to-slot1 reads were rejected on F405 hardware.
- [x] Add and run the reverse slot0-to-slot1 CPU isolation fixture so both
  directions are hardware-tested. DMA isolation remains separate.
- [ ] Verify DMA isolation and reject unauthorized DMA configuration.
- [x] Define application crash, restart, and rollback lifecycle policy;
  hardware watchdog implementation remains deferred until heartbeat ownership.

### Future package and platform security tests

- [ ] Test package installation, selection, replacement, and removal semantics
  after writable filesystem support exists.
- [x] Define a versioned package-signature extension for Secure Boot and
  authenticity verification without modifying the AMRN v4 bytes; the host
  codec uses the DSIG trailer and has boundary/CRC coverage.
- [x] Implement the feature-gated v5 kernel streaming verifier with
  target-profile trust-anchor lookup before SRAM copy and relocation.
- [ ] Test signed package verification and rejected signatures.
- [ ] Test secure boot and kernel image authenticity.
- [ ] Test version compatibility, anti-rollback, update, and rollback flows.
- [ ] Add end-to-end CLI device and application lifecycle tests.

#### Deferred until isolation foundation is accepted

- [x] Record why compiler PIC flags alone are insufficient for raw AMRN images
  with writable data.
- [x] Choose explicit relocation metadata as the first movable-application
  direction; specify the new AMRN revision before implementation.
- [x] Define and test the new AMRN relocation-table header and entry format in
  `dali-amrn::v3`; loader and CLI support remain deferred.
- [x] Extract the bounded set of supported ARM relocation records from the
  retained application ELF in the CLI.
- [x] Validate extracted relocation records through the AMRN v3 host encoder
  and emit/inspect the AMRN v3 package behind an explicit manifest format
  selection.
- [x] Add manifest-owned relocation slot selection and build/inspect the
  relocation fixture for non-zero slot 1 on the host; detailed output is in
  `docs/TESTING.md`.
- [x] Add relocation patch decoders and host tests for each supported ARM kind.
- [x] Implement bounded kernel-side relocation application before MPU launch.
- [x] Execute a format 3 relocation fixture on F405 hardware at the canonical
  manifest origins.
- [x] Execute the same relocation metadata with a non-zero slot delta on F405
  hardware; detailed evidence is in `docs/TESTING.md`.
- [x] Define the F405 multi-slot memory contract: two 16 KiB
  code/data pairs, 32 KiB DMA-visible SRAM, and CCM kernel runtime storage.
- [x] Generate the F405 kernel linker memory map from target metadata, placing
  ordinary runtime/static state in CCM and DMA buffers in SRAM.
- [x] Migrate the application linker, loader, and MPU code/data regions to the
  manifest-owned multi-slot contract while preserving slot 0 execution.
- [x] Add and validate the manifest-owned code/data slot table without changing
  the active single-application loader contract.
- [x] Design the hardware-neutral fixed-capacity slot manager over the
  manifest-owned slot table and integrate slot 0 selection without enabling
  concurrent applications.
- [x] Route the current single-application linker, package, inspection, and
  MPU boundaries through the manifest's active slot helper; preserve slot 0
  addresses and defer slot 1 activation.
- [ ] Add PSP/PendSV context switching only after one isolated application is
  stable and its fault boundary is tested.
- [ ] Add multiple application slots and concurrent application execution.
