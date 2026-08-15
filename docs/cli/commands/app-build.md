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
- `build.target_profile` for the Rust target triple;
- `build.profile` for `dev` or `release` selection.

The generated project defaults to `thumbv7em-none-eabihf` and the `dev`
profile. The target value belongs to the application manifest; the command
does not embed a board-specific target mapping.

## Output

The command first runs Cargo with the `embedded-payload` feature, uses
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

## Failure behavior

- Missing or malformed `dali.toml` is rejected.
- Unsupported Cargo profiles are rejected; only `dev` and `release` are
  supported by this command.
- Cargo or `cargo objcopy` failures are returned with the failing command.
- The command does not flash hardware, write an SD card, or install an
  application.
