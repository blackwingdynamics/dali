# 00 — Kernel Foundation and Universal Port Architecture

Status: **Highest priority — active on `kernel-foundation`**.

This roadmap is the primary execution plan for stabilizing the Dali OS kernel
as a hardware-neutral embedded operating system. It intentionally narrows the
project to the kernel, its portable contracts, and the ports required to run
it on real boards.

The target is not one firmware image that runs on every CPU. The target is one
portable kernel core with explicit architecture and board ports, so a new CPU
or board can be added without modifying existing kernel policy code.

## Scope

This roadmap covers:

- the hardware-neutral kernel core;
- CPU architecture contracts and ports;
- board and platform backend contracts;
- bounded boot, interrupt, timer, watchdog, memory, and lifecycle policy;
- hardware-neutral driver contracts;
- the existing STM32F405 port as the first regression target;
- one later independent backend as proof that the boundary is real.

The existing F405 MVP behavior is preserved while this work proceeds. The
parked branch `parked/f405-platform-state-2026-09-14` is the historical
baseline and must not be used as an active development branch.

## Explicitly parked

The following work is not part of kernel foundation stabilization:

- GUI, launcher, OLED presentation, and desktop-style features;
- Ustari protocol and future telemetry products;
- cartridge distribution, registry, and commercial application-store policy;
- new AMRN versions or new application delivery modes;
- pre-reset Secure Boot and production key infrastructure;
- arbitrary DMA-controller isolation;
- additional boards before the port boundary is accepted;
- broad codebase refactoring unrelated to a reviewed kernel boundary.

Existing implementations are retained for history and later work. They must
not expand the foundation scope or determine the portable kernel contract.

## Definition of a stable kernel

The kernel foundation is stable only when all of the following are true:

- kernel policy contains no vendor PAC, HAL, register, pin, linker, or board
  identifier;
- architecture-specific exception, context, stack, and fault mechanisms are
  isolated behind an architecture port;
- board-specific clocks, peripherals, interrupts, memory, and unsafe MMIO are
  isolated behind a board backend;
- the F405 port preserves the accepted boot and recovery behavior;
- hardware-neutral policy has deterministic host tests and typed errors;
- the target build, strict lints, and F405 hardware regression pass;
- one additional real backend can be introduced without editing existing
  kernel policy modules;
- documentation, generated profiles, and ownership boundaries match the
  implementation.

Compilation alone does not close a milestone whose behavior depends on real
hardware.

## Phase 0 — Freeze the baseline

Goal: make the current state recoverable and define the exact behavior that
the foundation work must preserve.

- [x] Create `parked/f405-platform-state-2026-09-14` at the current F405
  platform state.
- [x] Create `parked/pre-foundation-main` at the previous `main` tip.
- [x] Create and select the `kernel-foundation` branch.
- [x] Record the baseline commit, target profile, enabled feature set, kernel
  artifact, and existing F405 evidence in a single checkpoint.
- [ ] Record the current public kernel and backend contracts before editing.
- [x] Mark the storage, SDIO, USB CDC, and SPI paths frozen for this roadmap.
- [ ] Classify existing features as foundation, adapter, or parked scope.
- [x] Add a short change-control note requiring approval for ABI, memory-map,
  boot-path, storage-layout, or loader-mode changes.

Exit gate: the baseline can be rebuilt and compared after every foundation
checkpoint, and no parked feature is required by the kernel core contract.

## Phase 1 — Define the kernel layering contract

Goal: establish the permanent ownership model before moving implementation.

- [x] Define `kernel core` as policy and state that does not know a CPU vendor
  or board.
- [x] Define `architecture port` as CPU exception, interrupt, stack, context,
  atomic, and fault operations.
- [x] Define `board backend` as clocks, pins, peripherals, board interrupts,
  linker/memory integration, and board-owned unsafe code.
- [x] Define `driver adapter` as the translation from board resources to
  hardware-neutral driver contracts.
- [x] Define `target metadata` as declarative configuration, not a replacement
  for board-owned hardware initialization.
- [x] Map every current kernel module to exactly one owner.
- [ ] Reject ownership mappings that require a shared module to branch on a
  concrete board or CPU.
- [ ] Update the file-structure and platform-backend documentation to match
  the approved layering contract.

Exit gate: each hardware-dependent operation has one explicit owner and the
kernel core can be reviewed without reading a vendor HAL implementation.

