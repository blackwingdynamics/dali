# Universal Dali OS Platform Architecture

Status: **Active — architecture definition and migration planning**.

This roadmap converts Dali OS from an F405-first kernel into a portable OS
with independently packaged platform backends. A new board must be implemented
inside its own backend directory and target profile. Adding that board must not
require changes to kernel core, boot policy, runtime policy, loader policy,
security policy, or existing backend code.

## Architectural target

```text
Dali OS core
├── boot policy
├── runtime and scheduler policy
├── loader and cartridge policy
├── security policy
├── hardware-neutral driver contracts
└── platform backend boundary
    ├── f405/
    ├── pico-rp2040/
    ├── pico-rp2350/
    └── fpga-<soc>/
```

The core consumes typed capabilities and services. Only a platform backend may
depend on a vendor PAC, HAL, register map, linker layout, interrupt table, or
board pin mapping.

## Non-negotiable invariants

- [ ] A new board is added under a new platform/backend directory.
- [ ] A new board adds or extends only its target manifest/profile and backend
  registration metadata; existing OS core source files remain unchanged.
- [ ] Backend selection is compile-time isolated through a dedicated backend
  feature or equivalent generated configuration; runtime discovery never
  selects PAC/HAL code.
- [ ] No platform backend imports another platform backend.
- [ ] Hardware-neutral crates contain no PAC, HAL, register, pin, or board type.
- [ ] Hardware values come from the target manifest or generated typed profile.
- [ ] Backend selection is deterministic and rejects ambiguous or unsupported
  profiles at build time.
- [ ] Memory sections, linker symbols, and hardware-specific addresses are
  owned by the selected backend and never assumed by shared kernel policy.
- [ ] Protection policy is hardware-neutral; MPU, PMP, or equivalent register
  programming is supplied through a backend-owned protection provider.
- [ ] Existing F405 behavior, boot order, ABI, memory layout, loader mode,
  storage behavior, and frozen boundaries remain unchanged.
- [ ] No compatibility claim is accepted from compilation alone.

## Phase 0 — Baseline and architecture contract

- [ ] Record the current F405 revision, build profile, test results, boot trace,
  storage trace, and known I2C/OLED hardware limitation.
- [ ] Inventory every F405 dependency outside `kernel/src/platform/f405/`.
- [ ] Audit all `#[link_section]` attributes, linker symbols, static buffers,
  and hardware memory addresses outside backend directories.
- [ ] Audit all MPU/PMP or processor-protection register accesses outside
  backend directories.
- [ ] Define the platform service boundary and ownership rules.
- [ ] Define compile-time backend selection without runtime PAC/HAL discovery
  or fragile board-specific build-script behavior.
- [ ] Define backend capability metadata and unsupported-capability behavior.
- [ ] Update architecture and target-manifest documentation before code changes.
- [ ] Obtain review approval for the platform boundary.

Evidence gate: the F405 baseline trace must be reproducible and attached to the
checkpoint. No migration begins while the baseline is ambiguous.

## Phase 1 — Hardware-neutral kernel boundary

- [ ] Move common platform-facing policy behind typed hardware-neutral
  interfaces.
- [ ] Remove direct F405 PAC/HAL types from kernel core modules.
- [ ] Keep all unsafe MMIO and interrupt setup inside the selected backend.
- [ ] Make boot orchestration consume a backend-provided capability set.
- [ ] Make core security consume typed protection regions and permissions
  through a `MemoryProtectionProvider` boundary.
- [ ] Preserve the existing public contracts and generated target profiles.
- [ ] Add host tests for backend selection, capability validation, and failure
  reporting.

Evidence gate: build and flash the unchanged F405 behavior through the new
boundary. Compare boot, storage, loader, scheduler, watchdog, and diagnostic
traces with the Phase 0 baseline before proceeding.

## Phase 2 — Self-contained F405 backend

- [ ] Consolidate F405 clock, GPIO, timer, serial, I2C, SPI, watchdog,
  interrupts, SDIO, USB, linker, and memory ownership under
  `kernel/src/platform/f405/`.
- [ ] Remove accidental F405 assumptions from shared modules.
- [ ] Move F405 MPU register programming, CCM/SRAM section placement, and
  processor-specific fault setup into the F405 backend.
- [ ] Keep SDIO, Storage, USB CDC core, and SPI/ILI9341 frozen unless a separate
  approved change explicitly reopens them.
