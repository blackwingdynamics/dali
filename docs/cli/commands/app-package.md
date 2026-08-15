# `dali app package`

## Purpose

Create the AMRN deployment package from the native payload produced by
`dali app build`.

This is the application-aware packaging command. It reads `dali.toml` and
derives the payload and package paths. For raw files outside an application
project, use the generic `dali package` command instead.

## Syntax

```text
dali app package
```

The command reads `dali.toml`, derives the target/profile payload path, reads
`build.entry_offset`, and writes the package beside the payload:

```text
target/<target-profile>/debug/<application-name>.amrn
```

The `release` profile uses the `release` directory instead of `debug`.

## Workflow

```text
dali app build
dali inspect --input target/thumbv7em-none-eabihf/debug/telemetry.amrn
```

`dali app build` normally creates both artifacts. Use `dali app package` when
the native payload already exists and only packaging must be repeated.

## Failure behavior

- Missing or invalid `dali.toml` is rejected.
- A missing native payload reports its derived path.
- Invalid AMRN metadata is rejected by the shared AMRN encoder.
- The command does not flash hardware or write an SD card.
