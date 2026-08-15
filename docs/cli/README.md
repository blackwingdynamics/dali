# Dali CLI Documentation

Dali is the host-side command-line tool for creating and validating Dali OS
application packages. The Cargo package is named dali-cli; the installed
executable is named dali.

## Supported commands

| Command | Purpose | Status |
| --- | --- | --- |
| dali doctor | Check host toolchain and embedded build prerequisites | Supported |
| dali package | Wrap a linked native payload in an AMRN package | Supported |
| dali inspect | Validate and display an AMRN package | Supported |
| dali app new | Create a Dali application scaffold | Supported |
| dali app init | Initialize the current directory as a Dali application | Supported |
| dali app build | Build a native application payload | Supported |
| dali app package | Package the built payload as AMRN | Supported |

## Command layers

The CLI has two deliberately different command layers:

### Application workflow

Use `dali app ...` commands from an initialized application project. These
commands read `dali.toml` and derive artifact names and paths automatically:

```text
dali app new <name> [--sdk-path <path>]
dali app init [--sdk-path <path>]
dali app build
dali app package
```

`dali app build` produces both the native `.bin` payload and the `.amrn`
package. `dali app package` repackages an existing payload without rebuilding
it.

There is currently no separate `dali app inspect` command. Use the generic
`dali inspect` command to validate the AMRN artifact.

### Generic package operations

Use the top-level commands when working with raw artifacts or paths outside an
application project. They do not require application project context:

```text
dali package --input <payload> --output <package> --entry-offset <bytes>
dali inspect --input <package>
```

`dali inspect` also supports discovery without `--input` when run from an
application root or a directory containing exactly one `.amrn` file. An
explicit `--input` is required for another location or when multiple packages
exist.

## Documentation map

- [Installation](INSTALLATION.md) — install and verify the executable.
- [Quickstart](QUICKSTART.md) — first successful package workflow.
- [Commands](COMMANDS.md) — command catalog and syntax.
- [Doctor](commands/doctor.md) — host and toolchain diagnostics.
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
- [app build](commands/app-build.md)
- [app package](commands/app-package.md)

## Scope boundary

The current CLI does not flash hardware, mount storage, manage applications,
or provide an interactive terminal UI. Those capabilities require separate
contracts and are not implied by the current commands.

Application scaffolding is available through `dali app new`, both inside a
Dali workspace and outside it with an explicit `--sdk-path`. Initializing an
existing directory is available through `dali app init`; it never overwrites
existing managed files.

`dali inspect` can derive the current application's AMRN path when run from
its project root, or discover a single AMRN file in a package directory.
