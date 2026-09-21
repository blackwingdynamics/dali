# Development Operations and Release Workflow

This document is the navigation point for repeatable repository operations.
It connects onboarding, local setup, validation, release preparation,
rollback, and troubleshooting without changing the runtime or hardware
contract.

## Onboarding

1. Read the repository [contribution rules](../../CONTRIBUTING.md) and the
   [coding standards](../coding-standards/README.md).
2. Read the [architecture](../architecture/README.md), [roadmap](../roadmap/README.md),
   and [testing](../testing/README.md) boundaries.
3. Install the toolchain and host dependencies described in [setup and
   console](setup-and-console.md).
4. Run the workspace and kernel checks from [workspace and
   commands](workspace-and-commands.md).
5. For hardware work, review the [hardware](../hardware/README.md) procedure
   and record the required evidence before making an acceptance claim.

## Development setup and validation

The canonical local commands are maintained in [workspace and
commands](workspace-and-commands.md), [build, flash, and
simulation](build-flash-and-simulation.md), and [testing](../testing/README.md).
Keep host tests, embedded target checks, flashing, debugger inspection, and
Silicon Trace as separate validation layers.

## Release preparation

Before a release or external evaluation:

- run the applicable formatting, workspace, kernel, test, and documentation
  checks;
- run `just docs-format` when Markdown formatting changes are needed, then
  confirm the result with `just docs-format-check` and `just docs-check`;
- inspect the staged file list and `git diff --check` output;
- review version, ABI, AMRN, target-profile, security, and evidence changes;
- follow the [release checklist](../versioning/release-checklist.md) and
  [release tag policy](../versioning/release-tags-and-breaking-changes.md);
- ensure generated changelogs are produced by the configured release tooling
  rather than edited manually.

Release preparation does not promote host or embedded validation into hardware
acceptance. Physical claims must remain supported by the recorded Silicon
Trace for the relevant target and firmware.

## Rollback and recovery

Rollback is a documented decision, not an automatic assumption. First identify
the affected revision, artifact, target profile, and validation boundary.
Then stop distribution of the affected artifact, preserve the evidence, and
select a previously validated revision or recovery procedure documented by the
relevant subsystem.

For cartridge or repository failures, use the [cartridge distribution
procedures](../cartridge-distribution/README.md) and preserve the trust,
generation, and recovery records. For target boot or transport failures, use
the [build, flash, and simulation](build-flash-and-simulation.md) and
[debugging and troubleshooting](debugging-and-troubleshooting.md) procedures.
Do not erase storage, replace metadata, or reflash a target without confirming
the target and artifact scope.

## Troubleshooting

Start with the narrowest relevant procedure:

- host command or artifact issue: [CLI troubleshooting](../cli/TROUBLESHOOTING.md);
- build, flash, or simulator issue: [build, flash, and simulation](build-flash-and-simulation.md);
- debugger, probe, or console issue: [debugging and troubleshooting](debugging-and-troubleshooting.md);
- acceptance or evidence issue: [testing documentation](../testing/README.md).

Record the command, revision, target/profile, observed output, and limitation.
Do not infer hardware behavior from a host error, a successful flash, or device
enumeration alone.
