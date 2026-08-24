# Dali OS Roadmap

This file is the roadmap gateway. Each phase has its own status, scope,
acceptance boundary, and backlog.

## Current status — 2026-08-23

| Phase | Status | Scope |
| --- | --- | --- |
| [01 — Kernel Core and Security](roadmap/01-kernel-core-and-security.md) | **Completed** | F405 feature-gated core, MPU, bounded DMA policy, trust path, watchdog, and storage recovery |
| [02 — Hardware Drivers and System Subsystems](roadmap/02-hardware-drivers-and-subsystems.md) | **Active** | Driver contracts, backend coverage, and lifecycle hardening |
| [03 — System GUI and Launcher](roadmap/03-system-gui-and-launcher.md) | **Future** | Dali BIOS and bounded launcher services |
| [04 — First-Stage Bootloader](roadmap/04-first-stage-bootloader.md) | **Future** | Pre-reset kernel-image verification and FSBL/ROM handoff |

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
