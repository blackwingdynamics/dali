# Recorded F405 hardware evidence

Board identity, pinout, wiring, clocks, and memory facts are canonical in
[the STM32F405 board documentation](../boards/stm32f405/README.md). This file
retains the historical execution records, expected markers, observed traces,
and evidence status for those facts.

The following tests have been executed on the STM32F405RGT6 board with a
Raspberry Pi Pico 2 CMSIS-DAP probe and USB CDC console. These records are
evidence of the listed behavior only; they do not claim arbitrary
DMA-controller isolation or multi-application isolation beyond the documented
F405 scope.

## Aggregate evidence metadata

The sections below aggregate multiple F405 runs rather than describing one
single firmware image. The metadata that is common to the recorded runs is:

```text
Board and MCU: WeAct Studio STM32F405RGT6 Core Board, STM32F405RGT6
Board revision: v1.1 for the reported I2C/OLED setup; historical runs may differ
Wiring: Reported I2C/OLED setup used SWDIO, SWCLK, GND, PB6=SCL, PB7=SDA, VCC, and GND
Firmware revision: Varies by acceptance record; see the procedure-specific record
Power source: USB for the reported I2C/OLED setup
Transport: Raspberry Pi Pico 2 CMSIS-DAP/SWD and USB CDC console where stated
Expected trace: The markers and register observations defined by each section
Observed trace: The evidence explicitly quoted by each section
Limitations: Aggregate entries must not be generalized beyond their quoted behavior
```

The procedure-specific records in [`host.md`](host.md),
[`../mvp-acceptance/recorded-evidence.md`](../mvp-acceptance/recorded-evidence.md),
and [`signed-cartridges.md`](signed-cartridges.md) remain the source for
run-specific metadata and status.

## Boot, storage, and application path

### Latest F405 signed-cartridge regression — 2026-08-29

The release F405 image built from source revision `0cfa1dd` was flashed with
the Raspberry Pi Pico 2 CMSIS-DAP/SWD probe at 1000 kHz. The board's USB CDC
console was opened using the path reported by `dali device list`. The SD card
contained Binary v2 generation `version=1`, `sequence=2`, `slot=B`, with the
current 329-byte AMRN cartridge.

The target reported `Reset cause: Software`, initialized SDIO, loaded the
Root, Bundle, Timestamp, Snapshot, and Revocations roles, reconstructed trust
state, verified the AMRN signature, and loaded slot 0 with code at
`0x20008000` and data at `0x2000C000`. The observed application output was
exactly one:

```text
[INFO][APP] Hello World from AMRN
```

Result: passed for this F405 boot, Binary v2 repository, signed-cartridge,
and USB CDC path. This is not evidence for I2C/OLED rendering, Secure Boot,
arbitrary DMA isolation, or multi-application isolation.

- [x] F405 boot, 168 MHz clock, PB2 LED, PC13 key, SDIO initialization, and block-zero
  read.
- [x] FAT32 root scan and `.amrn` cartridge discovery.
- [x] AMRN validation, bounded application load, entry transfer, and the
  three-flash/long-pause application LED pattern.
- [x] ABI v2 application logging through USB CDC.
- [x] Empty SD/cartridge states remain informational and enter the heartbeat.

### ABI v3 isolation and fault recovery

- [x] Kernel-RAM read and write rejection with `MemManage` recovery.
- [x] Peripheral-MMIO read and write rejection with `MemManage` recovery.
- [x] Execute-never instruction rejection with `MemManage` recovery.
- [x] Invalid-PSP exception-entry rejection with recovery.
- [x] Precise BusFault decoding with `CFSR`, `BFAR`, and stacked `PC/LR`.
- [x] No-frame HardFault handler and kernel recovery boundary verified through
  SWD/GDB tracing; complete application restart lifecycle remains unverified.
- [x] SVC rejection matrix for unknown services, invalid pointers, oversized
  messages, and invalid UTF-8.
- [x] Accepted `Log` SVC and initial service authorization policy.
- [x] Three repeated invalid-PSP reset cycles, each producing fresh
  `UsageFault 0x00040000` status and kernel recovery.
- [x] Post-migration slot0 smoke test after moving the manifest contract to
  16 KiB code/data slots: AMRN loaded with code `0x20008000` and data
  `0x2000C000`, invalid-PSP recovery remained `UsageFault 0x00040000`.

### Host relocation and non-zero slot evidence

- [x] The relocation fixture was cartridged with AMRN format 3 using the
  manifest-owned `slot = "slot1"` selection. Its linked bases remained code
  `0x20008000` and data `0x2000C000`, while the cartridge load addresses were
  code `0x20010000` and data `0x20014000`.
