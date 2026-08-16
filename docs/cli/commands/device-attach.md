# `dali device attach`

## Purpose

Start a debug session with a target through a connected SWD probe. This
command does not build or flash firmware.

## Usage

```text
dali device attach --target f405
```

The target profile supplies the probe chip identifier through the generated
target registry. The command then invokes `probe-rs attach --chip <chip>`.
The host must have a configured probe, such as a Raspberry Pi Pico running
CMSIS-DAP or an ST-Link.

The command remains attached to the probe process and forwards its output.
Exit according to the probe-rs session controls.