## Phase 2 — Stabilize the architecture contract

Goal: remove CPU assumptions from common kernel APIs while preserving the
current ARMv7-M behavior in its port.

- [ ] Inventory all ARMv7-M assumptions in `dali-kernel-api` and `kernel/src`.
- [ ] Move ARM-specific saved-context layout out of the generic contract.
- [ ] Replace raw ARM exception semantics in common policy with typed
  architecture operations and validated opaque context records.
- [ ] Keep exception entry, PSP/MSP operations, PendSV/SysTick mechanics, and
  fault-register access in the architecture port.
- [ ] Define the minimum architecture capability set required by the kernel.
- [ ] Define explicit unsupported-capability errors instead of silent fallbacks.
- [ ] Keep all architecture `unsafe` operations in the architecture port.
- [ ] Add host tests for context-record validation and architecture capability
  negotiation without simulating hardware execution.
- [ ] Add target compilation checks for the F405 architecture implementation.

Exit gate: a non-ARM architecture can be described by the contract without
introducing ARM names, registers, or exception-return values into kernel
policy.

## Phase 3 — Stabilize the board and platform contract

Goal: make board integration composable and prevent one large board trait from
becoming the kernel architecture.

- [ ] Split the current composite board contract into focused lifecycle,
  timing, interrupt, watchdog, storage, logging, and protection capabilities.
- [ ] Keep capability ownership explicit and reject unavailable services with
  typed errors.
- [ ] Ensure kernel bootstrap consumes capabilities rather than concrete board
  types.
- [ ] Ensure linker scripts, memory regions, DMA regions, and interrupt vectors
  remain backend-owned.
- [ ] Make compile-time backend selection reject zero, duplicate, or ambiguous
  backend selections.
- [ ] Ensure an unselected backend cannot be linked into the kernel image.
- [ ] Remove board-specific policy branches from shared modules.
- [ ] Add host/build tests for capability validation and backend selection.

Exit gate: adding a backend requires a new backend directory and local
registration, not edits to existing kernel policy modules.

## Phase 4 — Reconcile the STM32F405 port

Goal: make the F405 backend the first complete implementation of the new
contracts without changing accepted runtime behavior.

- [ ] Map every F405 PAC/HAL import to the board or architecture port.
- [ ] Move remaining F405 exception and processor register code out of shared
  policy modules.
- [ ] Move F405 linker and memory ownership behind the backend build boundary.
- [ ] Keep SDIO, storage, USB CDC servicing, and frozen SPI paths unchanged
  unless a separate approved change reopens them.
- [ ] Rebuild the F405 firmware using only the new port composition.
- [ ] Compare boot, clock, logging, storage, loader, watchdog, and recovery
  traces with the parked baseline.
- [ ] Record board, power, wiring, firmware revision, transport, expected
  trace, observed trace, and limitations.

Exit gate: the F405 port passes the baseline regression with no weakened
claims and no F405 dependency in kernel policy.

## Phase 5 — Establish the minimal kernel runtime

Goal: stabilize the smallest useful OS kernel before restoring optional
subsystems.

- [ ] Define the kernel bootstrap state machine and failure states.
- [ ] Define bounded initialization order for architecture, board services,
  timer, interrupt controller, watchdog, and lifecycle loop.
- [ ] Define the kernel-owned heartbeat and progress boundary.
- [ ] Define the minimal timer and interrupt contract required by the runtime.
- [ ] Define fixed-size memory-region and buffer ownership policy.
- [ ] Define lifecycle states for boot, ready, running, faulted, recovering,
  and terminated contexts.
- [ ] Ensure malformed external input returns typed errors and cannot panic or
  jump outside validated execution paths.
- [ ] Add host tests for state transitions, ownership, bounds, and failure
  propagation.
- [ ] Verify the minimal runtime on the F405 target before re-enabling optional
  features.

Exit gate: the kernel can boot, enter its bounded lifecycle loop, report a
failure, recover to a defined kernel state, and remain alive without relying
on application, storage, or distribution features.

## Phase 6 — Reattach essential adapters

Goal: reconnect only the services required for the first supported board
without contaminating kernel policy.

- [ ] Define the generic block-storage boundary independently of SDIO.
- [ ] Keep FAT/filesystem policy above the block transport boundary.
- [ ] Keep AMRN parsing independent of both filesystem and board code.
- [ ] Keep logging policy independent of RTT, USB CDC, UART, or another
  transport.
