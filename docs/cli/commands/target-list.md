# `dali target list`

## Purpose

List Dali application target profiles supported by the current AMRN and ABI
contracts.

Profiles are declared in repository-level `targets/*.toml` manifests and
validated into the generated `dali-targets` registry. Adding a supported board
adds a manifest and a kernel mapping; it does not require duplicating board
constants inside individual CLI commands.

## Syntax

```text
dali target list
```

The command lists only profiles that are valid application targets. A
compile-time board backend without an assigned AMRN target identifier is not
listed as an application target.

## Output

Each profile includes:

- profile name;
- board and MCU;
- Rust compilation target;
- declared probe chip identifier, when available;
- AMRN target identifier;
- ABI version.

The current supported application profile is `f405`, backed by the WeAct
Studio STM32F405RGT6 Core Board.
