# Dali OS Coding Standards

This document defines the mandatory coding, commenting, safety, testing, and review standards for Dali OS. These rules apply to the kernel, embedded applications, package tooling, and future SDK code unless a stricter module-specific rule is documented.

The project prioritizes predictable behavior, explicit ownership, small safety boundaries, and code that can be reviewed against the architecture and roadmap.

## 1. Language and formatting

- Source code, comments, documentation, log messages, and error messages must be written in English.
- Rust code must be formatted with `rustfmt`.
- Formatting checks are mandatory before a change is accepted.
- Prefer readable line lengths; keep lines at or below 100–120 characters where practical.
- Do not use formatting changes to hide an unrelated code change.
- Public APIs and externally visible behavior must use stable, precise terminology.

Required baseline checks:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy -- -D warnings
git diff --check
```

The exact command set may be refined when the workspace and embedded runner are finalized, but warnings must not be ignored without a documented reason.

## 2. Absolute no-hardcoding rule

Hardcoding is forbidden in implementation code. Hardware, configuration, deployment, and runtime values must never be embedded as unexplained literals inside functions, drivers, loaders, or applications.

The following must be represented by named constants, typed values, configuration fields, or documented board/build definitions:

- memory addresses and SRAM ranges;
- GPIO pins and peripheral identifiers;
- clock frequencies, baud rates, timeouts, and delays;
- buffer sizes, payload limits, stack sizes, and queue capacities;
- filesystem paths and package names;
- protocol versions, target IDs, ABI versions, and feature flags;
- device identifiers, retry counts, and safety thresholds.

Simple language mechanics may use literals where their meaning is unambiguous. Protocol magic values are allowed only through named constants with documentation. If a value can change between boards, builds, deployments, or runtime configurations, it must not appear as a raw implementation literal.

Every exception must be reviewed, named, documented, and justified by the relevant architecture or hardware specification.

## 3. File size and patch discipline

Always generate minimal, surgically precise Git diffs or patches. Do not rewrite entire multi-hundred-line files when changing a single function.

Large implementation files are forbidden. Rust code must be divided into small, focused modules and submodules with one clear responsibility. A Rust implementation file must not exceed 300 lines without an explicit architectural exception documented in the review.

When a file approaches the limit, split it before adding more behavior. Do not hide unrelated formatting, renaming, or refactoring inside a functional change. Preserve user changes and modify only the lines required for the task.

## 4. Additional strict constraints

### Unsafe boundaries

`unsafe` must remain centralized in `board`, `loader/exec`, or an explicitly documented low-level module. It must not spread into parsers, storage policy, SDK APIs, or ordinary runtime logic.

### Lint suppression

Do not use `#[allow(...)]` to hide warnings, safety issues, dead code, or failed lint rules. An exception must identify the exact warning, explain why it is safe, and be reviewed.

### Contract changes

Changing an ABI, AMRN field, memory layout, public API, or security claim requires an update to the relevant specification, tests, and versioning documentation in the same change.

### Dependencies

New dependencies require review of `no_std` compatibility, license, maintenance, auditability, transitive dependencies, binary size, RAM cost, and security history. Important tools and dependencies must be pinned where reproducibility matters.

### Completion claims

Compilation is not completion. Reports must distinguish source compilation, host tests, embedded target checks, hardware tests, and MVP acceptance. Do not claim hardware or acceptance completion without the required evidence.

### Tests

Do not disable, ignore, or remove tests without an issue or roadmap reference and a documented reason. “Test later” is not acceptable evidence of completion.

### Fallback behavior

Invalid configuration, missing storage, malformed packages, and hardware failures must not trigger silent fallback behavior unless that fallback is explicitly documented and safe.

### Logging and secrets

Never log credentials, keys, private data, sensitive device information, or unnecessary memory dumps.

### Change scope

Mass formatting, renaming, migration, and repository-wide rewrites are forbidden outside the explicit task scope. Preserve unrelated user changes.

### Git safety

Destructive operations such as `git reset --hard`, `git clean -fd`, force push, and bulk deletion require explicit approval.

### Function complexity

Functions should normally remain below 50 lines, use no more than three nesting levels, and perform one responsibility. Split a function when it exceeds these limits unless the exception is documented in review.

