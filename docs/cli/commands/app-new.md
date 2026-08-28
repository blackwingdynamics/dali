# `dali app new`

## Purpose

Create a new Dali native application project from the versioned application
scaffold.

## Status

Implemented for the initial scaffold contract. The command creates source and
configuration files but does not build, cartridge, flash, or install an
application.

## Syntax

```text
dali app new <name> [--sdk-path <path>]
```

The project is created as `<current-directory>/<name>`. When run inside a Dali
workspace, the workspace-owned `dali` SDK is discovered automatically. When run
elsewhere, provide a local SDK checkout explicitly:

```text
dali app new telemetry --sdk-path /path/to/dali-kernel/crates/dali-sdk
```

The generated manifest stores a relative SDK path, not the absolute path.

## Generated files

See [the application project contract](../APPLICATION_PROJECT.md). The
scaffold contains a Cargo manifest, `dali.toml`, linker script, build script,
library boundary, and native ABI entry point.

## Failure behavior

- Invalid names are rejected before filesystem creation.
- An existing target path is never overwritten.
- An external project without `--sdk-path` is rejected explicitly.
- The SDK path must point to a Cargo crate named `dali`.
- Filesystem failures identify the affected path.

## Example

```text
dali app new telemetry
cd telemetry
```
