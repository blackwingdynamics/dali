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

### dali device list

Lists devices visible through the supported host transports without changing
device state:

~~~text
dali device list
~~~

See [DEVICE_DISCOVERY.md](DEVICE_DISCOVERY.md) for the normalized record and
failure contract.

### dali device info

Displays one discovered device and the generated target metadata associated
with it:

~~~text
dali device info <id-or-path>
~~~

The selector may be the transport-provided device `id` shown by
`dali device list`, or the device path when the transport exposes one. The
command is read-only and does not reset, attach, flash, or open the device.

### dali device console

Opens the runtime USB CDC console with the host `picocom` command. With no
port argument, exactly one visible CDC console must be discoverable:

~~~text
dali device console
dali device console --port /dev/ttyACM0
~~~

The explicit form is useful when more than one CDC device is connected. This
command does not flash, reset, or identify a target; it only opens the selected
host device path.

### dali device flash

Flashes an explicit firmware file through the target's declared DFU transport:

~~~text
dali device flash f405
dali device flash f405 --input custom.bin
dali device flash f405 --transport probe
dali device flash --transport dfu --target f405 --input <firmware>
~~~

The short form resolves the conventional kernel artifact from the target
manifest and workspace build directory. The explicit form remains available
for custom paths. The target manifest supplies the DFU identity, alternate
interface, flash address, and post-download transition. Probe flashing uses
the target's declared probe chip and ELF artifact. The command does not build
the input or choose a target automatically.

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

Displays complete metadata from a target profile:

~~~text
dali target info <profile>
~~~

For repository workflows, the field form resolves machine-readable transport
metadata so chip identifiers remain in manifests:

~~~text
dali target info <profile> --field probe-chip
~~~

The human-readable form reports the board, MCU, Rust target, probe identifier,
application and AMRN compatibility, clock, memory, LED, USB, and storage
metadata. The `probe-chip` form prints only the probe identifier.

See [commands/target-info.md](commands/target-info.md).

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

Unknown commands fail with usage information. SD-card and package installation
are not currently implemented. Probe flashing is not implemented yet. The
device discovery contract is defined separately in
[DEVICE_DISCOVERY.md](DEVICE_DISCOVERY.md).
