# Dali OS Agent Contract

This document is the mandatory operating contract for every coding agent in
this repository. Read it before inspecting, editing, testing, or deleting
anything. `AGENTS.bak` preserves the previous contract and is historical
reference only; this file is the active contract.

Dali OS is an early-stage `no_std` embedded operating system. Correctness,
explicit ownership boundaries, reversible changes, and real hardware evidence
are more important than implementation speed.

## 1. Highest priority: stabilize the kernel foundation

The active branch is `kernel-foundation`. The highest-priority work is to
clean up, simplify, and stabilize a universal kernel foundation.

The target is a hardware-neutral kernel core with explicit architecture and
board ports. It is not one binary that runs on every CPU. A new CPU or board
must be addable through a new port without editing existing kernel policy.

The active foundation roadmap is `docs/roadmap/00-kernel-foundation.md`.
It takes priority over GUI, Ustari, cartridge distribution, metadata,
commercial workflows, new AMRN versions, new boards, and unrelated feature
work.

The foundation work must be conservative:

- preserve the F405 behavior and evidence baseline;
- make small, atomic, reversible changes;
- do not delete working subsystems merely to make the tree look smaller;
- park non-foundation features instead of removing them without a replacement;
- do not weaken an existing contract or evidence claim silently;
- stop and request approval before an architecture, ABI, boot, storage, loader,
  memory, or feature-contract change.

The protected recovery points are:

- `parked/f405-platform-state-2026-09-14` — current full F405 state;
- `parked/pre-foundation-main` — previous `main` state.

Do not rewrite, delete, or develop on parked branches as part of foundation
work.

## 2. Source of truth and baseline contracts

Read the relevant documents before changing code:

- `docs/architecture/README.md` — kernel boundaries and scope;
- `docs/file-structure/README.md` — repository ownership;
- `docs/amrn-format/README.md` — cartridge bytes and validation;
- `docs/abi/README.md` — application execution contract;
- `docs/hardware/README.md` — board and memory facts;
- `docs/coding-standards/README.md` — engineering and safety rules;
- `docs/roadmap/README.md` — execution order and acceptance gates;
- `docs/testing/README.md` — evidence categories;
- `docs/mvp-acceptance/README.md` — physical MVP procedure;
- `docs/platform-backends/README.md` — port ownership and addition rules;
- `CONTRIBUTING.md` — branch, commit, PR, and CI rules.

If code and documentation disagree, stop and resolve the contract before
implementing. Do not silently choose a new behavior.

The first reference target is the WeAct Studio STM32F405RGT6 Core Board with
`thumbv7em-none-eabihf`, a 168 MHz clock, PB2 status LED, PC13 active-low key,
and hardware SDIO in 4-bit mode. These are F405 port facts, not universal
kernel facts.

The baseline MVP uses a 32-byte little-endian AMRN header, `DALI` magic,
CRC32 payload integrity, a maximum 64 KiB payload, fixed application SRAM
origin `0x20008000`, and the documented entry contract:

```rust
unsafe extern "C" fn(*const ServiceTable) -> !
```

The loader must validate magic, version, target, sizes, offsets, addresses,
CRC32, and entry point before copying or jumping. Do not change the ABI, AMRN
format, memory map, or loader mode without updating its specification, tests,
documentation, versioning, and roadmap, followed by explicit approval.

## 3. Non-negotiable hardware-neutrality rules

### Absolute no-hardcoding rule

**HARDCODING IS STRICTLY FORBIDDEN.** Do not place hardware, configuration,
deployment, protocol, or runtime values directly in functions, loaders,
drivers, applications, or kernel policy.

Values that change between boards, CPUs, builds, deployments, or runtime
configurations must be represented by named constants, types, validated
configuration, target metadata, manifest fields, or documented board/build
definitions. This includes:

- memory addresses, SRAM ranges, linker regions, and stack boundaries;
- GPIO pins, peripheral identifiers, interrupt numbers, and register values;
- clock frequencies, baud rates, timeouts, delays, and polling limits;
- buffer sizes, payload limits, queue capacities, and retry counts;
- filesystem paths, package names, target IDs, ABI versions, and feature flags;
- protocol magic values, device IDs, safety thresholds, and policy limits.

Protocol literals explicitly required by a specification must still be named,
documented constants. Any exception must be reviewed, named, documented, and
justified by the owning specification.

### No concrete board in the kernel

**A CONCRETE BOARD, MCU, PAC, HAL, PIN, REGISTER, OR BOARD ID MUST NEVER BE
HARD-CODED INTO KERNEL POLICY.**

The following are forbidden in `kernel/src` and hardware-neutral contracts:

