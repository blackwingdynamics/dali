# Dali OS Coding Agent Instructions

This document is the operating contract for any coding agent working in this repository. Read it before inspecting or modifying code. The repository is an early-stage embedded Rust platform, so correctness, explicit safety boundaries, and evidence are more important than implementation speed.

## 1. Project mission

Dali OS is a `no_std` Rust operating system and embedded runtime for STM32 microcontrollers and future autonomous or industrial devices.

The first milestone is deliberately narrow:

1. boot a Rust kernel on the STM32F405RGT6 WeAct Studio Core Board;
2. initialize the clock, PB2 status LED, PC13 user key, and RTT logging;
3. read a FAT16/FAT32 SD card over SDIO;
4. discover an `.amrn` package in the card root;
5. validate a fixed 32-byte AMRN header and CRC32 payload checksum;
6. load a native payload into the reserved SRAM region at `0x20008000`;
7. transfer control to `unsafe extern "C" fn(*const ServiceTable) -> !`;
8. verify execution through a deterministic application LED pattern.

The MVP application is trusted native code. It is not sandboxed, isolated,
signed, encrypted, dynamically linked, or interrupt-owning. The repository also
contains feature-gated ABI v3 single-application processor-side isolation and
AMRN v3/v4 loader paths; these are not the default MVP configuration and must
not be described as complete sandboxing, secure boot, DMA isolation, or
multi-application isolation. See `docs/SECURITY.md`, `docs/ABI.md`, and
`docs/TESTING.md` for the current evidence boundary.

## 2. Source of truth

Read the relevant documents before making a change:

- `docs/ARCHITECTURE.md` — system boundaries and MVP scope;
- `docs/AMRN_FORMAT.md` — package bytes and validation rules;
- `docs/ABI.md` — kernel/application execution contract;
- `docs/HARDWARE.md` — board, pins, clock, and SRAM layout;
- `docs/CODING_STANDARDS.md` — code, comments, unsafe, testing, and review rules;
- `docs/ROADMAP.md` — atomic implementation order;
- `docs/TESTING.md` — test strategy;
- `docs/MVP_ACCEPTANCE.md` — physical acceptance procedure;
- `docs/VERSIONING.md` — version and compatibility rules;
- `CONTRIBUTING.md` — branch, commit, PR, and CI rules.

If code and documentation disagree, stop and resolve the contract before implementing. Do not silently choose a new behavior.

## 3. Repository boundaries

```text
kernel/               Embedded kernel and bootstrap
apps/dali-app-hello/  Independent demo application
crates/dali-sdk/      Application SDK scaffold
crates/dali-cli/      Package and device CLI scaffold
crates/dali-usb/      Hardware-neutral bounded USB delivery primitives and host tests
crates/dali-amrn/     Hardware-neutral AMRN format parser and validation
crates/dali-device/   Hardware-neutral device discovery records and ordering
docs/                 Architecture and process documentation
scripts/              Validation and release automation
.github/              CI, release workflow, and PR policy
```

Module ownership:

- `kernel/src/main.rs` — bootstrap orchestration only;
- `kernel/src/platform/f405/board.rs` — board-specific pins, clocks, and peripheral ownership;
- `kernel/src/logging/` — logging facade and hardware backend boundary;
- `crates/dali-usb/` — transport-neutral bounded log delivery state and tests;
- `crates/dali-amrn/` — AMRN header, payload bounds, and CRC32 validation;
- `crates/dali-device/` — transport-neutral discovery records, states, and ordering;
- `kernel/src/storage/` — SD and filesystem access;
- `kernel/src/loader/` — AMRN parsing, CRC32, bounds checks, and execution;
- `kernel/src/runtime/` — future tasks, scheduling, IPC, services, and watchdogs.

Keep package parsing independent from filesystem internals and hardware. Keep hardware-specific code out of package-format logic.

## 4. Non-negotiable technical contracts

### Absolute no-hardcoding rule

Hardcoding is forbidden in implementation code. Do not place hardware, configuration, deployment, or runtime values directly in functions, loaders, drivers, or applications.

The following must be named constants, typed values, configuration fields, or documented board/build definitions:

- memory addresses and SRAM ranges;
- GPIO pins and peripheral identifiers;
- clock frequencies, baud rates, timeouts, and delays;
- buffer sizes, payload limits, stack sizes, and queue capacities;
- filesystem paths and package names;
- protocol versions, target IDs, ABI versions, and feature flags;
- device identifiers, retry counts, and safety thresholds.

