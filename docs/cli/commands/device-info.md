# `dali device info`

## Purpose

Display one host-visible device and the generated Dali target metadata matched
to that device. This command is read-only.

## Usage

```text
dali device list
dali device info <id-or-path>
```

Use the `id` shown by `dali device list`. A transport path can also be used
when the selected record exposes one, for example `/dev/ttyACM0` for a CDC
console.

The command discovers the supported transports, selects exactly one matching
record, and prints the normalized device fields. If the record is matched to a
declared target profile, the complete generated profile metadata is printed
under `target metadata:`. An unidentified device is reported without guessed
board information.

The command does not reset, attach, flash, open a terminal, or change device
state.
