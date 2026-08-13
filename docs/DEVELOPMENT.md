# Development Workflow

## Prerequisites

- Rust toolchain with the embedded target installed;
- Lefthook 1.7 or newer;
- `git-cliff` for local changelog previews;
- an SWD programmer/debug probe;
- a WeAct BlackPill STM32F411 board;
- a correctly wired 3.3 V SD-card interface;
- a supported RTT viewer.

## Workspace and Git hooks

The repository is a Cargo workspace containing the kernel, the future `dali-sdk`, the future `dali-cli`, and the initial demo-application scaffold. Run workspace commands from the repository root.

The embedded target is selected explicitly for kernel commands so host-side SDK and CLI tooling can be checked normally:

```text
cargo check-kernel
cargo build-kernel
```

Install the Git hooks after cloning:

```text
lefthook install
```

The pre-commit hook runs formatting, workspace checks, Clippy with warnings denied, workspace tests, and staged-diff validation. The commit-msg hook enforces the Conventional Commit format.

## Continuous integration

GitHub Actions runs the same validation layers on pushes and pull requests:

- Rust formatting;
- host workspace checks, tests, and Clippy;
- STM32F411 target checks, Clippy, and kernel build;
- repository whitespace validation.

The `Formatting`, `Host workspace checks`, `STM32F411 embedded checks`, and `Repository hygiene` jobs must be configured as required status checks in GitHub branch protection before merging is technically blocked.

## Build sequence

1. Format and lint the Rust workspace.
2. Build the kernel for `thumbv7em-none-eabihf`.
3. Build the demo application for the same target.
4. Assemble the `.amrn` package using the documented format.
5. Copy the package to the SD-card root.
6. Flash the kernel image.
7. Reset the board and capture the complete log.

The exact commands belong here once the workspace layout and runner are finalized. A successful host build alone is not hardware evidence.

Preview generated release notes locally with:

```text
git-cliff --unreleased
```

## Debugging rules

- keep the boot log deterministic;
- log validation failures with a reason;
- never hide SD or loader errors behind a generic panic during bring-up;
- record the exact package bytes used for an acceptance test;
- keep application and kernel linker layouts under version control.