- [ ] Generate the F405 backend manifest/profile without duplicating values in
  policy code.
- [ ] Record the F405 regression evidence for this migration.

Evidence gate: a fresh F405 firmware must boot and preserve the accepted
baseline traces. I2C device ACK and OLED rendering remain separate hardware
acceptance items and cannot be inferred from this migration.

## Phase 3 — Directory-local backend registration

- [ ] Define an explicit backend directory contract containing implementation,
  target metadata, linker/memory integration, and validation commands.
- [ ] Make backend discovery manifest-driven or generated, so adding a new
  directory does not require editing OS core source.
- [ ] Generate backend/profile registries from target metadata.
- [ ] Keep Cargo features or generated cfg values limited to compile-time
  backend isolation; they must not encode board policy in shared modules.
- [ ] Reject duplicate backend IDs, target IDs, capabilities, and memory ranges.
- [ ] Add a template or documented checklist for new backend directories.
- [ ] Add a compatibility test proving an unselected backend is not linked.

Evidence gate: build the F405 backend from its directory-local registration and
verify the binary, boot order, and runtime traces are unchanged.

## Phase 4 — Pico backend

- [ ] Identify the exact board and MCU: RP2040 or RP2350.
- [ ] Create a self-contained `kernel/src/platform/pico-<chip>/` backend.
- [ ] Add only the Pico target manifest/profile and backend-local integration.
- [ ] Implement clocks, GPIO, timer, watchdog, interrupts, and USB through the
  common contracts.
- [ ] Provide Pico-specific linker, memory, and protection implementation
  without importing F405 MPU, CCM, SRAM, or exception assumptions.
- [ ] Add Pico-specific linker and memory definitions in the backend directory.
- [ ] Add host/build validation before target flashing.
- [ ] Record real Pico boot and transport evidence.

Evidence gate: Pico evidence must cover the supported capabilities separately.
Pico success must not be inferred from F405 success or simulation output.

## Phase 5 — FPGA platform model

- [ ] Select the execution model: ARM SoC, RISC-V SoC, or supported soft-core.
- [ ] Define CPU, memory, interrupt controller, timer, serial, and
  memory-mapped peripheral contracts.
- [ ] Create a self-contained `kernel/src/platform/fpga-<soc>/` backend.
- [ ] Add the FPGA target profile and linker/memory definition locally.
- [ ] Keep FPGA-specific peripherals behind the common capability boundary.
- [ ] Keep MPU, PMP, or equivalent protection register programming inside the
  FPGA backend through the protection-provider boundary.
- [ ] Record hardware or vendor-board evidence for every enabled capability.

Evidence gate: an FPGA backend is accepted only with target-specific boot and
service evidence. A bitstream build alone is insufficient.

## Phase 6 — Universal build and release workflow

- [ ] Build any selected backend from its manifest/profile without editing core
  kernel source.
- [ ] Add a backend matrix for host checks, target checks, and release builds.
- [ ] Ensure generated output is deterministic and reviewed.
- [ ] Add CI checks for backend isolation and forbidden cross-backend imports.
- [ ] Document board bring-up, flashing, debugging, and evidence requirements.
- [ ] Publish a new-board contribution checklist.

## Per-backend checkpoint protocol

Every backend change is an independent checkpoint. Before starting the next
backend file or migration step:

1. record the source revision and exact changed files;
2. run formatting, focused tests, workspace checks, target checks, and Clippy;
3. build the target firmware;
4. flash the real target board;
5. capture the expected boot and service trace;
6. compare it with the previous accepted baseline;
7. record board, power, wiring, transport, firmware revision, result, and
   limitations;
8. obtain review approval.

If the required hardware or measurement equipment is unavailable, the
checkpoint remains open. A host test, simulator run, or successful flash is
not hardware acceptance.

## Completion criteria

- [ ] F405 is isolated behind the platform boundary.
- [ ] A new backend requires no edits to OS core policy modules.
- [ ] A new backend can be added in its own directory with local metadata,
  linker/memory definitions, and validation.
- [ ] F405, Pico, and at least one FPGA/SoC backend pass their own evidence
  gates, where those backends are implemented.
- [ ] Documentation, generated profiles, build matrix, and repository tree
  match the actual implementation.
- [ ] No unsupported portability, security, or hardware claims remain.