- [x] Host inspection accepted the slot1 cartridge with 49 retained relocation
  records, 8 bytes of initialized data, and 4 bytes of zero-initialized data.
- [x] F405 hardware executed the same relocated slot1 cartridge after the MPU
  and SVC slot-selection fix. The console reported AMRN validation followed by
  `[INFO][APP] Relocation fixture`; the same result was observed after CDC
  reconnect. The first reset produced a USB transport disconnect when the
  board cable moved, not a kernel fault.
- [x] F405 hardware regression after kernel-owned slot reservation was
  integrated: the slot1 cartridge again passed AMRN validation and emitted
  `[INFO][APP] Relocation fixture`. This verifies that reservation state does
  not change the established single-application relocation path.

### AMRN v4 hardware evidence

- [x] F405 hardware accepted the manifest-backed AMRN format 4 relocation
  cartridge: the console reported AMRN validation followed by
  `[INFO][APP] Relocation fixture`.
- [x] The observed cartridge contained identity metadata, `slot_id = 1`, and 49
  relocation records; the application executed from the selected non-zero slot.
- [x] The probe shutdown warning after flashing was classified as a transport
  teardown event after the target continued running, not as a loader failure.
- [x] F405 hardware loaded two distinct AMRN v4 cartridges in one boot: the
  loader reported `Loaded 2 application cartridge(s) into declared slots`, and
  deterministic slot selection entered the slot0 fixture. A later CDC console
  attach showed no replay because prior records had already drained.
- [x] F405 hardware rejected two real cartridges carrying the same identity with
  `CartridgeCatalog(DuplicateIdentity)` and entered the kernel heartbeat without
  executing either application.
- [x] F405 hardware rejected two real cartridges with different identities that
  claimed the same slot with `CartridgeCatalog(SlotOccupied)` and entered the
  kernel heartbeat without executing either application.
- [x] F405 hardware reported both loaded cartridges in their manifest-owned
  regions: slot0 code `0x20008000+16384`, data `0x2000C000+16384`, PSP top
  `0x2000D00C`; slot1 code `0x20010000+16384`, data `0x20014000+16384`, PSP
  top `0x2001500C`. This verifies loader placement and reservations, not
  runtime memory isolation or concurrent execution.
- [x] F405 hardware verified the v4 invalid-PSP lifecycle fault path. The
  console reported `Ready`, `Running`, `UsageFault 0x00040000`, then
  `Faulted`, `Recovering`, and `Terminated`; recovery entered the kernel
  recovery loop without restarting the application.
- [x] On 2026-08-20, F405 hardware repeated the slot0 invalid-PSP fixture with
  the explicit `abi-test-fixtures` kernel feature. The release image logged
  `UsageFault 0x00040000`, `Faulted`, `Recovering`, and `Terminated` after the
  signed AMRN cartridge launched; no application restart was observed before the
  test ended. This records application lifecycle recovery, not the separate
  watchdog-duration proof after termination.

### DMA isolation

- [x] Host contract tests validate kernel DMA-buffer range, alignment, empty
  range, and overflow rejection.
- [x] The hardware-neutral DMA ownership policy rejects application-owned DMA
  requests and authorizes only kernel-owned transport configuration.
- [x] The F405 SDIO path validates its manifest-declared DMA buffer before
  programming DMA2 and retains exclusive mutable ownership for the transfer.
- [x] F405 hardware evidence that the guarded SDIO path remains operational
  after the ownership check: firmware reported `SDIO card initialized` and
  `Read block 0 successfully` before loading two AMRN v4 cartridges.
- [x] F405 SWD evidence for kernel-owned DMA configuration and application-owned
  DMA denial. At the active diagnostic marker, DMA2 Stream 3 reported
  `CR=0x08025401`, `NDTR=128`, `PAR=0x40012C80`, and `M0AR=0x20000024`, while
  the target-visible trace reported `Application DMA request rejected` from
  both the kernel security boundary and the application. `DCOUNT=0` remains a
  storage-driver sequence observation and is not required for this DMA policy
  acceptance.

The first DMA-isolation hardware flow must use the release F405 target profile
and an SWD probe. Capture the board revision, firmware image hash, target
manifest revision, SD card and power details, SWD register state at DMA setup,
the SDIO read result, and the application-DMA rejection result. The expected
result is one successful kernel-owned SDIO block read, a kernel-only DMA
configuration snapshot, and no application DMA configuration path. This flow
is evidence for the current default-deny policy only; it does not establish
arbitrary peripheral or DMA-controller isolation.

### Watchdog and reset recovery

- [x] Host contract tests validate target timing metadata, explicit kernel
  ownership, single-arm behavior, and feed rejection before arming.
