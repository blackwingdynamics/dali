# `dali app build`

## Purpose

Build a Dali application's native machine-code payload for the target and
Cargo profile declared by `dali.toml`.

## Syntax

```text
dali app build
```

The command must run from an initialized application directory. It reads:

- `application.name` for the Cargo binary name;
- `build.target_profile` for the stable profile name from `dali target list`;
- `build.profile` for `dev` or `release` selection.
- `build.abi_version` for the application package contract; when omitted, the
  target profile's ABI version is used.

The generated project defaults to `f405` and the `dev`
profile. The target value belongs to the application manifest; the command
does not embed a board-specific target mapping.

## Output

The optional build.format_version field selects the AMRN package format. ABI
v3 defaults to format version 2; setting it to 3 enables the relocatable
package pipeline.

For ABI v2, the command runs Cargo with the `embedded-payload` feature, uses
`cargo objcopy` to write the native payload, and then creates the AMRN package
from that payload. It writes:

```text
target/<target-profile>/<profile>/<application-name>.bin
target/<target-profile>/<profile>/<application-name>.amrn
```

For the default `dev` profile, Cargo uses the `debug` output directory, so the
concrete path is `target/<target-profile>/debug/<application-name>.bin`.

The `.bin` file is the intermediate native payload. The `.amrn` file is the
deployable package. `dali app package` remains available when repackaging an
existing payload without rebuilding it.

For ABI v3, the command enables the `abi-current` feature, selects `memory.v3.x`,
and extracts the linker-defined code and initialized-data sections. It writes:

```text
target/<target-profile>/<profile>/<application-name>.code.bin
target/<target-profile>/<profile>/<application-name>.data.bin
target/<target-profile>/<profile>/<application-name>.amrn
```

The package records the linker-defined zero-initialized data size and the
target manifest's PSP stack reservation. ABI v3 output is currently suitable
for host inspection and contract testing only; kernel launch support is a
separate roadmap task.

When build.format_version is 3, the command retains supported ARM relocation
records from the linked ELF and emits an AMRN format version 3 package. This
package is host-inspectable but is not accepted by the kernel loader yet.

## Failure behavior

- Missing or malformed `dali.toml` is rejected.
- Unsupported Cargo profiles are rejected; only `dev` and `release` are
  supported by this command.
- Cargo or `cargo objcopy` failures are returned with the failing command.
- The command does not flash hardware, write an SD card, or install an
  application.
