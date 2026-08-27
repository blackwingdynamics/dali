# Debugging rules

- keep the boot log deterministic;
- log validation failures with a reason;
- never hide SD or loader errors behind a generic panic during bring-up;
- record the exact package bytes used for an acceptance test;
- keep application and kernel linker layouts under version control.

## Troubleshooting

### `rust-objcopy: command not found`

Install the Cargo binutils wrapper and LLVM tools:

```text
cargo install cargo-binutils --locked
rustup component add llvm-tools-preview
```

Then retry:

```text
just bin
```

### `.bin` file is not created

Run `just build` first and confirm that the ELF exists. Then run `just bin` and inspect:

```text
target/thumbv7em-none-eabihf/debug/dali-kernel
target/thumbv7em-none-eabihf/debug/dali-kernel.bin
```

The `.bin` file is created only after the ELF-to-binary conversion succeeds.

### `probe-rs` cannot find the chip

Confirm the probe connection, power, SWD wiring, and chip identifier. Use `DALI_CHIP` to provide the identifier reported by `probe-rs list`.

### DFU device is not detected

Confirm that the board is in DFU mode, the USB cable supports data, the board is powered, and host USB permissions are configured. Run `dfu-util -l` before retrying `just flash-dfu`.

### Kernel check succeeds but hardware behavior is wrong

Compilation and target checks do not validate wiring, clock behavior, SD electrical levels, or application execution. Record the failure as hardware evidence and follow `docs/MVP_ACCEPTANCE.md`.
