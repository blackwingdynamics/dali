# `dali device flash`

## Purpose

Flash an explicit firmware file through a target's declared USB DFU
configuration.

## Usage

```text
dali device flash f405
dali device flash f405 --input custom.bin
dali device flash f405 --transport probe
dali device flash --transport dfu --target f405 --input <firmware>
```

The short form uses the target manifest's declared kernel artifact under the
workspace target directory. The `--input` form overrides that artifact path.
The fully explicit form remains available for scripts. The command reads the
selected target profile from the generated registry and obtains the DFU vendor
ID, product ID, alternate interface, flash address, and leave behavior from the
target manifest. It then invokes `dfu-util` for the download.

The command does not build firmware, select a target automatically, or flash
through a debug probe unless `--transport probe` is selected. Probe flashing
uses the manifest's chip identifier and ELF artifact. Verify the target and
input path before running it; flashing changes device state.