Allowed literals are limited to simple language mechanics and values explicitly defined by a specification. Even protocol magic values must be represented by named constants and documented. If a value can change between boards, builds, deployments, or runtime configurations, it must not be a raw literal in implementation code.

Any exception must be reviewed, named, documented, and justified by the relevant architecture or hardware specification.

### Target

- MCU: STM32F405RGT6;
- board: WeAct Studio STM32F405RGT6 Core Board;
- target: `thumbv7em-none-eabihf`;
- clock target: 168 MHz;
- status LED: PB2, active-high; user key: PC13, active-low with pull-up;
- SD interface: hardware SDIO in 4-bit mode.

### SRAM

```text
0x20000000 - 0x20007FFF   Kernel reserved RAM
0x20008000 - 0x20017FFF   Application region, 64 KiB
0x20018000 - 0x2001FFFF   Kernel runtime and stack RAM
```

Do not change this layout without updating `kernel/memory.x`, the AMRN format, the ABI, hardware documentation, tests, and roadmap.

### AMRN

- extension: `.amrn`;
- magic: `DALI`;
- header size: 32 bytes;
- integer encoding: little-endian;
- payload integrity: CRC32;
- load address: `0x20008000`;
- maximum payload: 64 KiB;
- relocation and dynamic linking: unsupported in the MVP.

The loader must reject invalid magic, version, target, sizes, offsets, addresses, CRC32, and entry points before copying or jumping.

### ABI

```rust
unsafe extern "C" fn(*const ServiceTable) -> !
```

MVP applications do not own interrupts, do not use a scheduler, do not access kernel-private symbols, and do not depend on shared RTT logging.

## 5. Required implementation workflow

### Architecture change gate

Never change the repository architecture, module ownership, boot path,
storage layout, backend selection, loader mode, feature contract, ABI, or
filesystem package layout as part of an implementation task. If the requested
behavior appears to require an architectural change, stop before editing,
describe the exact proposed change and its impact, and obtain explicit user
approval. Do not silently substitute a different build profile, loader path,
package location, or storage contract to make a fixture or validation flow
work.

Before editing:

1. Read the relevant contract documents.
2. Identify the roadmap task being implemented.
3. Inspect the current working tree and preserve unrelated user changes.
4. Decide which module owns the change.
5. Define failure cases and validation evidence.
6. State any contract change before implementing it.

While editing:

- make the smallest coherent change;
- use `apply_patch` for file edits;
- always generate minimal, surgically precise Git diffs or patches;
- do not rewrite an entire multi-hundred-line file when changing a single function;
- do not create large monolithic files; split code into small, focused modules and submodules;
- keep each Rust implementation file at or below 300 lines unless an explicit architectural exception is documented;
- keep module boundaries explicit;
- avoid unrelated refactoring;
- use English for all code, comments, logs, errors, and documentation;
- update documentation when behavior or a contract changes;
- add tests or record hardware evidence for the affected behavior.

### No fake hardware or substitute fixtures

- Do not create fake hardware, fake devices, fake storage, fake readers, or
  simulated runtime behavior as a substitute for the real implementation or
  hardware acceptance.
- A test application is allowed only when it is a real compiled package that
  exercises the documented contract on the target; it must not stand in for a
  missing kernel or device implementation.
- Host tests are allowed only for genuinely hardware-neutral codecs,
  validators, and pure contracts using caller-owned byte slices or explicit
  test values. They must never be described as hardware evidence.
- Storage, loader, slot-allocation, fault, and lifecycle behavior that depends
  on device state must be verified through the real implementation and the
  required target hardware; if hardware is unavailable, record the limitation
  instead of inventing a substitute.

After editing:

1. Run the narrowest relevant tests while iterating.
2. Run the complete validation suite before handoff.
3. Inspect the diff and staged file list.
4. Report what changed, what passed, and what remains unverified.

## 6. Safety and `unsafe`

`unsafe` is forbidden by default. MVP use is limited to validated MMIO, validated SRAM copying, the validated native entry jump, and unavoidable low-level HAL operations.

`unsafe` must remain centralized in `board`, `loader/exec`, or an explicitly documented low-level module. It must not spread into parsers, storage policy, SDK APIs, or ordinary runtime logic.

Every unsafe block must:

- be as small as possible;
- follow complete input and bounds validation;
- have an immediate `SAFETY` comment;
- state the invariant that makes it valid;
- be wrapped in a safe abstraction when reused.

Never use `unsafe` to bypass an ownership problem, silence the compiler, or make an unreviewed design shortcut.