- `stm32`, `stm32f4xx-hal`, `f405`, `f411`, or another concrete board name;
- vendor PAC/HAL types, register addresses, pin numbers, linker symbols, or
  board-specific interrupt handlers;
- ARM-specific implementation details in a supposedly architecture-neutral
  policy module;
- board-specific branches in shared runtime, loader, storage policy, or
  scheduler code.

Vendor PAC/HAL code, unsafe MMIO, clocks, pins, interrupts, linker/memory
definitions, and processor-specific exception code belong in the selected
architecture or board backend under `crates/dali-boards/` or an equivalent
port-owned directory.

The kernel may consume typed capabilities, regions, timing profiles, and
operations supplied by a port. It may not know how that port implements them.

### Dependency direction

Dependencies must point toward stable, hardware-neutral contracts:

```text
kernel policy -> hardware-neutral contracts
architecture and board ports -> kernel contracts
drivers -> port-provided resources and driver contracts
CLI, SDK, and applications -> public application contracts
```

The kernel core must not depend on a concrete board crate, vendor HAL, CLI,
SDK, application, or deployment tool. A port may depend on the kernel API, but
the kernel API must not depend on a port. Reverse dependencies, cyclic module
ownership, and hidden cross-layer imports are architectural defects.

### Module and file placement

**FILES MUST NOT BE DROPPED INTO A DIRECTORY WITHOUT AN EXPLICIT MODULE
OWNER.** Before adding a file, identify its responsibility, owning module,
public/private boundary, dependency direction, and reason it cannot live in an
existing focused module.

Group code into small, coherent modules and submodules whenever separation is
possible. Split mixed responsibilities such as policy, parsing, transport,
hardware access, formatting, and error mapping. Keep `mod.rs` and façade files
focused on composition and exports; do not use them as dumping grounds for
implementation code.

Every new module must have a clear name, one primary responsibility, explicit
visibility, and a documented source-of-truth relationship. Remove duplicate
implementations instead of adding a second path that can drift.

### No large implementation files

**RUST IMPLEMENTATION FILES MUST NOT EXCEED 300 LINES.**

Split files by explicit responsibility and ownership before they become large.
Do not create monolithic modules, giant traits, broad façade files, or mixed
policy/driver/backend implementations. A documented, reviewed exception must
identify the file, explain why a split would damage the boundary, and be
approved before the file exceeds the limit.

Existing oversized files are inventory items, not permission for blind deletion
or mechanical splitting. Split one responsibility at a time, preserve public
behavior, validate the affected target path, and record the ownership reason.

Functions should normally remain below 50 lines, use no more than three
nesting levels, and perform one responsibility. Split them otherwise.

## 4. Architecture and ownership model

Keep these layers explicit:

- `kernel core`: policy, state, lifecycle, scheduling policy, memory policy,
  typed errors, and hardware-neutral contracts;
- `architecture port`: exception entry, interrupt control, stack/context
  operations, atomics, fault registers, and architecture-specific unsafe code;
- `board backend`: clocks, pins, peripherals, board interrupts, watchdog,
  linker/memory integration, and board-owned unsafe code;
- `driver adapter`: conversion from board resources to hardware-neutral driver
  contracts;
- `target metadata`: declarative configuration consumed through validated,
  typed profiles; it does not replace hardware ownership code.

Keep package parsing independent from filesystems and hardware. Keep storage
policy independent from SDIO. Keep logging policy independent of RTT, USB,
UART, or another transport. Keep application APIs independent from kernel
private symbols.

Primary ownership:

- `kernel/src/` — kernel policy and orchestration only;
- `crates/dali-kernel-api/` — public hardware-neutral port contracts;
- `crates/dali-boards/` — board and architecture implementations;
- `crates/dali-driver-api/` — allocation-free driver contracts and errors;
- `crates/dali-targets/` — target manifest validation and generated profiles;
- `crates/dali-amrn/` — hardware-neutral AMRN parsing and validation;
- `kernel/src/storage/` — filesystem and repository policy;
- `kernel/src/loader/` — validated loading and execution policy;
- `docs/roadmap/00-kernel-foundation.md` — active foundation sequence.

## 5. Scope and evidence boundary

The F405 MVP remains the first regression target: boot, clock, GPIO, SDIO,
read-only FAT16/FAT32 access, AMRN validation, bounded loading, entry transfer,
logging, watchdog, and documented recovery behavior.

The baseline ABI v2 application is trusted native code. Do not describe it as
sandboxed, isolated, secure-booted, signed, encrypted, dynamically linked, or
multi-application isolated.

