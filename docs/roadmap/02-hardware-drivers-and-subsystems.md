# Hardware Drivers and System Subsystems

Status: **Active**.

The current implementation target is the WeAct STM32F405RGT6 board. Hardware
specific facts remain in the F405 platform boundary and target manifest;
generic contracts must remain hardware-agnostic.

## Existing driver baseline

- Board-owned clock, GPIO, SDIO, USB CDC, watchdog, and interrupt ownership.
- Read-only FAT16/FAT32 access and bounded repository streaming.
- Kernel-owned SDIO transfer buffers in DMA-accessible SRAM.
- Bounded storage lifecycle state machine:
  `Unavailable -> Present -> Ready -> Removed/Fault`.
- Safe recovery heartbeat with bounded SDIO reinitialization after reinsertion.
- Kernel-owned watchdog servicing around opaque storage and verification work.

F405 hardware records for these paths are maintained in
[`docs/TESTING.md`](../TESTING.md). Driver claims must remain specific to the
observed backend and board configuration.

## Active backlog

- Add target-independent driver capability contracts without moving board
  facts into generic storage, loader, or policy modules.
- Verify removal and reinsertion while an application is already running;
  current recovery does not automatically relaunch an application.
- Add bounded fault telemetry for SDIO, USB, and watchdog transitions.
- Review every future DMA-capable backend under the same ownership policy;
  arbitrary DMA-controller isolation is not complete.
- Add cross-target watchdog semantics only when another watchdog backend is
  introduced.
- Define power-loss-safe storage update acceptance before writable production
  installation is enabled.

## Acceptance gate

Every driver slice requires host tests for hardware-neutral policy, the
appropriate target build and Clippy checks, and F405 evidence when behavior
depends on physical peripherals. No simulated device or fake storage is an
acceptance substitute.