Do not add `#[allow(...)]` to hide warnings, safety issues, dead code, or failed lint rules. Any exception must identify the exact warning, explain why it is safe, and be reviewed.

## 7. Error and realtime policy

- Do not use runtime `unwrap()`, `expect()`, `panic!()`, or `unreachable!()` without a documented and reviewed invariant.
- Do not ignore `Result` values.
- Use typed errors that preserve meaningful failure causes.
- Treat all SD-card and package data as untrusted input.
- Do not panic on malformed external data.
- Do not allocate from the heap in safety-critical or realtime paths.
- Use fixed-size buffers and bounded queues.
- Keep blocking SD operations away from future safety-critical tasks.
- Make integer overflow behavior explicit.
- Replace magic addresses, pin numbers, sizes, and protocol values with named constants or types.
- Do not introduce silent fallbacks for invalid configuration, missing storage, malformed packages, or hardware failures unless the fallback is explicitly documented.

## 8. Comment and documentation policy

Comments must explain why, not restate what the code does. Explain hardware constraints, protocol rules, invariants, timing assumptions, and safety arguments.

Do not add commented-out code, decorative banners, stale comments, or untracked TODOs. A TODO requires an issue or roadmap reference.

Public APIs require accurate Rust documentation. Use `# Errors`, `# Panics`, and `# Safety` sections where applicable.

## 9. Validation commands

Run these from the repository root:

```text
cargo fmt --all -- --check
cargo check --workspace --exclude dali-kernel
cargo check-kernel
cargo test -p dali -p dali-app-hello -p dali-cli
cargo clippy --workspace --all-targets --exclude dali-kernel -- -D warnings
cargo clippy -p dali-kernel --target thumbv7em-none-eabihf --bin dali-kernel -- -D warnings
cargo build-kernel
git diff --check
```

The Lefthook pre-commit hook runs the relevant checks automatically. GitHub Actions repeats them for pushes and pull requests. A host build does not prove embedded hardware behavior.

For hardware changes, record the board, wiring, firmware revision, SD card/filesystem, power source, logging channel, expected output, observed output, and result.

Never claim completion based only on compilation. Distinguish clearly between source compilation, host tests, embedded target checks, hardware tests, and MVP acceptance.

## 10. Git and release policy

Use Conventional Commits:

```text
feat(loader): validate AMRN payload bounds
fix(storage): reject truncated SD blocks
docs(amrn): define the CRC32 field
```

Keep commits atomic. Do not combine unrelated formatting, dependencies, behavior, or documentation changes.

Changelogs are generated from commit history by `git-cliff`. Do not edit release changelogs manually. Release tags use `vX.Y.Z` or `vX.Y.Z-alpha.N`; the release workflow creates the archived changelog and GitHub Release.

## 11. Things an agent must not do

- Do not expand the MVP without a roadmap and contract update.
- Do not claim sandboxing, secure boot, authenticity, memory isolation, or fault isolation before implementation and evidence exist.
- Do not add dynamic linking, new relocation contracts, application-owned interrupts, or SDK APIs prematurely.
- Do not overwrite user changes or use destructive Git commands.
- Do not commit `target/`, binaries, secrets, credentials, or local IDE files.
- Do not make external messages, releases, or repository settings changes unless explicitly requested.
- Do not commit code that bypasses a failed CI or Lefthook check.
- Do not add dependencies without checking `no_std` compatibility, license, maintenance, auditability, transitive impact, and binary/RAM cost.
- Do not change an ABI, AMRN field, memory layout, public API, or security claim without updating its specification, tests, and versioning.
- Do not disable or ignore tests without an issue or roadmap reference and a documented reason.
- Do not log secrets, credentials, private keys, sensitive device data, or unnecessary memory dumps.
- Do not perform mass formatting, renaming, migration, or repository rewrites outside the task scope.
- Do not use `git reset --hard`, `git clean -fd`, force push, or bulk deletion without explicit user approval.
- Do not claim hardware or acceptance completion without the required evidence.
- Do not add unpinned or unexplained dependencies.

Functions should normally remain below 50 lines, use no more than three nesting levels, and perform one responsibility. Split a function when it exceeds these limits unless the exception is documented.

Builds must remain reproducible: preserve lockfiles, pin important tools and dependencies, and document commands that produce generated artifacts.

## 12. Handoff format

At the end of a coding task, report:

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

Do not call work complete when it only compiles if the task requires hardware acceptance, integration, or documentation evidence.
