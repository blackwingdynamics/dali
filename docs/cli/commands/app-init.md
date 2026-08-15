# `dali app init`

## Purpose

Initialize the current directory as a Dali native application project without
overwriting existing managed files.

## Syntax

```text
dali app init [--sdk-path <path>]
```

The application name is derived from the current directory name. When run
inside a Dali workspace, the local SDK is discovered automatically. When run
elsewhere, provide a local SDK checkout explicitly:

```text
dali app init --sdk-path /path/to/dali-kernel/crates/dali-sdk
```

## Safety and failure behavior

- Existing files are not overwritten.
- The command preflights all managed paths before creating any files.
- Invalid directory names are rejected before filesystem mutation.
- The SDK path must point to a Cargo crate named `dali`.
- The command does not build, package, flash, or install the application.

## Example

```text
mkdir telemetry
cd telemetry
dali app init --sdk-path /path/to/dali-kernel/crates/dali-sdk
```

The generated files are defined by the [application project
contract](../APPLICATION_PROJECT.md).
