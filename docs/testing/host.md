# Host tests

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

Storage lifecycle host coverage includes initialization, ready operations,
card-removal classification, fault transitions, and bounded reinitialization:

```text
cargo test -p dali-kernel --lib --no-default-features \
  --features board-stm32f405-sd,usb-cdc,abi-context-switch,abi-relocation,abi-authentication,repository-loader,storage-write
```

The Phase 1 bounded-timeout host coverage is hardware-neutral contract
coverage. It verifies that a zero timer timeout is rejected without starting
the timer, a stalled SPI transfer returns a typed bounded error, a timeout
branch returns DriverError::Timeout, and ownership can be released after the
failed transfer. These tests do not prove F405 peripheral timing or external
device recovery:

```text
cargo test -p dali-driver-api --test driver_contracts \
  timer_timeout_and_spi_stall_paths_remain_bounded_and_recoverable
```

## Phase 1 F405 acceptance record

The remaining target acceptance requires a freshly built F405 firmware and a
real console capture. Keep the two scenarios separate:

- [x] Timer integration: the Silicon Trace recorded
  `[DRIVER][TIMER] Hardware timer tick elapsed`.
- [x] SPI bounded transfer: the Silicon Trace recorded
  `[DRIVER][SPI] Bounded transfer completed`.
- [x] Trust-store write/read: the Silicon Trace recorded
  `[STORAGE] Trust-store artifact write/flush/read-back test passed`.
- [x] AMRN signature verification: the Silicon Trace recorded
  `[SECURITY] AMRN signature verified`.
- [x] Timer hardware timeout probe: the Silicon Trace recorded
  `[DRIVER][TIMER] Hardware timer timeout enforced`.
- [x] Stalled SPI recovery: the Silicon Trace recorded
  `[DRIVER][SPI] Stalled transfer timeout enforced` and
  `[DRIVER][SPI] Recovery transfer completed`; the probe then deselected the
  device and released bus ownership before continuing boot.
- [x] Record board revision, wiring, power source, firmware revision, logging
  transport, expected trace, observed trace, result, and limitations for the
  Phase 1 run below. Unknown values are explicitly recorded as `Not recorded`.

### F405 bounded-timeout acceptance run — 2026-08-25

The release acceptance firmware from kernel revision `619c84d`, built with
the `driver-hardware-test` feature, was flashed and verified on the WeAct
Studio STM32F405RGT6 Core Board through a Raspberry Pi Pico 2 CMSIS-DAP probe.
The F405 USB CDC console was the selected host CDC port; a second port was not used.
Board revision, exact SWD wiring, and power source were not recorded in this
run.

The expected trace included the existing timer and SPI success markers, a
timer timeout result, and a stalled-SPI recovery sequence showing the stalled
condition, timeout result, recovery transfer, and ownership release.

The observed trace from the completed run was:

```text
[DRIVER][UART] Bounded timeout enforced
[DRIVER][SPI] Stalled transfer timeout enforced
[DRIVER][SPI] Recovery transfer completed
[STORAGE] Trust-store artifact write/flush/read-back test passed
[SECURITY] AMRN signature verified
[SECURITY] Application lifecycle: Ready
[SECURITY] Active application context: Running
[DRIVER][TIMER] Hardware timer timeout enforced
[DRIVER][TIMER] Hardware timer tick elapsed
[APP] DMA application request rejected
```

Validation layers:

- Host: driver contract coverage for bounded timer and SPI recovery.
- Embedded: release F405 target build with `driver-hardware-test`.
- Flashing: verified through the Raspberry Pi Pico 2 CMSIS-DAP probe.
- Silicon Trace: the console output listed above from the real F405 target.

This run confirms the UART bounded-timeout marker, stalled SPI timeout,
recovery transfer, timer timeout, timer integration, storage and signed-loader
path, application lifecycle, and DMA denial. The probe's bounded recovery path
released SPI device and bus ownership before the kernel continued boot.

Result: Partial record; the timer-timeout and stalled-SPI recovery acceptance
items passed. Board revision, exact wiring, and power source are explicitly
recorded as `Not recorded` below.

```text
Board revision: v1.1
Wiring: SWDIO, SWCLK, and GND connected; exact peripheral wiring not recorded
Firmware revision: 619c84d
Power source: USB
Transport: USB CDC console on the selected host port; Pico 2 CMSIS-DAP for flashing
Expected trace: Timer timeout; stalled SPI timeout; recovery transfer; ownership release
Observed trace: All expected timer and SPI markers listed above
Limitations: No independent logic-analyzer or oscilloscope capture
```

The timer timeout and stalled-recovery markers close the two remaining Phase 1
functional acceptance items. The metadata fields are recorded above; values
that were unavailable in the historical capture remain explicitly marked
`Not recorded`.

### Phase 2 F405 I2C acceptance record — 2026-08-25