Feature-gated ABI v3, MPU, context switching, relocation, signed cartridges,
repository metadata, and fault fixtures remain bounded capabilities. Do not
promote them to general security or portability claims without implementation,
target validation, and the required hardware evidence.

Compilation, flashing, enumeration, simulation, and host tests are not
hardware acceptance. Report host tests, target checks, embedded builds,
Silicon Trace, and physical acceptance separately.

Never invent fake hardware, fake storage, fake devices, fake readers, or
simulated runtime behavior as a substitute for real implementation or target
evidence. Host doubles are allowed only for genuinely hardware-neutral pure
contracts and must never be called hardware evidence.

## 6. Mandatory implementation workflow

Before editing:

1. Read the relevant architecture, ABI, AMRN, hardware, coding, roadmap, and
   testing documents.
2. Read this file and inspect the current working tree and branch.
3. Identify the exact foundation roadmap milestone and owning module.
4. Preserve unrelated user changes and parked recovery points.
5. Define failure cases, invariants, and required evidence.
6. State any proposed contract or architecture change before implementing it.

While editing:

- use `apply_patch` for local file edits;
- make the smallest coherent change;
- preserve public APIs, boot order, generated output, and frozen paths;
- avoid mass formatting, renaming, migration, and unrelated refactoring;
- use English for code, comments, logs, errors, and documentation;
- add or update tests and documentation for changed behavior;
- do not add untracked TODOs; reference a roadmap item or issue instead.

After editing:

1. Run the narrowest relevant checks while iterating.
2. Run the complete applicable validation suite before handoff.
3. Inspect the diff, file list, and staged file list.
4. Run `git diff --check`.
5. Report changed files, passed checks, hardware evidence, and limitations.

### Definition of Done

A milestone is not complete because the code compiles. It is complete only
when all applicable items are satisfied:

- implementation matches the owning contract and module boundary;
- failure cases and invariants are covered by focused tests;
- documentation and roadmap status match the implementation;
- target checks and strict lints pass;
- required real-hardware evidence is recorded, or the limitation is explicit;
- generated output and public APIs are unchanged or documented;
- the diff contains no unrelated cleanup or hidden behavior change;
- the change is reversible through an identified commit or recovery point.

### Permanent-quality rule

**TEMPORARY, TOY, AD-HOC, OR “JUST TO MAKE THE TEST PASS” SOLUTIONS MUST NOT
BE COMMITTED AS PRODUCTION CODE.**

Do not add timing hacks, arbitrary delays, caller-level polling, silent
fallbacks, placeholder hardware behavior, fake implementations, commented-out
code, or unbounded retry loops as a temporary measure. If the correct design
is not ready, stop at a documented boundary, add a roadmap item, and report
the limitation. Every committed path must meet the same high engineering and
enterprise-quality standard whether it is marked experimental or production.

Quality must be continuously controlled through focused ownership, typed
errors, bounded behavior, reviewable diffs, strict linting, tests, and evidence.
Do not defer known quality defects under the label of future cleanup.

### Architecture decision records

Any change to a kernel/port boundary, ABI, memory layout, scheduler,
storage/loader contract, public API, or feature ownership requires a concise
decision record before implementation. It must state the problem, options
considered, selected design, rejected alternatives, compatibility impact,
validation plan, and rollback path. Link the record from the relevant roadmap
item or contract document.

## 7. Architecture change gate

Do not silently change repository architecture, module ownership, boot path,
storage layout, backend selection, loader mode, feature contract, ABI,
filesystem layout, or memory layout.

If a task requires such a change, stop before editing and document:

- the exact proposed change;
- the affected modules and contracts;
- compatibility and migration impact;
- test and hardware evidence required;
- why the change is necessary for the foundation milestone.

Obtain explicit approval before implementation. Never substitute a different
profile, loader, package path, or fixture merely to make a check pass.

## 8. Safety, unsafe, errors, and realtime

`unsafe` is forbidden by default. It is limited to validated low-level MMIO,
validated SRAM copying, validated native entry transfer, architecture-port
operations, and unavoidable HAL operations.

Every unsafe block must be minimal, have an immediate `SAFETY` comment, state
the invariant that makes it valid, and be hidden behind a safe abstraction when
reused. Never use unsafe to bypass ownership or silence the compiler.

Do not add `#[allow(...)]` to hide warnings, safety issues, dead code, or lint
failures. Exceptions require a precise warning, safety explanation, and review.

Do not use runtime `unwrap()`, `expect()`, `panic!()`, or `unreachable!()`
without a documented, reviewed invariant. Do not ignore `Result` values.
Use typed errors that preserve meaningful causes. External data is untrusted.
Do not panic on malformed storage or package data.

