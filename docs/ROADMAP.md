# Dali OS Roadmap

This file is the roadmap gateway. Each phase has its own status, scope,
acceptance boundary, and backlog.

## Current status — 2026-08-25

| Phase | Status | Scope |
| --- | --- | --- |
| [01 — Kernel Core and Security](roadmap/01-kernel-core-and-security.md) | **Completed** | F405 feature-gated core, MPU, bounded DMA policy, trust path, watchdog, and storage recovery |
| [02 — Hardware Drivers and System Subsystems](roadmap/02-hardware-drivers-and-subsystems.md) | **Active** | Driver contracts, backend coverage, and lifecycle hardening |
| [03 — System GUI and Launcher](roadmap/03-system-gui-and-launcher.md) | **Future** | Dali BIOS and bounded launcher services |
| [04 — First-Stage Bootloader](roadmap/04-first-stage-bootloader.md) | **Future** | Pre-reset kernel-image verification and FSBL/ROM handoff |

## Current execution roadmap

The active driver branch follows these four phases. The order is intentional:
hardware acceptance closes the existing F405 contracts before a new bus or
display backend is introduced.

### Phase 1 — F405 bounded-timeout acceptance

- [ ] Capture F405 Timer timeout integration and expiration evidence.
- [ ] Capture external-SPI or intentionally stalled-SPI recovery evidence.
- [ ] Record firmware revision, board wiring, transport, expected output, and
  observed output in `docs/TESTING.md`.

### Phase 2 — Communication drivers and bus HAL

- [ ] Define a hardware-neutral I2C contract for bounded Write, Read, and
  Write-Read operations with repeated-start semantics.
- [ ] Implement the F405 I2C backend with non-blocking, bounded bus-lockup
  recovery.
- [ ] Add fixed-capacity host mocks and unit coverage for I2C ownership,
  repeated-start, timeout, and bus-error behavior.

### Phase 3 — Small-form-factor I2C display

- [ ] Select and specify an SSD1306 or SH1106 128x64 I2C display contract.
- [ ] Implement a bounded I2C OLED adapter only after Phase 2 acceptance.
- [ ] Add a lightweight text console for boot status, memory, and diagnostics.
- [ ] Record target rendering and unavailable-display recovery evidence.

### Phase 4 — Power and diagnostic infrastructure

- [ ] Complete accurate RCC reset-reason mapping for the supported target.
- [ ] Define and implement Ustari-based wired read-only `sysinfo`, `mem`, `ps`,
  and `dmesg` diagnostics.
- [ ] Add host, target, and hardware evidence for each diagnostic response.

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