- [x] F405 IWDG backend compiles, captures reset flags during early bootstrap,
  clears the latch, and exposes the cause through the platform boundary.
- [x] F405 hardware watchdog timeout and reset-cause evidence: with the kernel
  heartbeat running, halting the CPU for more than the declared 2000 ms IWDG
  timeout caused a reset, and the next boot logged `Reset cause: Watchdog`
  before returning to the kernel heartbeat.
- [x] On 2026-08-23, the opt-in `watchdog-feed-failure-test` F405 profile
  blocked the first backend refresh after IWDG arm. The target reset, and the
  next boot logged `Reset cause: Watchdog`, entered Safe Mode, skipped
  application loading, and returned to the kernel heartbeat. This earlier
  run exposed the reset-loop risk that the follow-up fix addresses.
- [x] On 2026-08-23, the opt-in F405 profile blocked one intentional refresh,
  produced the watchdog reset and Safe Mode transition, then emitted repeated
  `(IWDG refreshed)` diagnostics without a second reset. The heartbeat remained
  stable on the USB CDC console.
- [x] Feed-failure policy is explicit: after a backend feed error, the kernel
  stops issuing further feeds and allows the armed hardware watchdog to reset
  the target.
- The STM32F4 HAL IWDG `feed()` operation has no failure result or status bit
  that can distinguish a rejected reload. The F405 test-only profile therefore
  blocks the refresh before the HAL call and verifies the resulting hardware
  timeout through the next boot's RCC reset flag. A backend-error acceptance
  item remains applicable only to a future target whose watchdog controller
  exposes a detectable feed failure.
- [x] F405 hardware Safe Mode evidence confirms a watchdog reset logged
  `Reset cause: Watchdog`, reported the recovery transition, skipped AMRN
  application loading, and returned to the kernel heartbeat without an
  application log. The distinct safe-mode heartbeat LED pattern remains a
  separate visual observation.
- [x] F405 hardware held the kernel after the invalid-PSP application reached
  `Terminated` for 30 seconds without a new watchdog reset or boot sequence.
  This confirms watchdog servicing remains active after application recovery.

The F405 feed-failure acceptance profile is opt-in and must never be used for
production firmware:

```text
cargo build -p dali-firmware --bin dali-f405 --release --no-default-features \
  --features stm32f405,usb-cdc,abi-context-switch,abi-relocation,abi-authentication,repository-loader,storage-write,watchdog-feed-failure-test \
  --target thumbv7em-none-eabihf
```

With this feature, the F405 watchdog backend rejects one kernel feed after the
IWDG is armed on a normal boot. The kernel drops that runtime feed path and
leaves the armed IWDG to expire. After the reset, the backend recognizes the
watchdog reset cause, does not inject another failure, and Safe Mode feeds the
IWDG from its recovery heartbeat. Acceptance requires the console to show the
reset, the next boot's `[BOOT] Reset cause: Watchdog`, Safe Mode recovery, and
a stable heartbeat without another reset. This verifies the F405 timeout path;
it is not a claim that the STM32F4 HAL exposes an observable hardware feed-error
status.

### Diagnostic evidence

- The kernel now retains a bounded fault capture in a linker-owned `NOLOAD`
  section across software reset. The next boot reports the fault kind,
  `EXC_RETURN`, validated kernel-frame address, stacked PC/LR, and SCB status
  registers before consuming the record. This is source and target-build
  evidence only until a real F405 reset reproduces the capture.
- The F405 write path uses the STM32F4 HAL CPU/FIFO implementation. The custom
  raw DMA write experiment was removed after release hardware testing showed
  repeated `SdioTransmitUnderrun` failures while the HAL path passed.
- A custom SDIO DMA write backend is deliberately deferred, not forgotten. It
  is not a production claim or a current test requirement; future work must
  justify it with a measurable large-transfer or realtime CPU-offload need and
  repeatable F405 hardware evidence before reintroducing it.
- On 2026-08-20, the release F405 HAL CPU/FIFO write path passed the complete
  trust-store write/read-back, signed AMRN verification, slot-1 load, and
  relocation-fixture execution sequence on the same physical card.
- On 2026-08-20, the write-path data-path reset and HAL-order change produced
  the expected `DCTRL` transition from `0x00000098` to `0x00000099`, removed
  the stale `CMDREND` flag, and still ended with `TXUNDERR`. A subsequent
  experiment using peripheral flow control and peripheral `INCR4` bursts
  produced `SdioDmaFailure(0x00400000)` (`FEIF3`). Neither result is a
  successful trust-store write; the DMA configuration remains under hardware
  investigation.
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
