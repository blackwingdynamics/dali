# `dali target info`

## Purpose

`dali target info` displays the declarative metadata for one target profile.
It is a host-only diagnostic command and does not connect to a board, probe,
DFU device, or runtime console.

## Syntax

```text
dali target info <profile>
dali target info <profile> --field probe-chip|backend|kernel-binary|kernel-elf
```

`<profile>` is the stable profile name declared by a repository target
manifest. Use `dali target list` to see the available profiles.

## Human-readable output

The default form prints the profile, board, MCU, Rust target, probe chip,
application and AMRN compatibility, clock, memory regions, status LED, USB
controller and pins, and storage controller and pins when declared.

The output is intended for diagnosis and review. Its labels are stable enough
for humans but are not a machine-readable interchange format.

## Machine-readable fields

The `--field` form prints one manifest-owned value for scripting. Supported
fields are `probe-chip`, `backend`, `kernel-binary`, and `kernel-elf`. Repository
build recipes use these fields so backend and artifact names remain owned by
the target manifest rather than duplicated in command logic.

Profiles without a declared probe chip return an error. Unknown profiles and
unsupported fields also return an error.

## Validation boundary

The command reads the generated typed target registry. It does not validate
physical wiring, probe connectivity, firmware behavior, SD-card state, USB
enumeration, or kernel execution.