Phase 2 hardware acceptance was run on the WeAct Studio STM32F405RGT6 Core
Board using the release acceptance firmware and a real Raspberry Pi Pico 2
CMSIS-DAP probe. The active CDC console was the selected host port; the device list
reported it as `available`. The probe identifier was not retained in this public
record. The firmware source revision used for this
capture was not recorded separately from the flashed acceptance image. Board
revision, exact wiring, and power source were also not recorded.

The expected I2C acceptance result was a bounded transfer timeout followed by
peripheral bus recovery, while the existing boot, storage, security, and
lifecycle markers continued normally. The observed I2C trace was:

```text
[DRIVER][I2C] Bounded transfer timeout enforced
[DRIVER][I2C] Bus recovered
```

The complete relevant console output also recorded:

```text
[DRIVER][UART] Bounded timeout enforced
[DRIVER][SPI] Stalled transfer timeout enforced
[DRIVER][SPI] Recovery transfer completed
[DRIVER][I2C] Bounded transfer timeout enforced
[DRIVER][I2C] Bus recovered
[STORAGE] Trust-store artifact write/flush/read-back test passed
[SECURITY] AMRN signature verified
[SECURITY] Application lifecycle: Ready
[SECURITY] Active application context: Running
[DRIVER][TIMER] Hardware timer timeout enforced
[DRIVER][TIMER] Hardware timer tick elapsed
[APP] DMA application request rejected
```

Validation layers:

- Host: I2C ownership, repeated-start, timeout, and bus-error contract tests.
- Embedded: release F405 target build with the I2C backend enabled.
- Flashing: acceptance image downloaded through the Raspberry Pi Pico 2
  CMSIS-DAP probe.
- Silicon Trace: the console output listed above from the real F405 target.

Result: the F405 I2C bounded-timeout and bus-recovery acceptance is confirmed
by real Silicon Trace. Repeated-start semantics remain covered by the
hardware-neutral contract and host tests; this trace does not claim an
external OLED device rendering result. Phase 2 is closed. Missing board
revision, exact wiring, power source, and separately recorded firmware
revision remain evidence metadata limitations, not failed I2C behavior.

```text
Board revision: v1.1
Wiring: SWDIO, SWCLK, GND, I2C1 PB6=SCL, PB7=SDA, VCC, and GND connected
Firmware revision: Not recorded separately from the flashed acceptance image
Power source: USB; I2C device powered at 3.3 V
Transport: USB CDC console on the selected host port; Pico 2 CMSIS-DAP for flashing
Expected trace: Bounded I2C timeout followed by bus recovery
Observed trace: Bounded transfer timeout enforced; Bus recovered
Limitations: No external-device ACK or OLED rendering was observed in this run
```

### Phase 3 F405 OLED and diagnostics-console acceptance — 2026-08-25

Phase 3 hardware setup used the WeAct Studio STM32F405RGT6 Core Board with the
manifest-configured SSD1306 I2C OLED profile. The OLED was connected through
the board-owned I2C1 bus on PB6/PB7 and powered at 3.3 V. The acceptance
firmware retained one I2C1 singleton shared between the OLED and the optional
driver probe.

The expected result was successful bounded OLED initialization and rendering
of the boot diagnostics console, with deterministic text-grid updates. A
missing or non-responsive display was required to select headless operation
without blocking kernel boot. The observed hardware result was negative: the
OLED did not illuminate and no text was visible. The boot console continued,
but that is not display-rendering evidence.

- [x] Manifest-driven SSD1306 profile generated and consumed by the F405 backend.
- [x] One I2C1 singleton shared between the OLED backend and acceptance probe.
- [ ] Bounded diagnostics console rendered on the physical F405 OLED.
- [ ] Display-unavailable path selected headless operation without freezing boot.
- [x] Deterministic clipping, cursor tracking, scrolling, and overflow behavior
  covered by 21 passing hardware-neutral driver tests.

Validation layers:

- Host: display contract and diagnostics-console tests.
- Embedded: release F405 target build with the OLED and hardware-test features.
- Flashing: acceptance image downloaded to the F405 target.
- Silicon Trace: boot console output was captured; raw OLED frame evidence was
  not archived.

Result: Unverified. The OLED did not illuminate and no text rendering was
observed. Board revision and wiring are now recorded; firmware revision and a
raw OLED/electrical trace remain unavailable.

```text
Board revision: v1.1
Wiring: SWDIO, SWCLK, GND, I2C1 PB6=SCL, PB7=SDA, VCC, and GND connected
Firmware revision: Not recorded
Power source: USB; OLED powered at 3.3 V
Transport: USB CDC console; physical OLED panel observation
Expected trace: Bounded OLED initialization and boot diagnostics rendering
Observed trace: Boot diagnostics trace; OLED did not illuminate and no text was visible
Limitations: No raw OLED frame capture or independent electrical trace archived
```