### Reproducibility

Preserve lockfiles, pin important tools and dependencies, and document commands that produce generated artifacts. A local success that cannot be reproduced in CI is not an accepted validation result.

## 5. Commenting standard

Comments must explain why code exists, which invariant it protects, or which hardware/protocol constraint applies. Comments must not merely repeat what the code already says.

Good:

```rust
// Wrapping arithmetic prevents heartbeat overflow from triggering a panic
// during long-running operation.
counter = counter.wrapping_add(1);
```

Bad:

```rust
// Increment the counter.
counter += 1;
```

Comment rules:

- Write comments as complete English sentences.
- Use punctuation consistently.
- Keep comments close to the code they explain.
- Explain safety assumptions immediately before the relevant operation.
- Explain protocol fields, hardware quirks, timing assumptions, and non-obvious invariants.
- Remove comments that become inaccurate after a code change.
- Never use comments to justify code that should instead be simplified.

The following are prohibited:

- commented-out code;
- decorative banner comments;
- stale or speculative comments;
- unexplained abbreviations;
- comments that contradict the implementation;
- untracked `TODO` items.

`TODO` is allowed only with a concrete issue or roadmap reference:

```rust
// TODO(#42): Replace polling with a DMA completion event.
```

## 6. Documentation comments

Every public type, function, constant, trait, and module must have a Rust documentation comment when its purpose or contract is not obvious from its name.

Documentation must describe observable behavior, not private implementation details. Use the appropriate sections:

- `# Errors` for fallible operations;
- `# Panics` for documented panic conditions;
- `# Safety` for unsafe functions or operations;
- hardware, timing, ownership, and blocking behavior when relevant.

## 7. `unsafe` policy

`unsafe` is forbidden by default and may be used only where a safe abstraction cannot express the hardware or execution contract.

Allowed MVP use cases are:

- memory-mapped register access;
- copying validated bytes into the reserved application SRAM region;
- transferring control through the validated application entry point;
- interrupt or vector-table operations when explicitly specified;
- unavoidable low-level HAL operations.

Every unsafe operation must:

- keep the unsafe block as small as possible;
- validate inputs before entering the unsafe block;
- add an immediate `SAFETY` comment;
- document the invariant that makes the operation valid;
- wrap repeated unsafe logic in a small safe abstraction;
- avoid exposing raw pointers across module boundaries.

Example:

```rust
// SAFETY: The loader validated that `destination` is inside the reserved
// application SRAM region and that `length` does not exceed its capacity.
unsafe {
    core::ptr::copy_nonoverlapping(source, destination, length);
}
```

Native application code is trusted and is not sandboxed or fault-isolated in the MVP.

## 8. Error handling

Runtime code must not silently discard meaningful failures.

The following are forbidden in kernel runtime paths unless a boot-time invariant is explicitly documented and reviewed:

- `unwrap()`;
- `expect()`;
- `panic!()`;
- `unreachable!()`;
- ignored `Result` values;
- converting meaningful errors into an unexplained `bool` or `None`.

Use typed error enums with enough information to identify the failed operation. Preserve the original cause where practical. Malformed SD data and AMRN packages must be rejected gracefully.

## 9. Embedded and real-time rules

- The kernel remains `#![no_std]`.
- Heap allocation is forbidden in safety-critical or real-time paths.
- Use fixed-size buffers, queues, and collections where bounded behavior is required.
- Do not place blocking SD operations in a future safety-critical control task.
- Every buffer has an explicit maximum size.
- DMA buffers have documented alignment, lifetime, and ownership rules.
- Integer overflow behavior must be intentional.
- Memory addresses, pin assignments, and peripheral constants must be named and centralized.
- Magic numbers are forbidden in implementation code.
- Timing assumptions must be documented and testable.
- Logging must not make a critical path unbounded or block indefinitely.

The MVP application has no scheduler or application-owned interrupts. Its only
service is the bounded logging ABI defined in `docs/ABI.md`.

## 10. Module and API boundaries

Each module must have one clear responsibility:

