# Testing Strategy

## Host tests

Host-side tests should cover pure logic without requiring an MCU:

- header parsing;
- byte order and field encoding;
- truncated input;
- invalid magic and version;
- unsupported target ID and ABI version;
- invalid header size and reserved fields;
- size and offset overflow;
- payload bounds;
- CRC32 calculation and mismatch handling;
- fixed load address and entry-offset validation.

USB delivery primitives are tested in the hardware-neutral `dali-usb` crate:

```text
cargo test -p dali-usb
```

The kernel's hardware-independent storage and driver contracts are tested
without compiling or emulating a board backend:

```text
cargo test -p dali-kernel --lib
cargo clippy -p dali-kernel --lib -- -D warnings
```

These tests use a bounded in-memory block reader to verify the generic
filesystem adapter's multi-block reads, capacity reporting, and read-only
write rejection. Embedded target checks remain separate evidence for the F405
PAC adapter.

These tests cover FIFO ordering, bounded overflow behavior, partial writes,
disconnect/reconnect retention, link-state transitions, and deterministic
interleaving of storage progress with USB service events. They do not prove USB
electrical behavior, STM32 peripheral servicing, or host enumeration.

The delivery tests must also cover a pending transport flush and its retry. A
successful queue write alone is insufficient because `usbd-serial` may retain
accepted bytes in its software buffer before the USB IN endpoint accepts them.
The lifecycle model must also represent a log enqueue that software-pends USB
service after the host is already configured; otherwise a configured-but-idle
host can leave queued records unserved.
Tests must also distinguish a configured device from a host-open CDC terminal,
and retain records until the latter is true.

## Target tests

Hardware tests should cover:

- boot banner;
- 168 MHz clock initialization;
- board-specific storage status LED behavior;
- hardware SDIO initialization;
- one known block read;
- FAT16/FAT32 root-directory enumeration;
- `.AMRN` extension filtering and arbitrary package-name discovery;
- payload copy to the reserved SRAM address `0x20008000`;
- demo application entry;
- deterministic application LED pattern.

## Recorded F405 hardware evidence

The following tests have been executed on the STM32F405RGT6 board with a
Raspberry Pi Pico 2 CMSIS-DAP probe and USB CDC console. These records are
evidence of the listed behavior only; they do not claim DMA isolation,
multi-application isolation, or watchdog support.

### Boot, storage, and application path

- [x] F405 boot, 168 MHz clock, PB2 LED, SDIO initialization, and block-zero
  read.
- [x] FAT32 root scan and `.amrn` package discovery.
- [x] AMRN validation, bounded application load, entry transfer, and the
  three-flash/long-pause application LED pattern.
- [x] ABI v2 application logging through USB CDC.
- [x] Empty SD/package states remain informational and enter the heartbeat.

### ABI v3 isolation and fault recovery

- [x] Kernel-RAM read and write rejection with `MemManage` recovery.
- [x] Peripheral-MMIO read and write rejection with `MemManage` recovery.
- [x] Execute-never instruction rejection with `MemManage` recovery.
- [x] Invalid-PSP exception-entry rejection with recovery.
- [x] Precise BusFault decoding with `CFSR`, `BFAR`, and stacked `PC/LR`.
- [x] No-frame HardFault recovery verified through SWD/GDB boundary tracing.
- [x] SVC rejection matrix for unknown services, invalid pointers, oversized
  messages, and invalid UTF-8.
- [x] Accepted `Log` SVC and initial service authorization policy.
- [x] Three repeated invalid-PSP reset cycles, each producing fresh
  `UsageFault 0x00040000` status and kernel recovery.
- [x] Post-migration slot0 smoke test after moving the manifest contract to
  16 KiB code/data slots: AMRN loaded with code `0x20008000` and data
  `0x2000C000`, invalid-PSP recovery remained `UsageFault 0x00040000`.

### Host relocation and non-zero slot evidence

- [x] The relocation fixture was packaged with AMRN format 3 using the
  manifest-owned `slot = "slot1"` selection. Its linked bases remained code
  `0x20008000` and data `0x2000C000`, while the package load addresses were
  code `0x20010000` and data `0x20014000`.
- [x] Host inspection accepted the slot1 package with 49 retained relocation
  records, 8 bytes of initialized data, and 4 bytes of zero-initialized data.
- [x] F405 hardware executed the same relocated slot1 package after the MPU
  and SVC slot-selection fix. The console reported AMRN validation followed by
  `[INFO][APP] Relocation fixture`; the same result was observed after CDC
  reconnect. The first reset produced a USB transport disconnect when the
  board cable moved, not a kernel fault.

### Diagnostic evidence

- Precise kernel-RAM read decoding preserved `CFSR=0x00000082`,
  `MMFAR=0x20000000`, stacked `PC=0x200080F6`, and stacked `LR=0x200080B9`
  before recovery.
- The reserved-address BusFault fixture preserved `CFSR=0x00008200`,
  `BFAR=0x00100000`, stacked `PC=0x2000807A`, and stacked `LR=0x2000803D`.
- The no-frame HardFault SWD/GDB trace reached `HardFault`,
  `handle_hard_fault`, `handle_with_frame`, and `recover`; the debugger's
  post-fault unwind message is not acceptance evidence.
- The SVC rejection matrix kept the application alive, while the bounded
  `Log` service accepted the final completion message.

The USB console may report `read zero bytes from port` while the target resets
or the CDC device re-enumerates. That is a transport-session event and must be
distinguished from the kernel's fault and recovery records.

## Not yet evidenced

- [ ] Watchdog behavior after application termination; watchdog support is not
  implemented yet.
- [ ] Alternative RWPI/PIC contract behavior; explicit relocation metadata is
  hardware-verified.
- [x] Host-level SRAM slot allocation, exact reservation, occupied-slot
  rejection, undeclared-slot rejection, and release/reuse behavior.
- [ ] Hardware two-application boundary isolation; only one application is
  loaded and executed at a time.
- [ ] PendSV/SysTick context switching and MPU region switching.
- [ ] DMA isolation.
- [ ] Application restart, timeout, and watchdog lifecycle policy.

## MVP acceptance test

The MVP passes only when a freshly flashed kernel discovers an `.amrn` package on the SD card, validates its 32-byte header and CRC32, loads it into the reserved SRAM region, transfers control to `unsafe extern "C" fn(*const ServiceTable) -> !`, produces the documented application LED pattern, and delivers the application log messages on hardware.

Build success, parser tests, or a simulated jump do not independently prove the end-to-end milestone.
