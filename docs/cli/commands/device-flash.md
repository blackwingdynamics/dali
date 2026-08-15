# `dali device flash`

## Purpose

Flash an explicit firmware file through a target's declared USB DFU
configuration.

## Usage

```text
dali device flash --transport dfu --target f405 --input <firmware>
```

The input must be an existing regular file. The command reads the selected
target profile from the generated registry and obtains the DFU vendor ID,
product ID, alternate interface, flash address, and leave behavior from the
target manifest. It then invokes `dfu-util` for the download.

The command does not build firmware, select a target automatically, or flash
through a debug probe. Verify the target and input path before running it;
flashing changes device state.