- `board`: reference-board pins, clocks, and peripheral ownership;
- `storage`: SD and filesystem access;
- `loader`: AMRN parsing, CRC32 validation, bounds checking, and execution;
- `runtime`: future tasks, scheduling, IPC, services, and watchdog policy;
- `main.rs`: bootstrap orchestration only.

Additional rules:

- Keep implementation details private.
- Use `pub` only for an intentional module contract.
- Do not use wildcard imports.
- Do not create circular module dependencies.
- The parser must not depend on hardware.
- The loader must not depend on filesystem internals.
- Hardware-specific code must not leak into package-format logic.
- Native application execution must remain separate from package parsing.

## 11. Naming and data modeling

- Types and traits use `PascalCase`.
- Functions, modules, and variables use `snake_case`.
- Constants use `SCREAMING_SNAKE_CASE`.
- Boolean names use `is_`, `has_`, `can_`, or `should_` where appropriate.
- Prefer `Sram`, `Crc32`, and `Spi` in type names over all-capital abbreviations.
- Represent protocol states and target identifiers with enums or named constants.
- Use newtypes for values whose units or meaning must not be confused.
- Do not pass raw `u32` values across an API when a named type can express the contract.

## 12. Logging standard

Logs must be written in English and remain deterministic enough for hardware diagnosis.

Use stable subsystem prefixes:

```text
[INFO][BOOT] System clock: 100 MHz
[INFO][SD] Card initialized
[INFO][AMRN] AMRN package discovered in the card root
[INFO][AMRN] CRC32 valid
[INFO][AMRN] Loading payload: 2048 bytes at 0x20008000
[INFO][AMRN] Jumping to entry point
```

Logging rules:

- use stable subsystem prefixes;
- use the logging facade rather than calling a transport backend directly;
- keep color output optional through the `log-colors` feature;
- log validation failures with a reason;
- do not hide storage or loader errors behind a generic message;
- do not log credentials, keys, or secrets;
- avoid high-frequency logs in production paths;
- keep the MVP application proof on the LED and verify the bounded logging ABI independently.

## 13. Testing requirements

Every new module must have pure-logic tests, hardware evidence, or both.

Pure-logic tests should cover valid and invalid input, boundary values, truncation, integer overflow, CRC mismatch, unsupported versions, and unsupported targets.

The AMRN loader must test valid headers, invalid magic, invalid version, unsupported target, truncated input, oversized payload, out-of-range load address, invalid execution offset, CRC32 mismatch, and valid entry-point calculation.

Hardware-dependent code requires recorded evidence containing the board, wiring, firmware revision, storage media, power source, logging channel, and observed output.

Build success alone is not hardware acceptance evidence.

## 14. Code review checklist

Before accepting a change, review:

- Does it follow the architecture and roadmap?
- Does each module keep one responsibility?
- Is every unsafe operation necessary, minimal, and documented?
- Are all external inputs validated before use?
- Are bounds and overflow checks explicit?
- Are errors preserved and reported?
- Are magic numbers replaced with named constants or types?
- Do public APIs have accurate documentation?
- Do comments explain why rather than restating what?
- Is there a test or hardware evidence entry?
- Does the change avoid unrelated refactoring?
- Do formatting, linting, and diff validation pass?

## 15. Prohibited practices

The following practices are not accepted:

- undocumented `unsafe`;
- hardcoded hardware, configuration, or runtime values;
- rewriting large files for a small change;
- monolithic Rust files above the documented size limit;
- unreviewed `#[allow(...)]` attributes;
- undocumented API, ABI, AMRN, memory-layout, or security-contract changes;
- disabled, ignored, or removed tests without a tracked reason;
- silent fallback behavior for invalid external input;
- logging secrets or sensitive device data;
- unpinned or unexplained dependencies;
- destructive Git operations without explicit approval;
- claims of hardware completion without hardware evidence;
- runtime `unwrap()` or `expect()` without a reviewed invariant;
- magic memory addresses or pin numbers;
- unbounded allocation in a real-time path;
- silent error swallowing;
- commented-out code;
- stale comments;
- untracked TODO work;
- undocumented hardware assumptions;
- mixing unrelated responsibilities in one module;
- making security claims that are not backed by an implemented mechanism and evidence.