- [ ] Connect the existing F405 storage adapter through the generic boundary.
- [ ] Connect the existing F405 logging adapter through the generic boundary.
- [ ] Validate the original single-application boot path as an adapter-level
  integration test.
- [ ] Document which services are essential kernel services and which remain
  optional extensions.

Exit gate: the F405 MVP path works through generic contracts, while replacing
the transport does not require changing AMRN, loader, or kernel policy code.

## Phase 7 — Software and target validation gate

Goal: close every software checkpoint before claiming a stable foundation.

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo check --workspace --exclude dali-kernel`.
- [ ] Run `cargo check-kernel`.
- [ ] Run focused tests for every changed contract and policy module.
- [ ] Run `cargo test -p dali -p dali-app-hello -p dali-cli`.
- [ ] Run strict host Clippy for the workspace.
- [ ] Run target Clippy for the F405 firmware.
- [ ] Run `cargo build-kernel`.
- [ ] Run `git diff --check`.
- [ ] Inspect the final diff and staged file list for unrelated changes.
- [ ] Audit shared code for vendor identifiers, raw addresses, pins, timing
  literals, and architecture-specific unsafe operations.

Exit gate: all software checks pass and the audit confirms that the contracts
are hardware-neutral rather than merely compiling behind a wrapper.

## Phase 8 — F405 hardware stability gate

Goal: prove that the new boundary preserves real target behavior.

- [ ] Flash the exact firmware produced by the validated source revision.
- [ ] Confirm reset, clock, interrupt, watchdog, and kernel heartbeat output.
- [ ] Confirm the accepted storage and loader path where enabled.
- [ ] Confirm the expected recovery behavior for the affected boundary.
- [ ] Compare the complete trace with the parked F405 baseline.
- [ ] Record all hardware metadata and unverified scenarios.
- [ ] Keep the phase open if hardware or measurement equipment is unavailable.

Exit gate: F405 hardware behavior is unchanged or every intentional behavior
change is separately documented and approved.

## Phase 9 — Independent backend proof

Goal: demonstrate that the universal boundary is real with one additional
supported backend, without expanding the kernel feature surface.

- [ ] Select one real MCU/board whose architecture and memory model differ
  meaningfully from the F405 port.
- [ ] Add its target profile and backend-local linker/memory definitions.
- [ ] Implement only the capabilities required by the minimal kernel runtime.
- [ ] Do not import or modify the existing F405 backend.
- [ ] Build the new backend without editing shared kernel policy modules.
- [ ] Record target-specific boot and lifecycle evidence.
- [ ] Document unsupported capabilities explicitly.

Exit gate: the second backend builds and runs the minimal kernel with no
changes to existing kernel policy or the F405 backend.

## Phase 10 — Foundation release and maintenance rules

Goal: publish the first stable kernel foundation contract.

- [ ] Freeze the v1 kernel/architecture/board contract.
- [ ] Publish the supported capability matrix for each backend.
- [ ] Publish the unsupported-capability and recovery behavior.
- [ ] Update architecture, file-structure, platform-backend, testing, and
  development documentation.
- [ ] Record the F405 and second-backend evidence separately.
- [ ] Remove contradictory or overstated portability claims.
- [ ] Define the review gate for every future architecture or board port.
- [ ] Define the rule that new features must not bypass the port boundary.
- [ ] Merge the validated foundation branch into `main` only after review.

Exit gate: Dali OS has a documented, tested, hardware-neutral kernel core and
at least two independently owned ports with reproducible validation records.

## Checkpoint discipline

Each unchecked item is a separate milestone. A milestone is complete only
when its implementation, focused tests, required target checks, and relevant
hardware evidence are recorded. Do not combine multiple unchecked boundary
changes in one commit or one hardware checkpoint.

The current parked branches are recovery points, not targets for new work:

- `parked/f405-platform-state-2026-09-14` preserves the current platform state;
- `parked/pre-foundation-main` preserves the previous `main` state.

The active implementation branch is `kernel-foundation`.

The first recorded checkpoint is [Kernel Foundation Baseline
Checkpoint](00-baseline-checkpoint.md).

The Phase 1 ownership map is [Kernel Foundation Ownership
Review](00-ownership-review.md).
