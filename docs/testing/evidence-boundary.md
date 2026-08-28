# Not yet evidenced

- [x] Application restart and rollback policy is explicit: termination enters
  the kernel recovery heartbeat, automatic restart is rejected, and rollback is
  unavailable while cartridge storage is read-only.
- [x] Watchdog arming and feed ownership are integrated behind the platform
  facade, with heartbeat and scheduler-tick feed paths target-checked; this is
  not hardware evidence.
- [x] Hardware watchdog timeout, reset-cause behavior, and the test-only
  feed-failure path were observed on F405 with a real IWDG reset; F405 also
  logged the Safe Mode transition and skipped application loading. Feed-error
  status evidence for other watchdog controllers remains pending.
- [ ] Alternative RWPI/PIC contract behavior; explicit relocation metadata is
  hardware-verified.
- [x] Host-level SRAM slot allocation, exact reservation, occupied-slot
  rejection, undeclared-slot rejection, release/reuse behavior, and range
  containment across independent code/data slots.
- [x] Host-level root-cartridge selection classification distinguishes no cartridge,
  exactly one cartridge, and ambiguous multiple cartridges without inferring
  identity from filenames.
- [x] Host-level AMRN v4 parsing validates non-zero cartridge identity, ABI v3
  image metadata, explicit slot metadata, and the extended cartridge checksum.
- [x] Host-level CLI tests cover v4 manifest parsing, cartridge identity/version
  validation, target slot IDs, and v4 inspection output.
- [x] Host-level pure catalog tests cover discovery-order-independent selection,
  duplicate identity rejection, slot mismatch, duplicate slot rejection, and
  externally occupied-slot rejection, including ordered selection across both
  declared slots. These tests are not hardware evidence.
- [x] Host-side AMRN v4 codec tests cover cartridge bytes, CRC mismatch, and
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
  reject rollback on the read-only cartridge boundary, and require a bounded
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
  manifest-derived scheduler capacity; filesystem cartridge limits are not used
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
  and `Running` after loading two cartridges; the kernel logged both slot
  boundaries and then executed the slot0 fixture.
- [x] F405 hardware verified the lifecycle fault path through `Faulted`,
  `Recovering`, and `Terminated` using the v4 invalid-PSP application fixture.
- [x] The relocation fixture manifest produces an AMRN format 4 cartridge with a
  non-zero identity, compatibility metadata, required service bitset, and the
  manifest-selected slot1; this artifact is ready for the hardware run.
- [x] The slot0 fixture produces a separate AMRN format 4 cartridge with a
  distinct identity, manifest-selected slot0, and 49 retained relocation
  records. Both cartridge artifacts are ready for the two-cartridge hardware run.
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
  `0x2F4`. Slot-specific MPU region switching is verified separately below;
  this evidence does not by itself prove cross-slot memory isolation.
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
  a slot1-to-slot0 read, but not DMA isolation or complete application policy.
- [x] F405 hardware ran the two-cartridge fault-recovery fixture with
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
- [x] Bounded F405 SDIO DMA ownership and application-DMA denial.
- [ ] Arbitrary DMA-controller and peripheral isolation.
- [x] Application restart and rollback policy is covered by the lifecycle policy
  contract; F405 hardware watchdog timeout and Safe Mode recovery evidence are
  recorded above. Automatic restart, rollback, and interrupted-write recovery
  remain outside this acceptance result.
