# `dali doctor`

## Purpose

Check the host prerequisites for Dali application builds and hardware
workflows.

## Syntax

```text
dali doctor
```

## Checks

Required checks cover:

- `rustc`;
- `cargo`;
- the `thumbv7em-none-eabihf` Rust target;
- `cargo objcopy`.

Optional checks cover tools used for hardware workflows:

- `probe-rs`;
- `dfu-util`;
- `picocom`.

Missing optional tools produce warnings. A missing required prerequisite makes
the command fail after printing every check result.

## Output

Each check is printed with one of these statuses:

```text
[OK]    prerequisite is available
[FAIL]  required prerequisite is unavailable
[WARN]  optional tool is unavailable
```

The command checks tool availability only. It does not flash a board, probe a
device, change the Rust toolchain, or install dependencies.
