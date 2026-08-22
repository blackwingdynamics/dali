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
passes. The third state requires a deliberately interrupted prepared journal
write on the disposable card and remains a physical interruption test.

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
- [x] F405 hardware regression after kernel-owned slot reservation was
  integrated: the slot1 package again passed AMRN validation and emitted
  `[INFO][APP] Relocation fixture`. This verifies that reservation state does
  not change the established single-application relocation path.

### AMRN v4 hardware evidence

- [x] F405 hardware accepted the manifest-backed AMRN format 4 relocation
  package: the console reported AMRN validation followed by
  `[INFO][APP] Relocation fixture`.
- [x] The observed package contained identity metadata, `slot_id = 1`, and 49
  relocation records; the application executed from the selected non-zero slot.
- [x] The probe shutdown warning after flashing was classified as a transport
  teardown event after the target continued running, not as a loader failure.
- [x] F405 hardware loaded two distinct AMRN v4 packages in one boot: the
  loader reported `Loaded 2 application package(s) into declared slots`, and
  deterministic slot selection entered the slot0 fixture. A later CDC console
  attach showed no replay because prior records had already drained.
- [x] F405 hardware rejected two real packages carrying the same identity with
  `PackageCatalog(DuplicateIdentity)` and entered the kernel heartbeat without
  executing either application.
- [x] F405 hardware rejected two real packages with different identities that
  claimed the same slot with `PackageCatalog(SlotOccupied)` and entered the
  kernel heartbeat without executing either application.
- [x] F405 hardware reported both loaded packages in their manifest-owned
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
  signed AMRN package launched; no application restart was observed before the
  test ended. This records application lifecycle recovery, not the separate
  watchdog-duration proof after termination.

### DMA isolation

- [x] Host contract tests validate kernel DMA-buffer range, alignment, empty
  range, and overflow rejection.
- [x] The F405 SDIO path validates its manifest-declared DMA buffer before
  programming DMA2 and retains exclusive mutable ownership for the transfer.
- [x] F405 hardware evidence that the guarded SDIO path remains operational
  after the ownership check: firmware reported `SDIO card initialized` and
  `Read block 0 successfully` before loading two AMRN v4 packages.
- [ ] Hardware evidence for unauthorized DMA configuration or application-owned
  DMA. The current ABI exposes no application DMA service, so general DMA
  isolation is not claimed yet.

### Watchdog and reset recovery

- [x] Host contract tests validate target timing metadata, explicit kernel
  ownership, single-arm behavior, and feed rejection before arming.
- [x] F405 IWDG backend compiles, captures reset flags during early bootstrap,
  clears the latch, and exposes the cause through the platform boundary.
- [x] F405 hardware watchdog timeout and reset-cause evidence: with the kernel
  heartbeat running, halting the CPU for more than the declared 2000 ms IWDG
  timeout caused a reset, and the next boot logged `Reset cause: Watchdog`
  before returning to the kernel heartbeat.
- [x] Feed-failure policy is explicit: after a backend feed error, the kernel
  stops issuing further feeds and allows the armed hardware watchdog to reset
  the target.
- F405 backend feed-failure injection is not an observable hardware contract:
  the STM32F4 HAL IWDG `feed()` operation has no failure result or status bit
  that can distinguish a rejected reload. The real F405 safety evidence is the
  stronger failure mode already recorded above: when kernel servicing stops,
  the armed IWDG resets the target and the next boot reports `Watchdog`.
  A backend-error acceptance item remains applicable only to a future target
  whose watchdog controller exposes a detectable feed failure.
- [x] F405 hardware Safe Mode evidence confirms a watchdog reset logged
  `Reset cause: Watchdog`, reported the recovery transition, skipped AMRN
  application loading, and returned to the kernel heartbeat without an
  application log. The distinct safe-mode heartbeat LED pattern remains a
  separate visual observation.
- [x] F405 hardware held the kernel after the invalid-PSP application reached
  `Terminated` for 30 seconds without a new watchdog reset or boot sequence.
  This confirms watchdog servicing remains active after application recovery.

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

## Not yet evidenced

- [x] Application restart and rollback policy is explicit: termination enters
  the kernel recovery heartbeat, automatic restart is rejected, and rollback is
  unavailable while package storage is read-only.
- [x] Watchdog arming and feed ownership are integrated behind the platform
  facade, with heartbeat and scheduler-tick feed paths target-checked; this is
  not hardware evidence.
