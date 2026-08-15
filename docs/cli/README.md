# Dali CLI Documentation

Dali is the host-side command-line tool for creating and validating Dali OS
application packages. The Cargo package is named dali-cli; the installed
executable is named dali.

## Supported commands

| Command | Purpose | Status |
| --- | --- | --- |
| dali package | Wrap a linked native payload in an AMRN package | Supported |
| dali inspect | Validate and display an AMRN package | Supported |
| dali app new | Create a Dali application scaffold | Supported |
| dali app init | Initialize the current directory as a Dali application | Supported |

## Documentation map

- [Installation](INSTALLATION.md) — install and verify the executable.
- [Quickstart](QUICKSTART.md) — first successful package workflow.
- [Commands](COMMANDS.md) — command catalog and syntax.
- [Workflows](WORKFLOWS.md) — end-to-end user procedures.
- [Output](OUTPUT.md) — output and stream conventions.
- [Errors](ERRORS.md) — failure categories and recovery guidance.
- [Exit codes](EXIT_CODES.md) — scripting and CI contract.
- [Compatibility](COMPATIBILITY.md) — CLI, AMRN, target, and ABI compatibility.
- [Troubleshooting](TROUBLESHOOTING.md) — common host-side problems.
- [Testing](TESTING.md) — CLI validation strategy.
- [Contributing](CONTRIBUTING.md) — rules for extending the CLI.
- [Application project contract](APPLICATION_PROJECT.md) — scaffold and manifest contract.

Command-specific documentation lives in [commands/](commands/):

- [package](commands/package.md)
- [inspect](commands/inspect.md)
- [app new](commands/app-new.md)
- [app init](commands/app-init.md)

## Scope boundary

The current CLI does not flash hardware, mount storage, manage applications,
or provide an interactive terminal UI. Those capabilities require separate
contracts and are not implied by the current commands.

Application scaffolding is available through `dali app new`, both inside a
Dali workspace and outside it with an explicit `--sdk-path`. Initializing an
existing directory is available through `dali app init`; it never overwrites
existing managed files.
