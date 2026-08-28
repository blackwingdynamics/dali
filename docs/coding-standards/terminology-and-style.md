# Terminology and Documentation Style

This guide defines the canonical terms and writing conventions used across
Dali OS documentation.

## Canonical technical terms

- Use `target` for a supported MCU or board build identity.
- Use `target profile` for the manifest-backed compatibility record.
- Use `platform backend` for target-specific hardware adaptation behind the
  platform facade.
- Use `manifest` for declarative TOML configuration under `targets/`.
- Use `host validation` for host-only compilation or tests.
- Use `embedded validation` for target compilation, flashing, or debugger
  checks without claiming physical behavior.
- Use `Silicon Trace` only for output captured from the real target hardware.
- Use `hardware evidence` for a physical result with board, wiring, firmware,
  power, transport, expected trace, and observed trace recorded.
- Use `guarantee` only for behavior supported by implementation and evidence.
- Use `limitation` or `unverified` for behavior that is not yet accepted.
- Use `cartridge` as the canonical Dali OS term for a deployable `.amrn`
  artifact.
- Use `application payload` for executable code contained by a cartridge.
- Do not call a `.amrn` artifact an application or cartridge in user-facing
  documentation. Reserve `cartridge` for packaging operations, Cargo cartridge
  identity, or specifications that explicitly define cartridge semantics.

## Interface and hardware names

Use the established uppercase forms `I2C`, `SPI`, `SDIO`, `UART`, `USB CDC`,
`RCC`, `GPIO`, `MPU`, `AMRN`, `ABI`, and `F405`. Use `STM32F405RGT6` when the
exact MCU identity matters. Use `F405` when referring to the supported target
path generally.

## Status vocabulary

Use these status labels consistently:

- `Completed` — implementation and required validation are complete;
- `Implementation complete, hardware validation pending` — source and
  non-hardware validation are complete, physical acceptance is open;
- `Completed, device validation pending` — the contract is complete, but
  device interoperability remains unverified;
- `Future` — not started;
- `Active` — the current work item;
- `Unverified` — an observed or implemented behavior without sufficient
  acceptance evidence.

Do not use a completed checkbox for a physical claim that lacks the required
Silicon Trace. Keep status labels and checklist state aligned.

## Markdown conventions

- Every documentation directory containing Markdown files has a `README.md`
  navigation index.
- Use one H1 title per document and descend heading levels in order.
- Use relative links for repository-local documents and verify them with
  `just docs-check`.
- Put commands in fenced `text` blocks unless shell syntax is the subject.
- Keep code, comments, logs, errors, and documentation in English.
- State scope and limitations near the claim they qualify.