- [x] Hardware watchdog timeout and reset-cause behavior was observed on F405
  with a real IWDG reset; F405 also logged the Safe Mode transition and skipped
  application loading. Feed-failure hardware evidence remains pending.
- [ ] Alternative RWPI/PIC contract behavior; explicit relocation metadata is
  hardware-verified.
- [x] Host-level SRAM slot allocation, exact reservation, occupied-slot
  rejection, undeclared-slot rejection, release/reuse behavior, and range
  containment across independent code/data slots.
- [x] Host-level root-package selection classification distinguishes no package,
  exactly one package, and ambiguous multiple packages without inferring
  identity from filenames.
- [x] Host-level AMRN v4 parsing validates non-zero package identity, ABI v3
  image metadata, explicit slot metadata, and the extended package checksum.
- [x] Host-level CLI tests cover v4 manifest parsing, package identity/version
  validation, target slot IDs, and v4 inspection output.
- [x] Host-level pure catalog tests cover discovery-order-independent selection,
  duplicate identity rejection, slot mismatch, duplicate slot rejection, and
  externally occupied-slot rejection, including ordered selection across both
  declared slots. These tests are not hardware evidence.
- [x] Host-side AMRN v4 codec tests cover package bytes, CRC mismatch, and
  invalid relocation; the loader contract test covers undeclared-slot
  rejection. These tests are not hardware evidence.
- [x] Host-level application lifecycle tests cover ordered discovery, loading,
  readiness, running, fault, recovery, and terminal states; invalid skips,
  slot mismatches, and implicit restart are rejected. These tests are
  hardware-neutral contract evidence, not isolation or scheduler evidence.
- [x] Host-level active-context ownership tests reject activation before
  readiness, reject a second active context, and allow retirement only after
  terminal recovery. These tests do not prove runtime scheduling or isolation.
- [x] Host-level lifecycle policy tests require manual reset after termination,
  reject rollback on the read-only package boundary, and require a bounded
  heartbeat/feed owner before watchdog arming.
- [x] Host-level context-switch contract tests preserve the PSP, `r4..r11`,
  `CONTROL`, and `EXC_RETURN` record, enforce one running context, bound table
  capacity, exclude terminated contexts, and select ready contexts in order.
  These tests do not prove PendSV, SysTick, or MPU hardware behavior.
- [x] Host-level tick-budget tests reject a zero quantum, emit one PendSV
  request per elapsed quantum, clear requests after consumption, and restart
  tick accounting at the next quantum. These tests do not prove timer timing
  or interrupt latency.
- [x] Embedded target compilation verifies the explicitly disabled
  `abi-context-switch` ARM save/restore primitives and their `SavedContext`
  layout. This is target/source evidence only; no PendSV interrupt or context
  switch is enabled by this step.
- [x] Host-level scheduler tests verify bounded quantum rejection, one-shot
  preemption consumption, outgoing-state capture, and next-ready selection.
  These tests do not prove interrupt ownership, register transfer, or MPU
  switching.
- [x] Host-level scheduler tests verify that fault recovery retires the active
  context before selecting the next ready context and never resumes a
  terminated context. This does not prove the ARM exception return or MPU
  reprogramming.
- [x] Host-level PendSV preparation tests verify no-op behavior without a
  request and save-before-selection ordering. These tests do not prove the
  processor exception path.
- [x] Target-profile generation tests and embedded compilation consume the
  manifest-derived scheduler capacity; filesystem package limits are not used
  as scheduler capacity. Runtime interrupt ownership remains unimplemented.
- [x] Host-level scheduler-storage tests reject pre-initialization access,
  publish one initialized value, and reject repeated initialization. These
  tests do not prove interrupt masking on the target.
- [x] Target-profile generation carries the scheduler quantum and timer
  frequency as declared configuration; no timing literal is embedded in the
  scheduler implementation.
- [x] Embedded target compilation verifies bootstrap scheduler initialization
  and delayed SysTick enablement from target-profile configuration. This does
  not prove interrupt delivery or context switching on hardware.
- [x] Host-level scheduler-record tests retain the manifest-owned slot beside
  each saved CPU context. This is metadata-binding evidence only; it does not
  prove PendSV register transfer or MPU region switching.
