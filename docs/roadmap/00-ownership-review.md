# Kernel Foundation Ownership Review

Status: **Architecture layout boundary started — exception mechanics remain open**.

This review maps the current implementation to the foundation layers before
any code movement. The map is an ownership contract for the next refactoring
milestones; it is not permission to change ABI, memory layout, boot order,
storage layout, or loader behavior.

## Foundation layers

| Layer | Responsibility | Allowed dependencies |
| --- | --- | --- |
| Kernel core | Policy, lifecycle, bounded state, typed errors, validation, and orchestration | Hardware-neutral contracts and typed target data |
| Architecture port | CPU exceptions, interrupt control, stack/context operations, fault registers, and CPU-specific unsafe code | Kernel API and architecture implementation dependencies |
| Board backend | Clocks, pins, peripherals, board interrupts, linker/memory integration, and board unsafe code | Kernel API, target profile, vendor PAC/HAL |
| Driver adapter | Translation between board resources and hardware-neutral driver contracts | Driver API and selected board resources |
| Target metadata | Declarative target facts, capability validation, and generated profiles | Manifest parser/generator dependencies only |

The kernel core must not depend on a concrete board or CPU implementation. A
port may implement a kernel contract; the contract must not import the port.

## Current ownership map

### Kernel core policy

These modules are kernel policy or orchestration and should remain hardware-
neutral:

- `kernel/src/lib.rs`
- `kernel/src/bootstrap/`
- `kernel/src/runtime/application/`
- `kernel/src/runtime/memory/`
- `kernel/src/runtime/watchdog/`
- `kernel/src/drivers/lifecycle/`
- `kernel/src/loader/contract/`
- `kernel/src/loader/pipeline/` — except architecture-specific launch details
- `kernel/src/storage/` — filesystem and repository policy
- `kernel/src/abi.rs`
- `crates/dali-amrn/`
- `crates/dali-driver-api/`

These modules may consume typed regions, timing, capabilities, and operations,
but must not implement vendor register access or CPU exception mechanics.

### Architecture port boundary

The following current code contains architecture-specific behavior and must be
reviewed behind an architecture-owned boundary:

- `crates/dali-kernel-api/src/architecture.rs` — the saved-context layout is
  now supplied by the selected architecture port; exception operations and
  ARM field semantics remain under review;
- `kernel/src/runtime/scheduling/saved_state.rs` — the saved record still
  carries architecture-defined fields, but application control-state
  selection now comes from the architecture capability boundary;
- `kernel/src/security/fault/scb.rs` — Cortex-M SCB MMIO;
- `kernel/src/security/fault/mod.rs` — architecture-specific fault status and
  exception-return decoding mixed with kernel recovery policy;
- `kernel/src/security/launch/` — portable launch validation mixed with
  architecture-specific frame materialization and entry transfer;
- `kernel/src/security/scheduling/mod.rs` — PendSV/SysTick handlers and
  architecture-specific exported exception symbols;
- `kernel/src/security/privilege/svc.rs` — ARM exception frame and EXC_RETURN
  semantics mixed with service policy;
- `kernel/src/platform/mod.rs` — architecture operations are composed here,
  but this file also owns board registration, USB, MPU, watchdog, and global
  service state.

The first architecture refactor must separate policy validation from the
architecture implementation without changing the current F405 behavior.

### Board backend

The selected board implementation is correctly outside kernel policy:

- `crates/dali-boards/dali-board-stm32f405/` owns F405 PAC/HAL, clocks, pins,
  interrupts, SDIO, USB, watchdog, MPU registers, scheduling wrappers, and
  linker/memory integration;
- `crates/dali-firmware/` owns the selected firmware composition and entry
  point;
- `targets/f405.toml` owns declarative F405 target facts.

No new board is added until the shared boundary is accepted and the F405
regression gate is preserved.

### Driver adapters

These modules translate board resources or generic transports into contracts:

- `kernel/src/drivers/block.rs`;
- `kernel/src/drivers/sdio.rs`;
- `kernel/src/drivers/mod.rs` adapter types;
- `crates/dali-boards/dali-board-stm32f405/src/drivers/`;
- `crates/dali-boards/dali-board-stm32f405/src/sdio.rs`;
- `crates/dali-kernel-api/src/storage.rs` and `dma.rs` contracts.

The generic API must not expose SDIO or F405 details where a block-transport
contract is sufficient. This is an identified review item, not an approved
format change.

### Target metadata

The following own target configuration and generation:

- `targets/*.toml`;
- `crates/dali-targets/src/lib.rs`;
- `crates/dali-targets/build/`;
- `crates/dali-firmware/build.rs` selection and linker-artifact forwarding.

Generated profiles are authoritative outputs of the manifest generator and
must not be hand-edited or duplicated in kernel policy.

## Parked or optional subsystems

These are retained but are not foundation ownership blockers:

- `kernel/src/loader/repository/` and `repository_boot.rs` — optional signed
  repository path;
- `kernel/src/storage/durable/` — optional durable update/trust path;
- authentication, metadata, signed-cartridge, relocation, and multi-context
  feature paths;
- CLI, SDK, device discovery, and application fixture crates;
- display/OLED and diagnostic presentation paths.

They must not introduce dependencies into the minimal kernel core. Their
contracts remain protected until a later roadmap reopens them.

## Boundary findings

The review found these concrete issues for Phase 1 implementation:

1. `kernel/src/platform/mod.rs` was 485 lines and combined composition,
   architecture operations, board metadata, memory protection, USB, and
   watchdog service state. The first behavior-preserving split now leaves a
   249-line facade plus focused `architecture`, `registry`, `protection`, and
   `usb` modules. The remaining architecture policy split is still open.
2. `crates/dali-kernel-api/src/architecture.rs` exposes ARMv7-M layout in a
   common contract. The generic contract must retain only portable operations
   and opaque/typed context requirements.
3. `kernel/src/security/` contains both portable security policy and Cortex-M
   register/frame mechanics. These responsibilities must be separated.
4. `kernel/src/runtime/scheduling/` contains portable scheduling policy beside
   ARM-specific saved-state and PendSV terminology. The policy must not depend
   on an ARM register layout.
5. Several loader, repository, filesystem, and scheduler files exceed the
   300-line implementation limit. They are inventory items for focused
   ownership splits, not permission for mass mechanical rewriting.
6. `kernel/src/drivers/sdio.rs` and the public storage API require a contract
   review so generic block transport is not confused with SDIO policy.

## Next implementation sequence

- [ ] Approve this ownership map as the Phase 1 implementation boundary.
- [x] Split `kernel/src/platform/mod.rs` into focused composition modules with
  no behavior change.
- [ ] Separate portable context/scheduler records from ARM exception mechanics
  beyond the layout and initial-control capability boundaries.
- [ ] Move SCB and exception-frame operations behind the architecture port.
- [ ] Recheck kernel-wide dependency direction and forbidden hardware names.
- [ ] Run the complete software validation suite.
- [ ] Run the F405 target build and compare the existing hardware trace before
  closing the first code checkpoint.

No implementation file is complete until its ownership, diff, tests, target
checks, and required hardware evidence are recorded separately.
