# Dali CLI Documentation

Dali is the host-side command-line tool for creating and validating Dali OS
application cartridges. The Cargo package is named dali-cli; the installed
executable is named dali.

## Supported commands

| Command | Purpose | Status |
| --- | --- | --- |
| dali doctor | Check host toolchain and embedded build prerequisites | Supported |
| dali target list | List supported Dali application targets | Supported |
| dali target info | Display target profile metadata | Supported |
| dali target scaffold | Generate a reviewable target scaffold | Supported |
| dali package | Wrap a linked native payload in an AMRN cartridge | Supported |
| dali inspect | Validate and display an AMRN cartridge | Supported |
| dali key generate | Generate an Ed25519 release trust-anchor pair | Supported |
| dali app new | Create a Dali application scaffold | Supported |
| dali app init | Initialize the current directory as a Dali application | Supported |
| dali app build | Build a native application payload | Supported |
| dali app package | Package the built payload as AMRN | Supported |
| dali device list | Discover host-visible devices | Supported |
| dali device info | Display selected device and target metadata | Supported |
| dali device attach | Start a debug-probe attachment | Supported |
| dali device console | Open a runtime USB CDC console | Supported |
| dali device flash | Flash firmware through a declared transport | Supported |
| dali metadata | Create and verify repository metadata | Supported |

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

## Reproducible workflow map

Use these procedures as the canonical examples for the complete host and
device workflow:

- [Installation](INSTALLATION.md) — install and verify the CLI.
- [Quickstart](QUICKSTART.md) — create, build, package, and inspect a cartridge.
- [Key management](KEY_MANAGEMENT.md) — generate and use signing material.
- [Binary v2 developer workflow](BINARY_V2_DEVELOPER_WORKFLOW.md) — authorize,
  sign, verify, copy, flash, and capture device evidence.
- [Device discovery](DEVICE_DISCOVERY.md) and [device commands](COMMANDS.md) —
  identify a transport and select a console or flashing operation.

Every example separates host artifact validation from embedded compilation,
flashing, and Silicon Trace acceptance. The CLI cannot turn a successful host
operation into hardware evidence.

## Documentation map

- [Installation](INSTALLATION.md) — install and verify the executable.
- [Quickstart](QUICKSTART.md) — first successful package workflow.
- [Commands](COMMANDS.md) — command catalog and syntax.
- [Doctor](commands/doctor.md) — host and toolchain diagnostics.
- [Target list](commands/target-list.md) — supported target profiles.
- [Target info](commands/target-info.md) — complete target metadata and machine-readable fields.
- [Target scaffold](commands/target-scaffold.md) — reviewable board backend scaffold generation.
- [Metadata workflows](BINARY_V2_DEVELOPER_WORKFLOW.md) — repository metadata and authorization workflow.
- [Workflows](WORKFLOWS.md) — end-to-end user procedures.
- [Output](OUTPUT.md) — output and stream conventions.
- [Errors](ERRORS.md) — failure categories and recovery guidance.
- [Exit codes](EXIT_CODES.md) — scripting and CI contract.
- [Compatibility](COMPATIBILITY.md) — CLI, AMRN, target, and ABI compatibility.
- [Key management](KEY_MANAGEMENT.md) — release trust anchors, signing keys,
  generation, provisioning, rotation, and incident response.
- [Troubleshooting](TROUBLESHOOTING.md) — common host-side problems.
- [Testing](TESTING.md) — CLI validation strategy.
- [Contributing](CONTRIBUTING.md) — rules for extending the CLI.
- [Application project contract](APPLICATION_PROJECT.md) — scaffold and manifest contract.
- [Device discovery](DEVICE_DISCOVERY.md) — transport-neutral discovery contract.
- [Device info](commands/device-info.md) — selected device and target metadata.
- [Device attach](commands/device-attach.md) — debug-probe attachment.
- [Device console](commands/device-console.md) — runtime CDC console launcher.
- [Device flash](commands/device-flash.md) — manifest-driven firmware flashing through a declared transport.
- [ABI v3 fault tests](ISOLATION_FAULT_TEST.md) — non-production F405 kernel-memory read and write fault injection.

Command-specific documentation lives in [commands/](commands/):

- [package](commands/package.md)
- [inspect](commands/inspect.md)
- [app new](commands/app-new.md)
- [app init](commands/app-init.md)
- [app build](commands/app-build.md)
- [app package](commands/app-package.md)
- [target info](commands/target-info.md)
- [target scaffold](commands/target-scaffold.md)

## Scope boundary

The current CLI does not mount storage, manage applications, or provide an
interactive terminal UI. Probe flashing requires a separate transport
operation and is not implied by the DFU command.

Application scaffolding is available through `dali app new`, both inside a
Dali workspace and outside it with an explicit `--sdk-path`. Initializing an
existing directory is available through `dali app init`; it never overwrites
existing managed files.

`dali inspect` can derive the current application's AMRN path when run from
its project root, or discover a single AMRN file in a package directory.

Target and board metadata is declared in repository-level `targets/*.toml`
manifests. The `dali-targets` build script validates those manifests and
generates the typed registry consumed by CLI commands. CLI commands therefore
do not define one set of board constants per command; kernel backends remain
responsible for mapping the selected profile to typed HAL resources.