- [x] Embedded target compilation verifies the feature-gated SysTick exception
  hook and guarded PendSV request path. No register transfer is proven, so it
  is not hardware context-switch evidence.
- [x] Target ELF inspection verifies the feature-gated PendSV wrapper contains
  the raw `r4..r11` save, PSP/CONTROL/EXC_RETURN capture, scheduler helper call,
  and restore-primitive branch. This is target/source evidence for the wrapper;
  repeated CPU context switching and MPU switching are separately
  hardware-verified below.
- [x] F405 hardware verified v4 lifecycle activation through `Loaded`, `Ready`,
  and `Running` after loading two packages; the kernel logged both slot
  boundaries and then executed the slot0 fixture.
- [x] F405 hardware verified the lifecycle fault path through `Faulted`,
  `Recovering`, and `Terminated` using the v4 invalid-PSP application fixture.
- [x] The relocation fixture manifest produces an AMRN format 4 package with a
  non-zero identity, compatibility metadata, required service bitset, and the
  manifest-selected slot1; this artifact is ready for the hardware run.
- [x] The slot0 fixture produces a separate AMRN format 4 package with a
  distinct identity, manifest-selected slot0, and 49 retained relocation
  records. Both package artifacts are ready for the two-package hardware run.
- [x] F405 hardware accepted the first scheduler handoff with the
  `abi-context-switch,abi-relocation` kernel and the two v4 fixtures: the
  boot log entered `Slot 0 fixture` and then `Relocation fixture` after loading
  both declared slots. This proves initial slot0 execution and one slot0 to
  slot1 application handoff; it does not prove return switching, repeated
  preemption, register preservation, or memory-isolation behavior.
- [x] The two hardware fixtures expose independent volatile progress markers
  at the start of their declared data images. Target symbol inspection places
  them at the expected linked data offset; this prepares repeatable GDB
  observation but is not hardware context-switch evidence by itself.
- [x] Hardware execution of the v4 streaming loader, selection, relocation, and
  successful application path is documented above; target compilation alone
  would not prove this behavior. v4-specific rejection/recovery hardware tests
  remain separate work.
- [x] F405 GDB hardware evidence verified repeated PendSV CPU switching: PSP
  alternated between slot0 (`0x2000CFB8`) and slot1 (`0x20014FB8`) stack
  ranges, while slot0 progress reached `0x26A` and slot1 progress reached
  `0x2F4`. This proves repeated CPU context execution, but not MPU region
  switching or cross-slot memory isolation.
- [x] F405 GDB hardware evidence verified MPU region switching at
  `restore_selected`: slot0 used code/data bases
  `0x20008000`/`0x2000C000`, and slot1 used `0x20010000`/`0x20014000`.
  Code RASR was `0x0603001B` and data RASR was `0x1303001B` for both slots.
- [x] F405 hardware verified bidirectional CPU-side application-memory
  isolation. The slot1-to-slot0 read was rejected at `0x20008000`, and the
  reverse slot0-to-slot1 fixture was rejected while slot1 resumed.
- [x] F405 hardware ran the manifest-derived slot1 cross-slot fixture. The
  application read slot0 code origin `0x20008000` and the kernel reported
  `MemManage status=0x00000082 address=Some(536903680)`, followed by
  `Faulted`, `Recovering`, and `Terminated`. This proves CPU MPU rejection of
  a slot1-to-slot0 read, but not DMA isolation or faulted-context scheduling.
- [x] F405 hardware ran the two-package fault-recovery fixture with
  `abi-context-switch,abi-relocation`. GDB observed the slot1 entry breakpoint
  once, stopped in `recover_faulted_context`, and then observed slot0 progress
  increase from `0x0000024C` to `0x01289A9C`, `0x017BB673`, and `0x01ADDDF1`
  without another slot1 entry. This proves faulted-context exclusion and
  continued slot0 execution after recovery; complete application isolation
  and DMA isolation remain separate claims.
- [x] F405 hardware ran the reverse slot0-to-slot1 fixture with the relocation
  fixture in slot1. GDB observed slot0 entry once, then slot1 execution at
  `0x200100A0`; the slot1 progress marker increased from `0x00000000` to
  `0x0048D887` without a second slot0 entry. This proves the reverse CPU-side
  rejection and recovery direction, not DMA isolation.
