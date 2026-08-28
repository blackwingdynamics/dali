# Codebase Refactoring and Hardware Evidence Gates

Status: **Active — preparation and inventory in progress; hardware evidence
required for every checkpoint**.

The first active checkpoint is the baseline inventory and sequencing review.
No implementation file is marked complete until its individual hardware gate
has been reviewed and accepted.

This work track reorganizes oversized or mixed-responsibility implementation
files without changing runtime behavior. It is subordinate to the existing
architecture, boot order, public APIs, generated output, target profiles, and
frozen subsystem boundaries.

## Non-negotiable completion rule

Every file is one independent checkpoint. A checkpoint cannot be marked
complete, merged, or described as safe until all of the following evidence is
recorded for that exact change:

1. the pre-change revision and file diff are identified;
2. focused host tests and the relevant embedded checks pass;
3. the F405 firmware is built from the changed revision;
4. the firmware is flashed and booted on the supported F405 board;
5. the existing boot, storage, loader, scheduler, and diagnostic trace is
   compared with the pre-change baseline;
6. the board remains operational and no regression is observed in the
   affected path.

The hardware gate is evidence-based, not inferred from compilation or host
tests. If laboratory equipment is required for a particular peripheral, the
checkpoint remains open until the required F405 trace is available. No
refactoring batch may combine multiple unchecked files.

## Frozen boundaries

The following paths are excluded from this refactoring track unless a
separate architecture approval explicitly changes their status:

- `kernel/src/storage/`;
- `kernel/src/platform/f405/sdio_raw/`;
- USB CDC core servicing and polling loops;
- SPI display and ILI9341 experiments;
- `crates/dali-targets/src/lib.rs`, which remains the stable public typed
  profile facade.

Refactoring must not alter storage layout, loader mode, ABI, memory map,
feature contracts, target manifest semantics, or hardware ownership.

## Checkpoint protocol

For each file below, create a focused structural change, then stop for review
and hardware validation before starting the next file. The checkpoint record
must include:

- source revision and changed file list;
- moved symbols and preserved public API;
- host and embedded command output;
- firmware artifact identity;
- board, power source, wiring, and console transport;
- expected and observed boot trace;
- result, limitations, and reviewer approval.

The minimum software validation is:

```text
cargo fmt --all -- --check
cargo check --workspace --exclude dali-kernel
cargo check-kernel
cargo test -p <affected-crate>
cargo clippy --workspace --all-targets --exclude dali-kernel -- -D warnings
git diff --check
```

The exact target build, flash command, and trace command must be recorded in
the checkpoint. A hardware failure or unavailable instrument blocks that
checkpoint; it must not be hidden by a successful host build.

## Priority 1 — mandatory checkpoints

- [ ] `kernel/src/loader/repository/streaming.rs` (857 lines): separate the
  bounded stream state machine, envelope/record parsing, target selection, and
  replay/verification helpers.
- [ ] `crates/dali-cli/src/commands/inspect.rs` (534 lines): separate command
  input handling, cartridge inspection, and output rendering.
- [ ] `crates/dali-cli/src/commands/app/build.rs` (502 lines): separate build
  request parsing, manifest validation, linker preparation, and artifact
  assembly.
- [ ] `crates/dali-driver-api/tests/driver_contracts.rs` (459 lines): group
  contract tests by driver while preserving the test names and coverage.
- [ ] `crates/dali-metadata/src/parser/streaming/delegation.rs` (438 lines):
  separate delegation field parsing, bounded collection, and finalization.
- [ ] `kernel/src/loader/pipeline/signed.rs` (427 lines): separate signed
  cartridge reading, authentication/CRC validation, memory copying, and
  relocation application.
- [ ] `crates/dali-cli/src/commands/app/package.rs` (407 lines): separate
  package input preparation, metadata handling, and output writing.

## Priority 2 — parser, metadata, and runtime checkpoints

- [ ] `crates/dali-metadata/src/parser/streaming/mod.rs` (386 lines)
- [ ] `crates/dali-metadata/src/codec/binary/mod.rs` (376 lines)
- [ ] `crates/dali-metadata/src/parser/streaming/snapshot.rs` (370 lines)
- [ ] `kernel/src/runtime/scheduling/scheduler.rs` (360 lines)
- [ ] `crates/dali-metadata/src/parser/streaming/root.rs` (354 lines)
- [ ] `crates/dali-metadata/src/chain.rs` (349 lines)
- [ ] `kernel/src/loader/pipeline/execution.rs` (334 lines)
- [ ] `kernel/src/runtime/application/owner.rs` (332 lines)
- [ ] `crates/dali-cli/src/commands/metadata/bundle/common.rs` (329 lines)
- [ ] `crates/dali-metadata/src/parser/streaming/chain.rs` (313 lines)
- [ ] `kernel/src/loader/pipeline/identity.rs` (311 lines)
- [ ] `crates/dali-metadata/src/streaming.rs` (310 lines)
- [ ] `crates/dali-metadata/src/parser/streaming/revocation.rs` (307 lines)
- [ ] `kernel/src/security/fault/mod.rs` (302 lines)
- [ ] `crates/dali-amrn/src/v2/mod.rs` (302 lines)

## Priority 3 — boundary review checkpoints

These files are at or near the normal implementation-file limit and require
responsibility review before any split is approved:

- [ ] `kernel/src/loader/contract/catalog.rs` (300 lines)
- [ ] `crates/dali-amrn/src/v3/apply.rs` (299 lines)
- [ ] `crates/dali-metadata/src/parser/streaming/bundle/parser.rs` (299 lines)
- [ ] `kernel/src/loader/pipeline/relocation.rs` (296 lines)
- [ ] `kernel/src/loader/repository/chain/types.rs` (286 lines)
- [ ] `kernel/src/platform/mod.rs` (284 lines)
- [ ] `crates/dali-metadata/src/authorization.rs` (284 lines)

Each review checkpoint may be closed without splitting if the file has one
coherent responsibility and the review record explains why. If it is split,
the same hardware gate applies.

## Completion criteria

- [ ] Every approved split preserves public APIs and generated output.
- [ ] Every approved split preserves boot order, storage behavior, loader
  behavior, scheduler behavior, and trace markers.
- [ ] No frozen path is changed.
- [ ] No new hardcoded hardware, timing, buffer, or deployment value is
  introduced.
- [ ] Every checkpoint has its own software validation and F405 hardware
  evidence record.
- [ ] Final repository tree and documentation inventory match the actual
  source tree.
