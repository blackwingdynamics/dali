# Requirements and Evidence Traceability

This document maps the principal Dali OS requirements to their implementation
owners, validation layers, and current evidence boundary. Compilation and host
tests do not replace Silicon Trace evidence.

## Evidence categories

- **Host validation** — hardware-neutral tests or host-side contract checks.
- **Embedded validation** — target compilation, ELF inspection, flashing, or
  debugger checks without claiming physical behavior.
- **Silicon Trace** — output captured from the real F405 target.
- **Unverified** — implementation or partial observation without the required
  acceptance evidence.

## Traceability matrix

| Requirement | Implementation owner | Host validation | Embedded validation | Silicon Trace | Status |
| --- | --- | --- | --- | --- | --- |
| Boot the kernel on the F405 target | `kernel/src/platform/f405/` | Not applicable | `cargo check-kernel`, release build | Boot banner and system clock recorded in [`f405-silicon.md`](f405-silicon.md) | Completed for recorded F405 path |
| Initialize board LED, user key, clock, and logging | F405 board backend and logging facade | Driver contract tests where hardware-neutral | Target build and ELF checks | LED, GPIO, timer, and logging traces recorded in [`f405-silicon.md`](f405-silicon.md) | Completed for recorded F405 path |
| Read SDIO storage and discover a cartridge | `kernel/src/storage/`, `kernel/src/platform/f405/sdio_raw/`, loader discovery | Storage and selection contracts | F405 target build | SDIO initialization and cartridge load traces recorded in [`f405-silicon.md`](f405-silicon.md) | Completed for recorded path |
| Validate AMRN cartridge bounds and CRC32 | `crates/dali-amrn/`, `kernel/src/loader/` | AMRN parser and validator tests | Embedded kernel compilation | Header and payload validation trace recorded in [`f405-silicon.md`](f405-silicon.md) | Completed for recorded path |
| Load and execute the native application payload | `kernel/src/loader/`, ABI boundary | Loader and lifecycle contract tests | Target build and ELF validation | `Ready`, `Running`, and application output recorded in [`f405-silicon.md`](f405-silicon.md) | Completed for recorded path |
| Enforce bounded timer and SPI recovery contracts | `kernel/src/platform/f405/drivers/` | Driver contract tests | F405 release build | Timer timeout and SPI recovery markers recorded in [`f405-silicon.md`](f405-silicon.md) | Completed for recorded path |
| Provide bounded I2C operations and recovery | `crates/dali-driver-api/`, F405 I2C backend | I2C ownership, repeated-start, timeout, and bus-error tests | F405 target build | Timeout and bus-recovery markers recorded; connected-device ACK remains absent | Implementation complete; device validation pending |
| Initialize and render on an I2C OLED cartridge console | Display contract, F405 OLED backend, diagnostics console | Display clipping, command, timeout, and recovery tests | Display-enabled F405 build | No connected OLED ACK or physical rendering trace recorded | Unverified |
| Preserve hardware-neutral driver boundaries | `crates/dali-driver-api/` and platform facade | API and mock tests without PAC/HAL types | Kernel target compilation | Not applicable | Completed by source/target validation |

## Evidence recording rule

Every physical acceptance record must include the board and MCU, board
revision, wiring, firmware revision, power source, transport, expected trace,
observed trace, and limitations. A successful flash or device enumeration is
embedded validation, not Silicon Trace evidence.

The canonical detailed acceptance records are maintained in
[`f405-silicon.md`](f405-silicon.md) and the procedure-specific documents in
this directory. When a requirement has no required physical trace, the matrix
must say `Not applicable` rather than imply hardware acceptance.