- [ ] DMA isolation.
- [x] Application restart and rollback policy is covered by the lifecycle policy
  contract; F405 hardware watchdog timeout and Safe Mode recovery evidence are
  recorded above. Automatic restart, rollback, and interrupted-write recovery
  remain outside this acceptance result.

## Signed package acceptance

### Repository loader hardware acceptance status (2026-08-22)

Repository streaming watchdog progress is injected through the target
platform boundary. The FAT repository adapter invokes the bounded progress
hook only after a non-empty chunk has been accepted by the parser, hash, or
signature consumer. Storage or verification failures therefore do not feed
the watchdog. The repository loader remains board-agnostic; the platform
supplies the hook that maps valid chunk progress to the hardware watchdog
service operation.

The cryptographic replay path additionally divides each delivered body chunk
into bounded 64-byte verification units and invokes the same progress hook
after each unit. This prevents a single Ed25519/SHA-256 update from exceeding
the declared watchdog window while preserving the rule that only successfully
processed bytes can advance the watchdog.

#### Binary v2 host and linker evidence

The Binary v2 CLI dispatch and metadata codec were checked with:

```text
cargo test -p dali-cli -p dali-metadata
51 dali-cli tests passed
54 dali-metadata tests passed
cargo check -p dali-kernel --no-default-features --features board-stm32f405-sd,usb-cdc,abi-relocation,repository-loader,storage-write --target thumbv7em-none-eabihf
passed
```

The release image was rebuilt with the repository-loader feature set and
`CARGO_PROFILE_RELEASE_DEBUG=2`. The exact build configuration was:

```text
CARGO_PROFILE_RELEASE_DEBUG=2 cargo build -p dali-kernel --release --no-default-features \
  --features board-stm32f405-sd,usb-cdc,abi-context-switch,abi-relocation,abi-authentication,repository-loader,storage-write \
  --target thumbv7em-none-eabihf
```

The generated linker map, target LLVM size report, and symbols show:

```text
.dma_buffer           0x1200 =  4608 bytes @ 0x20000000
.fault_capture          0x44 =    68 bytes @ 0x10000000
.repository_workspace 0x7640 = 30272 bytes @ 0x20018000
.data                 0x080c =  2060 bytes @ 0x10000080
.bss                  0x1748 =  5960 bytes @ 0x1000088c
_stack_end                         0x10001fd4
_stack_start                       0x10010000
available CCM stack               0x0000e02c = 57388 bytes
runtime region remaining          0x000009c0 =  2496 bytes
```

The repository workspace is now a target-profile-owned, board-agnostic BSS
section in the declared runtime region. It contains the shared stream chunk,
AMRN pass state, and typed metadata outputs; it is not placed in CCM, so it
does not consume the privileged kernel stack budget. The previous repository
loader frame overflow is no longer present in this build. Static prologue
inspection of the release ELF measured these largest repository-path local
allocations:

```text
load_repository_package       0x251c = 9500 bytes
replay_targets                 0xdfc = 3580 bytes
capture_role_into              0xce4 = 3300 bytes
parse_targets_first_pass       0xcdc = 3292 bytes
verify_packages_into            0xc74 = 3188 bytes
load_file_with_key              0xa3c = 2620 bytes
verify_timestamp_and_snapshot  0x83c = 2108 bytes
```

These are static function-prologue measurements, not a proof of the maximum
whole-program call depth. The 32 KiB runtime-region headroom is also distinct
from the CCM stack headroom: the former is workspace capacity, while the
latter is the kernel's privileged stack space.

`cargo-bloat` is not installed in the validation environment. The measurements
above were obtained with the target LLVM `llvm-size`, `llvm-nm`, and
`llvm-objdump`; no network installation was attempted. This evidence does not
claim F405 hardware acceptance.

The feature-gated board-agnostic repository loader compiles for
`thumbv7em-none-eabihf`, and its host-visible boundary tests pass. A real F405
acceptance run on 2026-08-21 used the release-anchor bundle prepared by
`scripts/prepare-f405-binary-v2-sd.sh` and produced:

```text
[STORAGE] SDIO card initialized
[STORAGE] Trust-store artifact write/read-back test passed
[STORAGE] Read block 0 successfully
[LOADER] AMRN header and payload validated
[SECURITY] AMRN signature verified
[LOADER] Loaded 1 application package(s) into declared slots
[LOADER] Slot 1 (slot1) boundaries: code=0x20010000+16384 data=0x20014000+16384 psp_top=0x20015010
[SECURITY] Application lifecycle: Ready
[SECURITY] Active application context: Running
[APP] Relocation fixture
```