Do not allocate from the heap in safety-critical or realtime paths. Use fixed
buffers and bounded queues. Make overflow behavior explicit. Do not introduce
silent fallbacks for invalid configuration, missing hardware, malformed data,
or failed services.

### Interrupt and concurrency discipline

- Blocking storage, filesystem, logging flushes, and long policy operations
  are forbidden in interrupt context.
- Interrupt handlers must be short, bounded, reentrant where required, and
  limited to ownership-safe event capture or acknowledgement.
- Critical sections must be as short as possible and must never hide an
  unbounded loop or hardware wait.
- Shared state requires an explicit owner, synchronization rule, and lifetime.
- Watchdog feed ownership must be unique and documented.
- Every polling loop requires a named bound, progress rule, and typed timeout.
- Do not fix starvation or lifecycle defects with arbitrary delays or caller-
  level extra polling.

### Generated and configured content

Generated files must not be edited by hand. The generator and its source
manifest are authoritative. Board facts must have one source of truth in the
target manifest or backend configuration and must not be copied into kernel
policy, CLI code, tests, or documentation as independent values. A generator
change must include generated-output validation and documentation updates.

## 9. Frozen and protected paths

During kernel foundation work, do not change these paths without explicit
approval and a separate evidence plan:

- `kernel/src/storage/` and the accepted SDIO/storage behavior;
- F405 SDIO raw transport and DMA setup;
- USB CDC core servicing and polling/interrupt ownership;
- SPI/ILI9341 experiments;
- memory layout, linker ownership, ABI, and AMRN MVP format;
- parked branches and recorded hardware evidence.

Refactoring must preserve behavior. A passing host build is not permission to
change a frozen hardware path.

## 10. Validation commands

Run from the repository root as applicable:

```text
cargo fmt --all -- --check
cargo check --workspace --exclude dali-kernel
cargo check-kernel
cargo test -p dali -p dali-app-hello -p dali-cli
cargo clippy --workspace --all-targets --exclude dali-kernel -- -D warnings
just kernel-clippy f405
cargo build-kernel
git diff --check
```

For hardware changes, record board, wiring, firmware revision, SD card/filesystem,
power, logging/transport, expected trace, observed trace, result, and limits.

## 11. Git, commits, and deletion policy

Use Conventional Commits and keep commits atomic:

```text
refactor(api): split architecture and board contracts
refactor(platform): isolate board composition from kernel policy
test(kernel): add portable capability contract coverage
docs(roadmap): define kernel foundation execution plan
```

Do not combine formatting, dependencies, behavior, documentation, and
unrelated cleanup in one commit. Do not commit `target/`, binaries, secrets,
credentials, IDE files, or generated local artifacts.

No drive-by cleanup is allowed. Do not rename, reformat, reorganize, delete,
or migrate code merely because a nearby task makes it convenient. Every
cleanup change needs its own scope, owner, rationale, validation, and atomic
commit. Unrelated quality findings must become separate roadmap items.

Do not add a dependency without checking `no_std` compatibility, license,
maintenance, auditability, transitive impact, and binary/RAM cost. Preserve
lockfiles and reproducible tool versions. Do not edit generated changelogs;
release changelogs are produced from Conventional Commits.

**DELETION IS NOT A CLEANUP STRATEGY BY DEFAULT.** Do not mass-delete modules,
features, applications, documents, tests, or evidence. First prove that the
item is obsolete, identify all references, preserve a recovery point, and get
explicit approval for material deletion. Prefer parking, feature isolation,
or a focused deprecation step.

Never use `git reset --hard`, `git clean -fd`, force push, bulk deletion, or
history rewriting without explicit user approval. Never overwrite unrelated
user changes.

Before merging `kernel-foundation` into `main`, audit implementation, tests,
hardware evidence, documentation, roadmap status, and security claims. Fix
stale or contradictory documentation before merging.

Do not make external messages, releases, repository-settings changes, or
remote administrative changes unless explicitly requested.

## 12. Documentation and language

Documentation must state why, ownership, invariants, limits, and evidence.
Comments must explain why rather than repeat code. Public APIs require accurate
Rust documentation with `# Errors`, `# Panics`, and `# Safety` where applicable.

All repository artifacts produced by implementation work must use English:
code, comments, logs, errors, tests, and documentation.

## 13. Handoff format

Every coding handoff must report:

```text
Summary:
- ...

Validation:
- command: result

Hardware evidence:
- not run / details

Remaining limitations:
- ...
```

Never call work complete when it only compiles if the task requires runtime,
integration, target, hardware, or documentation evidence.
