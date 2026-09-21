# System GUI and Launcher

Status: **Future — Dali BIOS backlog**.

This phase is intentionally outside the current kernel and driver acceptance
scope. It must not change the ABI, storage layout, boot contract, or security
boundary without a separately approved architecture decision.

## Backlog

- Define a hardware-neutral launcher state and service contract.
- Specify display, input, power, and recovery capabilities before selecting a
  hardware backend.
- Design a Dali BIOS presentation layer for boot status, storage status,
  cartridge selection, diagnostics, and Safe Mode.
- Keep launcher code out of privileged kernel policy and application memory.
- Add a bounded event queue and failure behavior before any UI implementation.
- Define accessibility, headless operation, and serial-console fallback.

## Entry criteria

The phase starts only after the driver capability contracts and the FSBL
handoff contract are stable. A UI prototype is not evidence of a production
launcher or a secure boot path.
