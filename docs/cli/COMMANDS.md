# Dali CLI Commands

## Command syntax

The executable is dali. When running from the repository without installing
it, use Cargo's package and binary selectors:

~~~text
cargo run -p dali-cli --bin dali -- <command> <options>
~~~

## Available commands

### dali doctor

Checks the host toolchain, embedded target, required build tools, and optional
hardware tools.

See [commands/doctor.md](commands/doctor.md).

### dali target list

Lists the current Dali application target profiles and their AMRN/ABI
compatibility metadata.

The command reads the generated registry built from the repository's
`targets/*.toml` manifests. The profile name shown here is the value used in
an application's `dali.toml`; the registry resolves it to the Rust target and
the remaining board metadata during the build.

See [commands/target-list.md](commands/target-list.md).

### dali target scaffold

Generates a non-production board backend template and documentation checklist
from an existing target profile. The command refuses to overwrite files and
does not modify board module registration.

~~~text
dali target scaffold <profile> [--output <workspace-root>]
~~~

See [commands/target-scaffold.md](commands/target-scaffold.md).

### dali target info

Resolves machine-readable transport metadata from a target profile. The field
form is used by repository workflows so chip identifiers remain in manifests.

~~~text
dali target info <profile> --field probe-chip
~~~

### dali package

Creates an AMRN package from a linked native payload.

See commands/package.md.

### dali inspect

Validates an AMRN package and prints its decoded fields without modifying the
input.

See commands/inspect.md.

## Application commands

The following commands are supported:

```text
dali app new <name> [--sdk-path <path>]
dali app init [--sdk-path <path>]
dali app build
dali app package
```

See [commands/app-new.md](commands/app-new.md) and [Application project
contract](APPLICATION_PROJECT.md) for the scaffold, manifest, and overwrite
policy. See [commands/app-init.md](commands/app-init.md) for initialization
of an existing directory, [commands/app-build.md](commands/app-build.md) for
native payload builds, and [commands/app-package.md](commands/app-package.md)
for application-aware packaging.

There is no `dali app inspect` command. AMRN validation is provided by the
generic `dali inspect` command because it validates a package artifact rather
than application project configuration.
The `app package` command creates the AMRN artifact from the payload path
derived from the same manifest.

## Unsupported commands

Unknown commands fail with usage information. Hardware flashing, SD-card
installation, device discovery, package listing, and interactive terminal
operations are not currently CLI commands.
