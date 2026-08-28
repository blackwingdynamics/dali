# Roadmap Status and Execution

This file is the roadmap gateway. Each phase has its own status, scope,
acceptance boundary, and backlog.

## Current status — 2026-08-28

| Phase | Status | Scope |
| --- | --- | --- |
| [01 — Kernel Core and Security](01-kernel-core-and-security.md) | **Completed** | F405 feature-gated core, MPU, bounded DMA policy, trust path, watchdog, and storage recovery |
| [02 — Hardware Drivers and System Subsystems](02-hardware-drivers-and-subsystems.md) | **Completed, device validation pending** | Driver contracts, F405 backend coverage, and bounded I2C recovery; physical I2C device interoperability awaits laboratory measurement |
| [03 — System GUI and Launcher](03-system-gui-and-launcher.md) | **Future — Dali BIOS backlog** | Full launcher and GUI architecture; the separate I2C OLED diagnostics console is tracked under the hardware-driver roadmap |
| [04 — First-Stage Bootloader](04-first-stage-bootloader.md) | **Future** | Pre-reset kernel-image verification and FSBL/ROM handoff |

## Current execution roadmap

The active driver branch follows these four phases. The order is intentional:
hardware acceptance closes the existing F405 contracts before a new bus or
display backend is introduced.

### Phase 1 — F405 bounded-timeout acceptance

- [x] Capture F405 Timer timeout integration and expiration evidence.
- [x] Capture external-SPI or intentionally stalled-SPI recovery evidence.
- [x] Record the available F405 Silicon Trace and its unrecorded hardware
  metadata limitations in `docs/testing/README.md`.

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
- [ ] Record physical target rendering and unavailable-display recovery
  evidence.

The Phase 3 implementation is complete, but physical OLED acceptance is
pending. The F405 Silicon Trace confirms the boot path and bounded I2C error
handling; it does not yet confirm an ACK from a connected OLED or rendering on
the physical panel. The work remains limited to an I2C OLED and did not modify
the frozen SPI/ILI9341 path.
The detailed action sequence is maintained in
[02 — Hardware Drivers and System Subsystems](02-hardware-drivers-and-subsystems.md).

### Phase 4 — Power and diagnostic infrastructure

- [ ] Complete accurate RCC reset-reason mapping for the supported target.
- [ ] Define and implement Ustari-based wired read-only `sysinfo`, `mem`, `ps`,
  and `dmesg` diagnostics.
- [ ] Add host, target, and hardware evidence for each diagnostic response.

### Current work — repository quality and structure

This is the active development track while laboratory hardware validation is
pending.

- [x] Reconcile architecture, roadmap, testing, and file-structure
  documentation with the implemented repository.
- [ ] Inventory large files, mixed responsibilities, misplaced modules, and
  obsolete or duplicated code without changing runtime behavior.
- [ ] Execute the per-file refactoring and hardware-evidence gates in
  [07 — Codebase Refactoring and Hardware Evidence Gates](07-codebase-refactoring-and-hardware-gates.md).
- [ ] Split oversized implementation files only along explicit ownership and
  responsibility boundaries.
- [ ] Preserve public APIs, generated output, boot order, frozen subsystem
  boundaries, and hardware evidence claims during refactoring.
- [ ] Validate each atomic refactoring with formatting, focused tests,
  workspace checks, and `git diff --check`.
- [ ] Keep the I2C physical acceptance item open until laboratory equipment is
  available and a real F405 trace confirms device ACK and OLED rendering.

### Current work — universal platform architecture

The next architecture track is defined in
[08 — Universal Platform Architecture](08-universal-platform-architecture.md).
Its purpose is to make the kernel core hardware-neutral so a new board can be
added in its own backend directory and target profile without modifying the
OS core or existing backends.

- [ ] Define and approve the hardware-neutral platform boundary.
- [ ] Isolate the F405 backend behind that boundary without changing its boot
  behavior or evidence claims.
- [ ] Make backend discovery and capability selection manifest-driven.
- [ ] Add Pico and FPGA/SoC backends only in their own directories after the
  boundary is accepted.
- [ ] Record target evidence independently for every backend checkpoint.

### Current work — documentation quality and enterprise readiness

- [x] Execute the documentation quality and enterprise-readiness
  roadmap in [06 — Documentation Quality and Enterprise Readiness](06-documentation-quality-and-enterprise-readiness.md).
