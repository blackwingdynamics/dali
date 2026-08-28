# 1. Language and formatting

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
