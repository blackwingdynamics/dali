# `dali app cartridge`

## Purpose

Create the AMRN deployment cartridge from the native payload produced by
`dali app build`.

This is the application-aware packaging command. It reads `dali.toml` and
derives the payload and cartridge paths. For raw files outside an application
project, use the generic `dali cartridge` command instead.

## Syntax

```text
dali app cartridge
```

The command reads `dali.toml`, derives the target/profile artifact paths, reads
`build.entry_offset`, and writes the cartridge beside the artifacts:

```text
target/<target-profile>/debug/<application-name>.amrn
```

The `release` profile uses the `release` directory instead of `debug`.

ABI v2 reads `<application-name>.bin`. ABI v3 reads the generated
`<application-name>.code.bin` and `<application-name>.data.bin` sections and
recomputes the zero-initialized data size from the linked symbols.

## Workflow

ABI v3 defaults to AMRN format version 2. Setting build.format_version to 3
reads retained ARM relocation records from the linked ELF and emits AMRN
format version 3. The kernel loader does not accept that format yet.

```text
dali app build
dali inspect --input target/thumbv7em-none-eabihf/debug/telemetry.amrn
```

`dali app build` normally creates both artifacts. Use `dali app cartridge` when
the native payload already exists and only packaging must be repeated.

## Failure behavior

- Missing or invalid `dali.toml` is rejected.
- A missing native payload reports its derived path.
- Invalid AMRN metadata is rejected by the shared AMRN encoder.
- The command does not flash hardware or write an SD card.
