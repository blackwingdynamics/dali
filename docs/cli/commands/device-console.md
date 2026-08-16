# `dali device console`

## Purpose

Open the runtime USB CDC console for a Dali device. The command is host-side
and does not modify device state.

## Usage

```text
dali device console
dali device console --port /dev/ttyACM0
```

Without `--port`, the command discovers Linux CDC records and requires exactly
one usable console path. With `--port`, it opens the explicitly supplied path.
The command starts `picocom`; install it before using this command.

If no CDC console is visible, connect a running firmware image and wait for
the host to create `/dev/ttyACM*`. If multiple consoles are visible, select
one explicitly.

The command does not flash, reset, mount storage, or infer a target from a
connection-scoped CDC identity.
