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
The command opens the CDC port in a line-synchronizing terminal. It discards
an incomplete line left by a previous USB session, so reconnecting cannot
display a suffix such as `llo World from AMR` as a new log record.

If no CDC console is visible, connect a running firmware image and wait for
the host to create `/dev/ttyACM*`. If multiple consoles are visible, select
one explicitly.

The command does not flash, reset, mount storage, or infer a target from a
connection-scoped CDC identity.
