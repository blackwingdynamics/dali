# Contributing to Dali OS

Dali OS is a safety-oriented embedded Rust platform. Contributions must prioritize predictable behavior, explicit contracts, reviewability, and reliable evidence. All issues, code, comments, documentation, logs, errors, commit messages, and pull requests must be written in English.

## Before starting

Read:

1. [Architecture](docs/ARCHITECTURE.md)
2. [Coding Standards](docs/CODING_STANDARDS.md)
3. [Roadmap](docs/ROADMAP.md)
4. [Testing](docs/TESTING.md)
5. [Development](docs/DEVELOPMENT.md)

Confirm that the change belongs to the current roadmap phase. If it changes the architecture, AMRN format, ABI, memory layout, hardware assumptions, or security model, update the relevant documentation before changing code.

## Issues and tasks

Before implementation:

1. Check for an existing issue or roadmap task.
2. Define one narrow objective.
3. State expected behavior and acceptance evidence.
4. Identify affected modules and documents.
5. Split broad work into atomic tasks.

Every task should state what changes, which module owns it, what failures exist, how it will be tested, whether hardware is required, and whether a public contract changes.

## Branches

Use a focused branch from the current integration branch:

```text
feat/amrn-header-parser
fix/sd-init-timeout
docs/contribution-guide
test/crc32-boundaries
refactor/board-bootstrap
```

Use lowercase kebab-case. Keep one objective per branch, avoid unrelated refactoring, and do not rewrite shared history without explicit agreement.

## Commit messages

Commits must follow Conventional Commits:

```text
<type>(<scope>): <imperative summary>
```

Examples:

```text
feat(loader): validate AMRN payload bounds
fix(storage): reject truncated SD blocks
test(loader): cover CRC32 mismatch handling
docs(architecture): define the MVP SRAM layout
refactor(board): isolate STM32F411 pin setup
build(deps): pin embedded-fatfs version
chore(ci): run clippy with warnings denied
```

Allowed types are `feat`, `fix`, `test`, `docs`, `refactor`, `build`, `chore`, and `perf`.

Commit rules:

- use an imperative summary such as `add`, `fix`, or `define`;
- keep the subject preferably under 72 characters;
- do not end the subject with a period;
- use a specific scope such as `loader`, `storage`, `board`, `docs`, or `ci`;
- explain non-obvious motivation in the body;
- reference issues when applicable;
- never include credentials, generated secrets, or unrelated files.

## Atomic commits

Each commit must be independently understandable and, where practical, buildable. Do not combine formatting with behavior, unrelated dependency cleanup with a feature, or documentation for one feature with code for another.

Good sequence:

```text
docs(amrn): define the fixed header layout
test(loader): add header parser cases
feat(loader): implement AMRN header parsing
```

Keep generated binaries out of commits unless the repository explicitly requires them.

## Code and documentation requirements

All code must comply with [CODING_STANDARDS.md](docs/CODING_STANDARDS.md).

In particular:

- use English for source, comments, logs, errors, and documentation;
- document every unsafe operation with an immediate `SAFETY` comment;
- validate all external input before use;
- use explicit typed errors;
- avoid magic addresses, pin numbers, and sizes;
- keep realtime paths bounded;
- document public APIs;
- preserve module boundaries.

Update documentation in the same change when behavior or a contract changes, including architecture, AMRN fields, ABI, memory layout, hardware pins, security claims, build procedures, or acceptance criteria.

For the MVP, do not introduce sandboxing, dynamic linking, relocation, application-owned interrupts, or SDK APIs without updating the architecture and roadmap first.

## Validation

Run the applicable checks before opening a pull request:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy -- -D warnings
git diff --check
```

For embedded changes, verify the target build and record the exact command and result. For hardware-dependent changes, record:

- board and MCU;
- wiring and peripherals;
- firmware revision;
- SD-card type and filesystem;
- power source;
- logging channel;
- expected and observed output;
- whether the test passed, failed, or was not run.

Build success is not hardware acceptance evidence. Parser tests do not prove that a package executed on the MCU.

Loader changes must include boundary tests for malformed headers, truncation, invalid targets, oversized payloads, invalid addresses, invalid entry offsets, and CRC32 mismatches where applicable.

## GitHub CI and branch protection

GitHub Actions runs on pushes to the integration branches and on pull requests. The workflow checks formatting, host workspace code, host tests, host Clippy, the STM32F405 kernel target, embedded Clippy, the kernel build, and repository whitespace.

Maintainers must configure branch protection so these checks are required before merging:

- `Formatting`;
- `Host workspace checks`;
- `STM32F405 embedded checks`;
- `Repository hygiene`.

The workflow reports failures, but GitHub branch protection is what prevents a failed pull request from being merged. Contributors must not bypass a failed required check without a documented maintainer decision.

## Pull requests

A pull request must include:

- a clear Conventional Commit-style title;
- the problem and solution;
- affected modules;
- documentation changes or a reason none are required;
- validation commands and results;
- hardware evidence or a statement that hardware testing was not required;
- known limitations and follow-up tasks;
- a linked issue or roadmap task when available.

Recommended structure:

```text
## Summary

## Scope

## Validation

## Hardware evidence

## Limitations
```

Keep pull requests small enough to review and split unrelated work into separate pull requests.

## Review policy

Reviewers check architectural alignment, correctness, failure handling, memory bounds, overflow behavior, unsafe justification, realtime behavior, test coverage, hardware evidence, documentation accuracy, dependency impact, and scope.

A pull request may be rejected for undocumented unsafe code, hidden failures, unsupported security claims, missing contract documentation, unrelated changes, or insufficient evidence.

Review feedback must be specific and respectful. Resolve feedback with a new commit or a clear response; do not silently ignore safety or correctness concerns.

## Dependencies and generated files

New dependencies require justification covering `no_std` compatibility, license, maintenance, auditability, transitive impact, memory cost, and binary-size cost. Cryptography and filesystem implementations should use suitable reviewed dependencies when their constraints and licenses fit the project.

Generated binaries and packages may be committed only when explicitly required. Document their generation command and keep the source process reproducible. Never commit secrets, private keys, device credentials, or sensitive logs.

## Security disclosures

Report suspected vulnerabilities privately to the maintainers when a private channel is available. Do not publish exploit details before assessment. Never include secrets or production device data in issues, commits, pull requests, or fixtures.

## Contributor checklist

- [ ] The change has one clear objective.
- [ ] Architecture, roadmap, and coding standards were reviewed.
- [ ] Code, comments, logs, and errors are in English.
- [ ] Every unsafe operation has immediate safety documentation.
- [ ] Errors and external inputs are handled explicitly.
- [ ] Tests or hardware evidence were added or recorded.
- [ ] Documentation was updated when a contract changed.
- [ ] Commit messages follow Conventional Commits.
- [ ] No unrelated changes are included.
- [ ] Required validation commands pass.
- [ ] No secrets or unreviewed generated artifacts are included.
