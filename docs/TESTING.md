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
- 100 MHz clock initialization;
- board-specific storage status LED behavior;
- SPI1 SD initialization;
- one known block read;
- FAT16/FAT32 root-directory enumeration;
- `.AMRN` extension filtering and arbitrary package-name discovery;
- payload copy to the reserved SRAM address `0x20008000`;
- demo application entry;
- deterministic application LED pattern.

## MVP acceptance test

The MVP passes only when a freshly flashed kernel discovers an `.amrn` package on the SD card, validates its 32-byte header and CRC32, loads it into the reserved SRAM region, transfers control to `unsafe extern "C" fn(*const ServiceTable) -> !`, produces the documented application LED pattern, and delivers the application log messages on hardware.

Build success, parser tests, or a simulated jump do not independently prove the end-to-end milestone.