The F405 backend maps bounded SDIO command/data timeouts to `CardRemoved`
because this board exposes no card-detect GPIO. The recovery heartbeat retains
the reader after a failed bring-up, probes at the configured interval, and
performs bounded reinitialization when the card responds again. On 2026-08-23,
F405 hardware recorded the complete trace:

```text
[STORAGE] Card removed; entering safe recovery loop
[STORAGE] Recovery heartbeat active
[STORAGE] Reinitialization probe started
[STORAGE] Card reinitialized; state Ready
```

The feature-gated `storage-write` target build contains a real storage
acceptance path. On an initialized F405 SDIO card it writes each of the three
reserved trust-store artifact names (`DALI-ACT.BIN`, `DALI-CAN.BIN`, and
`DALI-CMT.BIN`) through the FAT filesystem boundary, flushes the underlying
card, and reads the artifact back before continuing. The candidate sequence
completes before the commit-marker sequence begins. The path is destructive to
those reserved files, so it is for a disposable test card only. A successful
build is not hardware evidence; the console must report:

```text
[STORAGE] Trust-store artifact write/flush/read-back test passed
```

The F405 hardware acceptance record for this path is dated
`2026-08-22 13:16:45`, uses kernel revision `baa9a47`, and records package
digest `53bf7b2bd3af6b230802fc71f5ca006b67a980e3e4587c0973be140386af3d04`.
This proves the bounded flush/read-back ordering on the real SDIO card. It is
not power-loss or interrupted-write recovery evidence.

With the reboot recovery wiring enabled, the boot log additionally reports one
of these states before the acceptance write sequence:

```text
[RECOVERY] No durable commit journal found; using provisioned state
[RECOVERY] Committed generation selected: version=1 sequence=2 slot=B
[RECOVERY] Prepared commit discarded; previous active state retained
```

The first two states are suitable for the normal and post-reset acceptance
passes. The interruption profile below provides the third state without
depending on a timing window.

The manual F405 run at `2026-08-22 14:37:05` observed the committed-recovery
state (`version=1 sequence=2 slot=B`) followed by successful trust-store
flush/read-back, signed AMRN verification, slot loading, relocation, and
`Ready -> Running` execution. This closes the manual committed-journal reboot
check; Prepared interruption and power-loss recovery remain separate tests.

The `storage-interruption-test` profile provides a bounded two-boot Prepared
state acceptance procedure on a disposable card. On the first boot it replaces
any existing test journal with two valid `Prepared` records in `DALI-CMT.BIN`,
flushes them, verifies the read-back, logs the staged fixture, and stops before
application loading. Reset or power-cycle the board, then run the normal
acceptance console capture. The second boot must log `Prepared commit
discarded; previous active state retained`, after which the normal artifact and
package acceptance may proceed.
This proves durable Prepared-state discard; it does not by itself prove that a
power cut occurred during a physical sector write.

The F405 two-boot reset acceptance run at `2026-08-22 15:01:38` and
`2026-08-22 15:02:05` observed the staged Prepared journal, rebooted the target,
discarded the Prepared state, and then completed trust-store flush/read-back,
signed AMRN verification, slot loading, relocation, and `Ready -> Running`.
This closes the bounded reset-recovery test. Physical power-loss during a
sector write remains unverified.

The feature-gated `storage-rollback-test` profile writes a valid committed
generation `version=2` to `DALI-CMT.BIN` on a disposable F405 card. A subsequent
production boot must stream and authenticate the existing signed bundle
manifest, observe its older `version=1`, and reject it before package loading.
The F405 run at `2026-08-22 23:31:09` observed:

```text
[RECOVERY] Committed generation selected: version=2 sequence=2 slot=B
[LOADER] Binary v2 repository verification failed: bundle-rollback
[SECURITY] Rejection: package generation older than committed generation
```

The follow-up production boot restored the card to the committed `version=1`
state and completed signed package verification, slot loading, and
`Ready -> Running`.

Build the rollback fixture explicitly:

```text
cargo build -p dali-kernel --release --no-default-features \
  --features board-stm32f405-sd,usb-cdc,abi-context-switch,abi-relocation,abi-authentication,repository-loader,storage-write,storage-rollback-test \
  --target thumbv7em-none-eabihf
```

Build the interruption profile explicitly:

```text
cargo build -p dali-kernel --release --no-default-features \
  --features board-stm32f405-sd,usb-cdc,abi-context-switch,abi-relocation,abi-authentication,repository-loader,storage-write,storage-interruption-test \
  --target thumbv7em-none-eabihf
```

Build this mode explicitly; it is not part of the default kernel profile:

```text
cargo build -p dali-kernel --no-default-features \
  --features board-stm32f405-sd,usb-cdc,abi-relocation,storage-write \
  --target thumbv7em-none-eabihf
```

The DMA contract has hardware-neutral tests for aligned in-range ranges,
out-of-range rejection, alignment and empty-range rejection, and arithmetic
overflow. These tests validate the range policy only; they are not evidence
that a DMA controller, peripheral, or application can access memory safely.

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
