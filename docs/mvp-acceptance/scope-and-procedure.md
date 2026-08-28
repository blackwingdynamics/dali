
# This procedure validates the baseline ABI v2 MVP path on physical hardware

```text
Kernel boot
  -> SD initialization
  -> AMRN discovery
  -> header and CRC32 validation
  -> SRAM loading
  -> native entry-point transfer
  -> application LED pattern
```

## 1. Acceptance scope

The MVP passes only when the kernel reads and executes an independently built `.amrn` package from an SD card on the reference board.

This is the baseline release acceptance procedure. It does not replace the
feature-gated ABI v3 isolation and AMRN v4 hardware evidence recorded in
`docs/testing/README.md`.

This test does not prove:

- sandboxing or memory isolation;
- package authenticity or digital signatures;
- application fault isolation;
- dynamic linking or relocation;
- application-owned interrupts;
- SDK or CLI completeness.

## 2. Required hardware

- WeAct Studio STM32F405RGT6 Core Board;
- SWD programmer/debug probe;
- 3.3 V microSD card in the on-board SDIO socket;
- FAT16 or FAT32 SD card;
- RTT-capable debug connection;
- PB2 status LED and PC13 user key.

Record the board revision, SD-card type, wiring, power source, and probe before testing.

## 3. Required artifacts

- kernel image built for STM32F405;
- independently built `.amrn` package; `hello.amrn` is the reference
  package name;
- package built for `thumbv7em-none-eabihf`;
- package load address: `0x20008000`;
- package payload size no greater than 64 KiB;
- package format version: `1`;
- package target ID: `0x02` (`STM32F405RGT6`);
- package ABI version: `2`;
- valid CRC32 over the payload.

The package must use the documented entry ABI:

```rust
unsafe extern "C" fn(*const ServiceTable) -> !
```

## 4. SD-card preparation

1. Format the card as FAT16 or FAT32.
2. Copy exactly one test package to the card root.
3. Use any valid `.amrn` filename; `hello.amrn` is only the reference
   package name.
4. Safely eject the card.
5. Record the actual package filename, size, format version, load address,
   entry offset, and CRC32.

The MVP does not require package installation, package deletion, or hot swap. The card is inserted before reset and remains unchanged during the test.

## 5. Test procedure

1. Connect the debug probe and RTT viewer.
2. Flash the kernel image to the STM32F405.
3. Insert the prepared SD card.
4. Reset the board.
5. Capture the complete RTT output from reset through application execution.
6. Confirm the kernel heartbeat appears before package execution.
7. Confirm the SD card is initialized and the root package is discovered.
8. Confirm the header and CRC32 are accepted.
9. Confirm the payload is loaded at `0x20008000`.
10. Confirm the loader transfers control to the application entry point.
11. Observe the application LED pattern.

## 6. Expected kernel output

The exact formatting may evolve, but the following events must be present and identifiable:

```text
====================================
   Dali OS Kernel Booting...
====================================
[INFO][BOOT] System clock: 168 MHz
[INFO][STORAGE] SDIO card initialized
[INFO][STORAGE] Read block 0 successfully
[INFO][LOADER] AMRN header and payload validated
[INFO][APP] Hello World from AMRN
[INFO][APP] Hello World from AMRN
[INFO][APP] Hello World from AMRN
```

The application submits its messages through the kernel service table; it must
not access RTT or USB CDC directly.

## 7. Expected LED behavior

Before application execution, the kernel reports storage state through the
board-specific status LED: solid on when storage is ready, slow blinking when
no card is detected, and fast blinking after a storage failure.

After the loader validates the package and transfers control, the kernel stops
controlling the LED and the application produces a distinct pattern:

```text
three short flashes -> long pause -> repeat
```

The pattern must be visibly distinguishable from every kernel storage status.
The application must not return from its entry point.

## 8. Pass criteria

The test passes only when all criteria are true:

- [x] The kernel image boots on the reference board.
- [x] The system clock reaches the documented target.
- [x] The status LED shows the correct storage status.
- [x] The SD card initializes over SDIO.
- [x] The FAT16/FAT32 filesystem is read successfully.
- [x] Exactly one root `.amrn` package is discovered.
- [x] The 32-byte header is accepted.
- [x] The target and fixed load address are accepted.
- [x] The payload size is within the 64 KiB limit.
- [x] The CRC32 matches.
- [x] The payload is copied to the reserved SRAM region.
- [x] The entry address is validated and the Thumb bit is set.
- [x] Control is transferred to the application.
- [x] Three `[INFO][APP] Hello World from AMRN` messages are delivered.
- [x] The application produces the documented three-flash LED pattern.

Build success, a valid parser test, or a simulated function-pointer call is not sufficient for an MVP pass.

## 9. Failure criteria

The test fails when any of the following occurs:

- the kernel cannot initialize the SD card;
- the filesystem cannot be read;
- the package is not discovered;
- malformed package data causes a panic or uncontrolled jump;
- an invalid CRC32 is accepted;
- the payload is copied outside the reserved SRAM region;
- the entry point is outside the payload;
- the application pattern cannot be distinguished from the kernel pattern;
- the application returns or corrupts the kernel before the test completes.

## 10. Evidence record

Every completed acceptance test must record:

```text
Date:
Tester:
Board and MCU:
Board revision:
Probe:
Power source:
SD card and filesystem:
Kernel revision:
Application package revision:
Package size:
Package CRC32:
Observed log:
Observed LED pattern:
Result: PASS / FAIL
Known issues:
```
