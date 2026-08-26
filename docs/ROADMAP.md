# Dali OS Roadmap

This file is the roadmap gateway. Each phase has its own status, scope,
acceptance boundary, and backlog.

## Current status — 2026-08-26

| Phase | Status | Scope |
| --- | --- | --- |
| [01 — Kernel Core and Security](roadmap/01-kernel-core-and-security.md) | **Completed** | F405 feature-gated core, MPU, bounded DMA policy, trust path, watchdog, and storage recovery |
| [02 — Hardware Drivers and System Subsystems](roadmap/02-hardware-drivers-and-subsystems.md) | **Completed, device validation pending** | Driver contracts, F405 backend coverage, and bounded I2C recovery; physical I2C device interoperability awaits laboratory measurement |
| [03 — System GUI and Launcher](roadmap/03-system-gui-and-launcher.md) | **Implementation complete, hardware validation pending** | Manifest-driven I2C OLED diagnostics console; physical OLED ACK and rendering evidence awaits laboratory measurement |
| [04 — First-Stage Bootloader](roadmap/04-first-stage-bootloader.md) | **Future** | Pre-reset kernel-image verification and FSBL/ROM handoff |

## Current execution roadmap

The active driver branch follows these four phases. The order is intentional:
hardware acceptance closes the existing F405 contracts before a new bus or
display backend is introduced.

### Phase 1 — F405 bounded-timeout acceptance

- [x] Capture F405 Timer timeout integration and expiration evidence.
- [x] Capture external-SPI or intentionally stalled-SPI recovery evidence.
- [x] Record the available F405 Silicon Trace and its unrecorded hardware
  metadata limitations in `docs/TESTING.md`.

### Phase 2 — Communication drivers and bus HAL — Completed 2026-08-25

- [x] Define a hardware-neutral I2C contract for bounded Write, Read, and
  Write-Read operations with repeated-start semantics.
- [x] Implement the F405 I2C backend with non-blocking, bounded bus-lockup
  recovery.
- [x] Add fixed-capacity host mocks and unit coverage for I2C ownership,
  repeated-start, timeout, and bus-error behavior.
- [x] Confirm the F405 I2C timeout and bus-recovery path with a real Silicon
  Trace on 2026-08-25.

### Phase 3 — Small-form-factor I2C display and diagnostics console

- [x] Select and specify an SSD1306 or SH1106 128x64 I2C display contract.
- [x] Implement a bounded I2C OLED adapter only after Phase 2 acceptance.
- [x] Add a lightweight text console for boot status, memory, and diagnostics.
- [x] Record target rendering and unavailable-display recovery evidence.

The Phase 3 implementation is complete, but physical OLED acceptance is
pending. The F405 Silicon Trace confirms the boot path and bounded I2C error
handling; it does not yet confirm an ACK from a connected OLED or rendering on
the physical panel. The work remains limited to an I2C OLED and did not modify
the frozen SPI/ILI9341 path.
The detailed action sequence is maintained in
[`docs/roadmap/02-hardware-drivers-and-subsystems.md`](roadmap/02-hardware-drivers-and-subsystems.md).

### Phase 4 — Power and diagnostic infrastructure

- [ ] Complete accurate RCC reset-reason mapping for the supported target.
- [ ] Define and implement Ustari-based wired read-only `sysinfo`, `mem`, `ps`,
  and `dmesg` diagnostics.
- [ ] Add host, target, and hardware evidence for each diagnostic response.

### Current work — repository quality and structure

This is the active development track while laboratory hardware validation is
pending.

- [ ] Reconcile architecture, roadmap, testing, and file-structure
  documentation with the implemented repository.
- [ ] Inventory large files, mixed responsibilities, misplaced modules, and
  obsolete or duplicated code without changing runtime behavior.
- [ ] Split oversized implementation files only along explicit ownership and
  responsibility boundaries.
- [ ] Preserve public APIs, generated output, boot order, frozen subsystem
  boundaries, and hardware evidence claims during refactoring.
- [ ] Validate each atomic refactoring with formatting, focused tests,
  workspace checks, and `git diff --check`.
- [ ] Keep the I2C physical acceptance item open until laboratory equipment is
  available and a real F405 trace confirms device ACK and OLED rendering.

## Active safety boundaries

- [x] SDIO implementation and storage behavior remain at the Known-Good
  baseline; no changes are permitted in `kernel/src/platform/f405/sdio_raw/`,
  `kernel/src/storage/`, or SDIO manifest limits during these phases.
- [x] USB CDC core servicing and polling loops are frozen.
- [x] ILI9341 and all SPI display experiments are frozen.

## Evidence rules

Completion requires implementation, relevant host/target validation, and
hardware evidence when the behavior depends on a physical target. The
canonical records are [`docs/TESTING.md`](TESTING.md),
[`docs/MVP_ACCEPTANCE.md`](MVP_ACCEPTANCE.md), and
[`docs/SECURITY.md`](SECURITY.md).

The completed security claims are scoped to the configured STM32F405 path.
They do not claim arbitrary DMA-controller isolation, confidentiality,
complete production multi-application isolation, or pre-reset Secure Boot.
The feature-gated ABI v3 context-switching and MPU-switching path remains an
experimental platform capability, while the baseline ABI v2 application
remains trusted native code.

## Working rule

Keep each phase atomic, preserve hardware-agnostic contracts, and obtain
explicit approval before changing architecture, boot path, storage layout,
loader mode, ABI, or package layout.
