# Development Workflow

## Prerequisites

- Rust toolchain with the embedded target installed;
- Lefthook 1.7 or newer;
- `git-cliff` for local changelog previews;
- `just` for the repository task runner;
- an SWD programmer/debug probe;
- a WeAct BlackPill STM32F411 board;
- optionally, a WeAct Studio STM32F405RGT6 Core Board with its on-board SDIO
  socket;
- a correctly wired 3.3 V SD-card interface;
- a supported RTT viewer.

## Tool installation

### Automated Arch Linux setup

From the repository root, install the complete Arch Linux development
environment with:

```text
just setup-arch
```

The script installs the pinned Rust target and components, Cargo tools, DFU and
USB utilities, `probe-rs`, Renode, the .NET runtime required by the packaged
Renode build, and probe-rs udev rules. It is safe to run again. A logout/login
may be required after the script adds the current user to `plugdev`.

The script performs system package installation and requires `sudo`. It does
not flash hardware, delete project files, or modify Git history.

### Automated Debian-based Linux setup

On Debian, Ubuntu, and compatible distributions, install the complete
development environment with:

```text
just setup-debian
```

This script uses `apt-get` for system dependencies, installs `probe-rs` through
Cargo, and installs Renode's portable Linux release so a distro-specific .NET
runtime package is not required. It is safe to run again. A logout/login may
be required after the script adds the current user to `plugdev`.

### Automated Fedora setup

On Fedora, install the complete development environment with:

```text
just setup-fedora
```

This script uses `dnf` or `dnf5` for system dependencies, installs `probe-rs`
through Cargo, and installs Renode's portable Linux release. It is safe to run
again. A logout/login may be required after the script adds the current user
to `plugdev`.

### Automated macOS setup

On macOS with Homebrew installed, install the complete development environment
with:

```text
just setup-macos
```

This script installs the embedded Rust toolchain, Cargo tools, DFU utilities,
`probe-rs`, and Renode through Homebrew. macOS does not require Linux udev
rules for debug probes. Install Homebrew separately if it is not already
available.

### Automated Windows setup

From PowerShell, run the setup script with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/setup-windows.ps1
```

After the first setup, the same script is also available through:

```text
just setup-windows
```

The script uses WinGet for Windows build prerequisites, Cargo for Rust tools
and `probe-rs`, and portable releases for Renode and `dfu-util`. Windows USB
drivers are device-specific: DFU requires a WinUSB-compatible driver, while
ST-Link may require its vendor driver.

Install the embedded Rust target and required Rust tools:

```text
rustup target add thumbv7em-none-eabihf
rustup component add llvm-tools-preview
cargo install cargo-binutils --locked
cargo install just --locked
cargo install lefthook --locked
cargo install git-cliff --version 2.13.0 --locked
```

Install `probe-rs` and `dfu-util` using the package-manager or installation method appropriate for the host operating system. The exact hardware flashing tools are host dependencies, not Cargo workspace members.

## Workspace and Git hooks

The repository is a Cargo workspace containing the kernel, the future `dali-sdk`, the future `dali-cli`, and the initial demo-application scaffold. Run workspace commands from the repository root.

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
```

The pre-commit hook runs formatting, workspace checks, Clippy with warnings denied, workspace tests, and staged-diff validation. The commit-msg hook enforces the Conventional Commit format.

## Continuous integration

GitHub Actions runs the same validation layers on pushes and pull requests:

- Rust formatting;
- host workspace checks, tests, and Clippy;
- STM32F411 target checks, Clippy, and kernel build;
- repository whitespace validation.

The `Formatting`, `Host workspace checks`, `STM32F411 embedded checks`, and `Repository hygiene` jobs must be configured as required status checks in GitHub branch protection before merging is technically blocked.

## Kernel build sequence

### 1. Run local validation

```text
just ci
```

This runs formatting, host checks, embedded checks, tests, Clippy, and whitespace validation.

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

This builds the ELF and runs it through `probe-rs` using the configured STM32F411 chip identifier. Use `DALI_CHIP` to override the identifier when the probe reports a different compatible name:

```text
DALI_CHIP=STM32F411CEUx just flash-probe
```

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

The default board is the F411 BlackPill. Select another board as a recipe
argument without writing Cargo features or environment variables:

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
target/thumbv7em-none-eabihf/debug/dali-kernel.bin
```

### 6. Observe RTT output

After flashing, reset the board and connect an RTT viewer through the debug probe. The boot log must remain deterministic and include the documented boot events. Capture the complete output for hardware evidence.

### 7. Build cleanup

```text
just clean
```

This runs `cargo clean` and removes Cargo build artifacts. It does not remove source files, packages, SD-card contents, or Git history.

Build success alone is not hardware evidence. Record the board, probe, wiring, firmware revision, power source, tool versions, expected output, observed output, and result.

### 8. Run the Renode simulation

Renode is the supported development simulator for the initial kernel bring-up:

```text
just simulate
```

The scenario uses the closest available STM32F4 reference platform. It can
support development checks for boot, linker placement, SysTick progress, and
selected GPIO behavior. It does not replace physical STM32F411 acceptance
testing and does not currently expose the kernel's RTT output. See
`simulation/README.md` for the evidence boundary.

Preview generated release notes locally with:

```text
git-cliff --unreleased
```

## Debugging rules

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
