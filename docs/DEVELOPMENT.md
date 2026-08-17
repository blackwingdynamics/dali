# Development Workflow

## Prerequisites

- Rust toolchain with the embedded target installed;
- Lefthook 1.7 or newer;
- `git-cliff` for local changelog previews;
- `just` for the repository task runner;
- an SWD programmer/debug probe;
- a WeAct Studio STM32F405RGT6 Core Board with its on-board SDIO socket;
- optionally, a WeAct BlackPill board for target-profile scaffold generation;
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
USB utilities including `picocom`, `probe-rs`, Renode, the .NET runtime
required by the packaged Renode build, and probe-rs udev rules. It is safe to
run again. A logout/login
may be required after the script adds the current user to `plugdev`.

The script performs system package installation and requires `sudo`. It adds
the current user to `uucp` for USB CDC serial access and to `plugdev` for debug
probe access. Log out and back in after the first setup so both group changes
become active. It does not flash hardware, delete project files, or modify Git
history.

### Automated Debian-based Linux setup

On Debian, Ubuntu, and compatible distributions, install the complete
development environment with:

```text
just setup-debian
```

This script uses `apt-get` for system dependencies, installs `probe-rs` through
Cargo, installs `picocom`, and installs Renode's portable Linux release so a
distro-specific .NET runtime package is not required. It is safe to run again.
A logout/login may be required after the script adds the current user to
`plugdev`.

### Automated Fedora setup

On Fedora, install the complete development environment with:

```text
just setup-fedora
```

This script uses `dnf` or `dnf5` for system dependencies, installs `probe-rs`
through Cargo, installs `picocom`, and installs Renode's portable Linux release.
It is safe to run again. A logout/login may be required after the script adds
the current user to `plugdev`.

### Automated macOS setup

On macOS with Homebrew installed, install the complete development environment
with:

```text
just setup-macos
```

This script installs the embedded Rust toolchain, Cargo tools, DFU utilities,
`picocom`, `probe-rs`, and Renode through Homebrew. macOS does not require Linux
udev rules for debug probes. Install Homebrew separately if it is not already
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

The kernel uses `embedded-sdmmc` `0.10.0` with default features disabled. The
filesystem layer is read-only, uses fixed-size buffers, and scans the first FAT
volume's root directory for 8.3 filenames with the `AMRN` extension. It does
not write to the card or require heap allocation. The root scan must not call
the `embedded-sdmmc` volume close operation: that library updates the FAT32
FSInfo sector during close, which conflicts with the kernel's read-only block
device. The scan releases its directory and retains no usable volume handle
after the manager is dropped.
The root scan uses the FAT long-file-name API because the four-character
`.amrn` extension cannot fit in an 8.3 short entry. Comparisons are ASCII
case-insensitive for host-created names.

### USB CDC runtime console

The kernel also exposes the shared logging facade through USB CDC-ACM on the
board USB data pins. After flashing and leaving DFU mode, Linux should expose a
runtime console such as `/dev/ttyACM0`:

```text
just console
```

For a specific device path, use `just console port=/dev/ttyACM1` or set
`DALI_USB_CONSOLE_PORT`. Without an override, the recipe discovers the first
available `/dev/ttyACM*` device. The conventional terminal rate is 115200.
The recipe automatically waits for the device and reconnects after a board
reset or USB disconnect. Exit `picocom` with `Ctrl-A`, then `Ctrl-X`.

The USB console is initialized during bootstrap. The STM32F4 `OTG_FS`
interrupt owns USB control traffic, CDC state, and endpoint writes, so
enumeration and reconnect handling continue while blocking storage bring-up is
running. Main-context logging only appends to the bounded queue; it does not
depend on an enumeration delay or on the main loop resuming. It does not
require an SWD probe. The terminal rate is conventional;
USB CDC does not use a physical UART baud clock. RTT remains available when an
SWD probe is connected.

Boot logging uses a fixed-capacity queue in kernel RAM. Messages emitted before
the host finishes USB enumeration are retained and drained when CDC becomes
available; startup does not depend on an arbitrary enumeration delay. The queue
stores up to 32 formatted lines of 128 bytes each. If that bounded capacity is
exceeded, the oldest messages are discarded and the console emits an explicit
overflow warning after the queue is writable.

CDC transmission also requires an explicit flush after bytes enter the
`usbd-serial` software buffer. A `WouldBlock` result is normal transport
backpressure and must be retried by later USB service events; it is not proof
that a queued log reached the host.

Appending a log record also pends `OTG_FS`. This makes queue insertion a bounded
service trigger during blocking storage operations while keeping all USB state
and endpoint access in the interrupt owner.
The queue is drained only after both USB configuration and the CDC terminal's
host-open (`DTR`) signal are present; device enumeration alone does not consume
boot records.

## Workspace and Git hooks

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
| `f405` | WeAct Studio STM32F405RGT6 Core Board | PB2 | On-board SDIO 4-bit socket |

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

On the STM32F405 board, `PB2` is active-high. Applications must not depend on
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

## Continuous integration

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

This runs `cargo clean` and removes Cargo build artifacts. It does not remove source files, packages, SD-card contents, or Git history.

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
