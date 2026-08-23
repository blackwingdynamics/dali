# Dali OS Baseline ABI v2 MVP Acceptance Test

This procedure validates the baseline ABI v2 MVP path on physical hardware:

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
`docs/TESTING.md`.

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

## 11. Recorded partial acceptance evidence

On 2026-08-15, the WeAct Studio STM32F405RGT6 board was programmed through a
Pico 2 CMSIS-DAP probe and booted from a FAT32 SD card. The USB CDC console
reported successful SDIO initialization, block-zero read, AMRN validation,
and three application log records:

```text
[INFO][STORAGE] SDIO card initialized
[INFO][STORAGE] Read block 0 successfully
[INFO][LOADER] AMRN header and payload validated
[INFO][APP] Hello World from AMRN
[INFO][APP] Hello World from AMRN
[INFO][APP] Hello World from AMRN
```

This confirms the application logging path on physical F405 hardware. It does
not by itself close the complete MVP acceptance procedure, including reset
log capture, reconnect behavior, and the full documented LED observation.

On 2026-08-15, a subsequent F405 run produced an SDIO initialization timeout
and the documented slow storage-status blink. After powering down the board,
reseating the SD card, and restarting it, the same firmware completed SDIO
initialization, read block zero, validated the AMRN package, and delivered all
three application log records. The corrected three-flash/long-pause application
pattern was also observed on the refreshed package. This is evidence that the
software path can recover after a clean card reseat; it also records that the
current hardware setup is sensitive to SD-card contact or power quality. The
observation does not identify which physical component is responsible.

On 2026-08-16, the F405 board was programmed through the Pico 2 CMSIS-DAP
probe while the STM32 USB CDC port was connected separately. The host
identified the STM32 console as `/dev/ttyACM1`, distinct from the Pico's
`/dev/ttyACM0`. The console received the complete boot sequence, successful
SDIO initialization, block-zero read, AMRN validation, and three application
log records. With the SD card removed, the board showed the documented slow
storage-status blink. After the card was inserted, it showed three short
application flashes followed by a long pause. Reset caused the USB CDC
terminal to lose its port temporarily; reopening the console showed the boot
and application logs again. This is hardware evidence for the storage-status
and application LED distinction and CDC reset/reconnect behavior. The formal
acceptance record still requires the board revision, power source, exact
package revision and CRC32, and tester fields.

### Acceptance evidence record — 2026-08-16

```text
Date: 2026-08-16 01:56
Tester: Giorgi Magradze
Board and MCU: WeAct Studio STM32F405RGT6 Core Board, STM32F405RGT6
Board revision: v1.1
Probe: Raspberry Pi Pico 2 running CMSIS-DAP
Power source: STM32 USB and Pico USB
SD card and filesystem: 128 GB microSD, FAT32
Kernel revision: 4fb7b41
Application package revision: hello.amrn
Package size: 3999 bytes
Package CRC32: 0xBFDB25F5
Observed log: Complete boot, SDIO, AMRN validation, and three application log records
Observed LED pattern: Slow storage blink without SD card; three short flashes and
  a long pause with the SD card inserted
Result: PASS
Known issues: None for the documented MVP acceptance path.
```

The package metadata was recovered from the exact `hello.amrn` file retained
on the FAT32 SD card used for the hardware run.

### Binary v2 acceptance evidence record — 2026-08-22

```text
Date: 2026-08-22 12:27:00
Tester: Giorgi Magradze
Board and MCU: WeAct Studio STM32F405RGT6 Core Board, STM32F405RGT6
Probe: Raspberry Pi Pico 2 running CMSIS-DAP
SD card and filesystem: FAT32 Binary v2 repository bundle
Kernel revision: 714339c
Package digest: 53bf7b2bd3af6b230802fc71f5ca006b67a980e3e4587c0973be140386af3d04
Observed log: SDIO initialized; trust-store read/write passed; block zero read;
  AMRN header and payload validated; AMRN signature verified; one package
  loaded into slot 1; Ready -> Running; Relocation fixture executed
Observed slot: code=0x20010000+16384 data=0x20014000+16384 psp_top=0x20015010
Result: PASS
Known issues: Package size, board revision, and power source were not captured.
  Watchdog reset/Safe Mode is tracked as a separate acceptance scenario.
```

This record proves the configured release-anchor Binary v2 path on physical
F405 hardware. It does not claim Secure Boot, production key custody, general
DMA isolation, or power-loss recovery.

### Durable trust-store flush acceptance evidence — 2026-08-22

```text
Date: 2026-08-22 13:16:45
Tester: Giorgi Magradze
Board and MCU: WeAct Studio STM32F405RGT6 Core Board, STM32F405RGT6
Probe: Raspberry Pi Pico 2 running CMSIS-DAP
SD card and filesystem: FAT32 Binary v2 repository bundle
Kernel revision: baa9a47
Package digest: 53bf7b2bd3af6b230802fc71f5ca006b67a980e3e4587c0973be140386af3d04
Observed log: Trust-store artifact write/flush/read-back passed; AMRN header
  and payload validated; AMRN signature verified; one package loaded into
  slot 1; Ready -> Running; Relocation fixture executed
Observed slot: code=0x20010000+16384 data=0x20014000+16384 psp_top=0x20015010
Result: PASS
Known issues: This run proves the bounded F405 flush/read-back sequence. A
  power-loss/interrupted-write recovery test remains separate.
```

This record proves the F405 SDIO durable-flush boundary for the acceptance
artifacts: candidate data is written, flushed, and read back before the
commit-marker artifact is written, flushed, and read back. It does not claim
power-loss recovery, production key custody, Secure Boot, or general DMA
isolation.

### Manual F405 reboot-recovery acceptance evidence — 2026-08-22

```text
Date: 2026-08-22 14:37:05
Tester: Giorgi Magradze
Board and MCU: WeAct Studio STM32F405RGT6 Core Board, STM32F405RGT6
Probe: Raspberry Pi Pico 2 running CMSIS-DAP
SD card and filesystem: FAT32 Binary v2 repository bundle
Kernel revision: d32ad62
Package digest: 53bf7b2bd3af6b230802fc71f5ca006b67a980e3e4587c0973be140386af3d04
Verification method: Manual hardware run after SD card reseat
Observed log: Committed generation selected: version=1 sequence=2 slot=B;
  trust-store write/flush/read-back passed; AMRN header and payload validated;
  AMRN signature verified; one package loaded into slot 1; Ready -> Running;
  Relocation fixture executed
Result: PASS
Known issues: This run verifies committed-journal recovery after reboot. It
  does not prove Prepared-only interruption recovery or power-loss recovery.
```

This is manually observed F405 evidence for the reboot recovery selector and
the complete signed Binary v2 boot path. The separate Prepared interruption
test remains pending.
