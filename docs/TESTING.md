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

## Target tests

Hardware tests should cover:

- boot banner;
- 100 MHz clock initialization;
- PC13 heartbeat;
- SPI1 SD initialization;
- one known block read;
- FAT16/FAT32 root-directory enumeration;
- `.amrn` discovery;
- payload copy to the reserved SRAM address `0x20008000`;
- demo application entry;
- deterministic application LED pattern.

## MVP acceptance test

The MVP passes only when a freshly flashed kernel discovers `hello.amrn` on the SD card, validates its 32-byte header and CRC32, loads it into the reserved SRAM region, transfers control to `unsafe extern "C" fn() -> !`, and produces the documented application LED pattern on hardware.

Build success, parser tests, or a simulated jump do not independently prove the end-to-end milestone.
