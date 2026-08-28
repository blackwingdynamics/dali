# Continuous integration

GitHub Actions runs the same validation layers on pushes and pull requests:

- Rust formatting;
- host workspace checks, tests, and Clippy;
- STM32F405 legacy and isolation target checks, Clippy, and kernel builds;
- repository whitespace validation.

The `Formatting`, `Host workspace checks`, `STM32F405 embedded checks`, and `Repository hygiene` jobs must be configured as required status checks in GitHub branch protection before merging is technically blocked.

## Kernel build sequence

### 1. Run local validation

```text
just ci
```

This runs formatting, host checks, both embedded ABI matrix entries, tests,
Clippy, and whitespace validation.

### 2. Build the kernel ELF

```text
just build
```

The embedded ELF is created at:

```text
target/thumbv7em-none-eabihf/debug/dali-kernel
```

The ELF contains symbols and debug information and is the preferred artifact for probe-based debugging.

### 3. Create the raw binary

```text
just bin
```

This runs `cargo objcopy` and creates:

```text
target/thumbv7em-none-eabihf/debug/dali-kernel.bin
```

The `.bin` file is a raw firmware image suitable for flashing tools. It is generated output, is ignored by Git, and is not written to the repository root.

### 4. Flash with a debug probe

Connect the SWD probe, power the board safely, and run:

```text
just flash-probe
```

This builds the ELF and runs it through `probe-rs` using the chip identifier
resolved from the selected target manifest.

### 5. Flash through STM32 DFU mode

Put the board into its STM32 DFU boot mode, connect USB, verify that the host detects the DFU device, and run:

```text
just flash-dfu
```

This builds the raw binary and writes it to the documented internal Flash address. Use `DALI_DFU_DEVICE` to override the USB device identifier:

```text
DALI_DFU_DEVICE=0483:df11 just flash-dfu
```

The command does not invoke `sudo`. Configure the host's USB permissions separately.

### 5a. Select a board backend

The default board is the F405 reference board. Select it explicitly as a
recipe argument without writing Cargo features or environment variables:

```text
just build
just build f405
```

The F405 backend can be checked with:

```text
just kernel-check f405
```

The generated ELF is located at:

```text
target/thumbv7em-none-eabihf/debug/dali-kernel
```

For F405 SWD flashing, run:

```text
just flash-probe f405
```

For DFU flashing, put the board into DFU mode and run:

```text
just flash-dfu f405
```

The raw F405 binary is generated at:

```text
target/thumbv7em-none-eabihf/debug/dali-kernel-f405.bin
```

### 6. Observe RTT output

After flashing, reset the board and connect an RTT viewer through the debug probe. The boot log must remain deterministic and include the documented boot events. Capture the complete output for hardware evidence.

### 7. Build cleanup

```text
just clean
```

This runs `cargo clean` and removes Cargo build artifacts. It does not remove source files, cartridges, SD-card contents, or Git history.

Build success alone is not hardware evidence. Record the board, probe, wiring, firmware revision, power source, tool versions, expected output, observed output, and result.

### 8. Run the Renode simulation

Renode is the supported development simulator for the initial kernel bring-up:

```text
just simulate
just simulate f405
```

The optional board argument selects the firmware build. Both variants run on
the same closest-available STM32F4 reference platform. The simulation can
support development checks for boot and linker placement, but it is not an
electrical model of either board, does not prove SD-card wiring, and does not
replace physical acceptance testing. USB CDC is disabled for this scenario
because the reference model does not implement the OTG_FS global registers
used by the production backend. It also does not currently expose the kernel's
RTT output. See `simulation/README.md` for the evidence boundary.

Preview generated release notes locally with:

```text
git-cliff --unreleased
```
