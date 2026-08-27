# Workspace and Git hooks

The repository is a Cargo workspace containing the kernel, the hardware-neutral
`dali-usb` delivery primitives, the hardware-neutral `dali-amrn` format layer,
the future `dali` package, the `dali` command package tool, and the initial
demo-application scaffold. Run workspace commands from the repository root.

`dali-usb` is `no_std` and has no MCU or HAL dependency. It owns only bounded
delivery mechanics that can be tested on the host. Board USB resources and
production CDC servicing remain in the kernel logging backend.

`dali-amrn` is `no_std` and has no filesystem, MCU, or HAL dependency. It owns
explicit AMRN v1 header decoding, payload bounds, entry validation, and CRC32
verification so the kernel and CLI can share the format contract.

The host CLI can wrap a raw payload in a contract-valid AMRN package:

```text
cargo run -p dali-cli --bin dali -- package \
  --input <payload.bin> \
  --output <package.amrn> \
  --entry-offset <byte-offset>
```

The entry offset is explicit because the package command does not infer symbol
locations from an ELF file. The input must already be linked native payload
for the documented target and load address.

Inspect an existing package without changing it:

```text
cargo run -p dali-cli --bin dali -- inspect \
  --input <package.amrn>
```

The inspection command reports the decoded header fields and rejects invalid
AMRN data, CRC32 mismatches, and trailing bytes.

To install the CLI locally and use it without `cargo run`:

```text
cargo install --path crates/dali-cli --locked
dali inspect --input <package.amrn>
```

Cargo installs the executable under its configured binary directory, normally
`~/.cargo/bin`. That directory must be in `PATH` for the `dali` command to be
available directly.

The embedded target is selected explicitly for kernel commands so host-side SDK and CLI tooling can be checked normally:

```text
cargo check-kernel
cargo build-kernel
```

Install the Git hooks after cloning:

```text
lefthook install
```

## Just commands

Run `just` or `just --list` to inspect the available commands:

```text
just ci
just build
just bin
just flash-probe
just flash-dfu
just attach
```

`just bin` requires `cargo-binutils` and the `llvm-tools-preview` Rust component. `just flash-probe` requires `probe-rs`. `just flash-dfu` requires `dfu-util` and appropriate host permissions; it never invokes `sudo` automatically.

### Board selection

Hardware recipes accept one optional positional board argument. The default is
`f405`.

| Argument | Board | On-board LED | SD interface |
| --- | --- | --- | --- |
| `f405` | WeAct Studio STM32F405RGT6 Core Board | PB2 | On-board SDIO 4-bit socket; PC13 user key |

Use the same recipe names for either board:

```text
just build
just build f405
just kernel-check f405
just bin f405
just flash-dfu f405
just flash-probe f405
```

### Storage status LED

The kernel reports the initial storage state through the board-specific status
LED. The board backend hides LED polarity, so the logical behavior is identical
on both supported boards:

| Storage state | LED behavior | Meaning |
| --- | --- | --- |
| Ready | Solid on | The card initialized and block zero was read successfully. |
| Not detected | Slow blink, 1 second per transition | No card was detected or storage is not configured for the selected board. |
| Failure | Fast blink, 100 milliseconds per transition | The card or storage transport reported an operational failure. |

On the STM32F405 board, `PB2` is active-high and `PC13` is an active-low user
key. Applications must not depend on
the physical polarity; the board backend owns that mapping.

Do not pass Cargo feature names to these recipes. The recipe converts the board
argument into the correct compile-time backend automatically. `just ci` checks
the default F405 backend; use `just kernel-check f405` and
`just kernel-clippy f405` for the F405 target checks.

Verify the tools before using hardware commands:

```text
rustc --version
cargo --version
just --version
lefthook version
git-cliff --version
rust-objcopy --version
probe-rs --version
dfu-util --version
picocom --version
```

The pre-commit hook runs formatting, workspace checks, Clippy with warnings denied, workspace tests, and staged-diff validation. The commit-msg hook enforces the Conventional Commit format.