This is hardware evidence for the configured F405 release-anchor path:
SDIO access, Binary v2 repository traversal, AMRN validation, signature
verification, slot loading, and relocation-fixture execution. It does not
prove production key custody, Secure Boot, DMA isolation, or power-loss
recovery. The remaining repository acceptance scenarios are candidate
write/read-back with atomic activation, revoked-developer-key rejection on
target, and interrupted-write/power-loss recovery.

#### Final F405 Binary v2 acceptance record — 2026-08-22

The kernel was rebuilt with the release repository-loader feature set and
flashed to the WeAct Studio STM32F405RGT6 through the Raspberry Pi Pico 2
CMSIS-DAP probe. The SD card contained the freshly generated release-anchor
Binary v2 bundle. The package digest was:

```text
53bf7b2bd3af6b230802fc71f5ca006b67a980e3e4587c0973be140386af3d04
```

The observed F405 output at `2026-08-22 12:27:00` was:

```text
[STORAGE] SDIO card initialized
[STORAGE] Trust-store artifact write/read-back test passed
[STORAGE] Read block 0 successfully
[LOADER] AMRN header and payload validated
[SECURITY] AMRN signature verified
[LOADER] Loaded 1 application package(s) into declared slots
[LOADER] Slot 1 (slot1) boundaries: code=0x20010000+16384 data=0x20014000+16384 psp_top=0x20015010
[SECURITY] Application lifecycle: Ready
[SECURITY] Active application context: Running
[APP] Relocation fixture
```

This closes the normal Binary v2 F405 acceptance path for the recorded build:
SDIO initialization, trust-store access, repository traversal, signed AMRN
verification, slot loading, relocation, and `Ready -> Running` application
execution. The package file size, board revision, and power source were not
captured in this run and remain unspecified evidence fields. The separate
watchdog reset/Safe Mode scenario remains a distinct test.

- [x] Host AMRN tests cover v5 header/trailer split parsing and signed-range
  boundary validation.
- [x] The feature-gated kernel target build covers v5 streaming signature,
  CRC32, relocation, and target-profile trust-anchor checks before SRAM copy.
- [x] The loader service-capability policy accepts the declared Log service and
  rejects undeclared required-service bits in host/unit coverage.
- [x] A real v5 relocation package is generated and inspected with the
  development test key; this is host/package evidence, not hardware evidence.
- The F405 signed-loader hardware image must be built with Cargo's `release`
  profile; the feature-complete development link does not fit the board's
  documented flash region.
- [x] F405 hardware: provision the documented development test public key and
  execute a valid signed v5 package; the kernel verified the signature before
  loading slot 1 and the relocation fixture ran.
- [x] F405 hardware: provision the generated release public trust anchor and
  execute a release-profile signed v5 package; the kernel logged
  `AMRN signature verified`, loaded one package into slot 1, and ran the
  relocation fixture. This verifies the configured release trust-anchor path
  on the development board; production key custody, rotation, and Secure Boot
  remain separate acceptance requirements.
- [x] F405 hardware: reject an unknown key ID before SRAM copy; the loader
  returned `UnknownTrustAnchor` and entered the kernel heartbeat.
- [x] F405 hardware: reject a modified signed payload; the loader returned
  `V5SignedPackage(CrcMismatch)` and entered the kernel heartbeat.
- [x] F405 hardware: reject a truncated DSIG trailer; the loader returned
  `V5SignedPackage(InvalidSignature)` and entered the kernel heartbeat.
- [ ] Secure Boot and kernel-image authenticity.
- [x] F405 hardware: run the feature-gated trust-store artifact
  write/flush/read-back acceptance path on a disposable FAT32 card and record
  the console result. Candidate durability is verified before commit-marker
  durability; power-loss recovery remains separate.

## MVP acceptance test

The MVP passes only when a freshly flashed kernel discovers an `.amrn` package on the SD card, validates its 32-byte header and CRC32, loads it into the reserved SRAM region, transfers control to `unsafe extern "C" fn(*const ServiceTable) -> !`, produces the documented application LED pattern, and delivers the application log messages on hardware.

Build success, parser tests, or a simulated jump do not independently prove the end-to-end milestone.
