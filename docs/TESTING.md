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

- [x] Application restart and rollback policy is explicit: termination enters
  the kernel recovery heartbeat, automatic restart is rejected, and rollback is
  unavailable while package storage is read-only.
- [ ] Hardware watchdog arming, feed ownership, timeout, and safe-mode reset;
  the watchdog remains intentionally disabled until its heartbeat contract is
  implemented.
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
  reject rollback on the read-only package boundary, and keep the watchdog
  disabled until a bounded heartbeat/feed owner exists.
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
- [x] Host-level PendSV preparation tests verify no-op behavior without a
  request and save-before-selection ordering. These tests do not prove the
  processor exception path.
- [x] Target-profile generation tests and embedded compilation consume the
  manifest-derived scheduler capacity; filesystem package limits are not used
  as scheduler capacity. Runtime interrupt ownership remains unimplemented.
- [x] Host-level scheduler-storage tests reject pre-initialization access,
  publish one initialized value, and reject repeated initialization. These
  tests do not prove interrupt masking on the target.
- [x] Target-profile generation carries the scheduler quantum as declared
  configuration; no timer frequency or quantum literal is embedded in the
  scheduler implementation.
- [x] Embedded target compilation verifies bootstrap scheduler initialization
  from target-profile configuration; timer enablement and interrupt register
  transfer remain disabled.
- [x] Host-level scheduler-record tests retain the manifest-owned slot beside
  each saved CPU context. This is metadata-binding evidence only; it does not
  prove PendSV register transfer or MPU region switching.
- [x] Embedded target compilation verifies the feature-gated SysTick exception
  hook and guarded PendSV request path. No timer is enabled and no register
  transfer occurs, so it is not hardware context-switch evidence.
- [x] Target ELF inspection verifies the feature-gated PendSV wrapper contains
  the raw `r4..r11` save, PSP/CONTROL/EXC_RETURN capture, scheduler helper call,
  and restore-primitive branch. This is target/source evidence only; hardware
  context switching and MPU switching remain unverified.
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
- [x] Hardware execution of the v4 streaming loader, selection, relocation, and
  successful application path is documented above; target compilation alone
  would not prove this behavior. v4-specific rejection/recovery hardware tests
  remain separate work.
- [ ] Runtime two-application memory isolation; loader placement and slot
  reservations are hardware-evidenced, but the runtime still enters only one
  application and does not yet switch contexts.
- [ ] PendSV/SysTick context switching and MPU region switching.
- [ ] DMA isolation.
- [x] Application restart and rollback policy is covered by the lifecycle policy
  contract; hardware watchdog implementation remains separate and pending.

## MVP acceptance test

The MVP passes only when a freshly flashed kernel discovers an `.amrn` package on the SD card, validates its 32-byte header and CRC32, loads it into the reserved SRAM region, transfers control to `unsafe extern "C" fn(*const ServiceTable) -> !`, produces the documented application LED pattern, and delivers the application log messages on hardware.

Build success, parser tests, or a simulated jump do not independently prove the end-to-end milestone.
