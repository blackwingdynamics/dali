# 11. Recorded partial acceptance evidence

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
initialization, read block zero, validated the AMRN cartridge, and delivered all
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

## Acceptance evidence record — 2026-08-16

```text
Date: 2026-08-16 01:56
Tester: Giorgi Magradze
Board and MCU: WeAct Studio STM32F405RGT6 Core Board, STM32F405RGT6
Board revision: v1.1
Probe: Raspberry Pi Pico 2 running CMSIS-DAP
Wiring: Exact SWD, USB CDC, SDIO, LED, and key wiring not recorded
Power source: STM32 USB and Pico USB
Transport: USB CDC console; Pico 2 CMSIS-DAP for flashing
SD card and filesystem: 128 GB microSD, FAT32
Kernel revision: 4fb7b41
Firmware revision: 4fb7b41 (kernel revision)
Application package revision: hello.amrn
Package size: 3999 bytes
Package CRC32: 0xBFDB25F5
Observed log: Complete boot, SDIO, AMRN validation, and three application log records
Expected trace: Successful boot, SDIO initialization, AMRN validation, and application logs
Observed LED pattern: Slow storage blink without SD card; three short flashes and
  a long pause with the SD card inserted
Result: PASS
Limitations: Exact wiring and independent electrical signals were not recorded.
Known issues: None for the documented MVP acceptance path.
```

Validation layers:

- Host: AMRN and application contract tests.
- Embedded: F405 target build.
- Flashing: Pico 2 CMSIS-DAP programming.
- Silicon Trace: USB CDC boot and application output recorded above.

The package metadata was recovered from the exact `hello.amrn` file retained
on the FAT32 SD card used for the hardware run.

### Binary v2 acceptance evidence record — 2026-08-22

```text
Date: 2026-08-22 12:27:00
Tester: Giorgi Magradze
Board and MCU: WeAct Studio STM32F405RGT6 Core Board, STM32F405RGT6
Board revision: Not recorded
Probe: Raspberry Pi Pico 2 running CMSIS-DAP
Wiring: Not recorded
Power source: Not recorded
Transport: USB CDC console; Pico 2 CMSIS-DAP for flashing
SD card and filesystem: FAT32 Binary v2 repository bundle
Kernel revision: 714339c
Package digest: 53bf7b2bd3af6b230802fc71f5ca006b67a980e3e4587c0973be140386af3d04
Observed log: SDIO initialized; trust-store read/write passed; block zero read;
  AMRN header and payload validated; AMRN signature verified; one package
  loaded into slot 1; Ready -> Running; Relocation fixture executed
Observed slot: code=0x20010000+16384 data=0x20014000+16384 psp_top=0x20015010
Result: PASS
Expected trace: SDIO, trust-store, AMRN validation, slot loading, Ready, and Running
Limitations: Package size, wiring, board revision, and power source were not captured.
Known issues: Watchdog reset/Safe Mode is tracked as a separate acceptance scenario.
```

Validation layers:

- Host: Binary v2 metadata, repository, and loader contract tests.
- Embedded: release F405 repository-loader target build.
- Flashing: Pico 2 CMSIS-DAP programming.
- Silicon Trace: USB CDC output recorded above from the F405 target.

This record proves the configured release-anchor Binary v2 path on physical
F405 hardware. It does not claim Secure Boot, production key custody, general
DMA isolation, or power-loss recovery.

### Durable trust-store flush acceptance evidence — 2026-08-22

```text
Date: 2026-08-22 13:16:45
Tester: Giorgi Magradze
Board and MCU: WeAct Studio STM32F405RGT6 Core Board, STM32F405RGT6
Board revision: Not recorded
Probe: Raspberry Pi Pico 2 running CMSIS-DAP
Wiring: Not recorded
Power source: Not recorded
Transport: USB CDC console; Pico 2 CMSIS-DAP for flashing
SD card and filesystem: FAT32 Binary v2 repository bundle
Kernel revision: baa9a47
Package digest: 53bf7b2bd3af6b230802fc71f5ca006b67a980e3e4587c0973be140386af3d04
Observed log: Trust-store artifact write/flush/read-back passed; AMRN header
  and payload validated; AMRN signature verified; one package loaded into
  slot 1; Ready -> Running; Relocation fixture executed
Observed slot: code=0x20010000+16384 data=0x20014000+16384 psp_top=0x20015010
Result: PASS
Expected trace: Trust-store artifact write, flush, read-back, and subsequent signed load
Limitations: Wiring, board revision, and power source were not captured. This run proves the bounded F405 flush/read-back sequence. A
  power-loss/interrupted-write recovery test remains separate.
```

Validation layers:

- Host: durable journal and repository write/read-back contract tests.
- Embedded: release F405 storage-write target build.
- Flashing: Pico 2 CMSIS-DAP programming.
- Silicon Trace: flush/read-back output recorded above from the F405 target.

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
Board revision: Not recorded
Probe: Raspberry Pi Pico 2 running CMSIS-DAP
Wiring: Not recorded
Power source: Not recorded
Transport: USB CDC console; manual hardware reboot after SD-card reseat
SD card and filesystem: FAT32 Binary v2 repository bundle
Kernel revision: d32ad62
Package digest: 53bf7b2bd3af6b230802fc71f5ca006b67a980e3e4587c0973be140386af3d04
Verification method: Manual hardware run after SD card reseat
Expected trace: Committed-generation selection followed by signed load and Running state
Observed log: Committed generation selected: version=1 sequence=2 slot=B;
  trust-store write/flush/read-back passed; AMRN header and payload validated;
  AMRN signature verified; one package loaded into slot 1; Ready -> Running;
  Relocation fixture executed
Result: PASS
Limitations: Wiring, board revision, and power source were not captured. This run verifies committed-journal recovery after reboot. It
  does not prove Prepared-only interruption recovery or power-loss recovery.
```

Validation layers:

- Host: journal recovery and repository-selection contract tests.
- Embedded: release F405 repository-loader target build.
- Flashing: Pico 2 CMSIS-DAP programming and manual board reboot.
- Silicon Trace: reboot and USB CDC output recorded above from the F405 target.

This is manually observed F405 evidence for the reboot recovery selector and
the complete signed Binary v2 boot path. The separate Prepared interruption
test remains pending.
