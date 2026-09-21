# Prerequisites

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
required by the cartridged Renode build, and probe-rs udev rules. It is safe to
run again. A logout/login
may be required after the script adds the current user to `plugdev`.

The script performs system cartridge installation and requires `sudo`. It adds
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
distro-specific .NET runtime cartridge is not required. It is safe to run again.
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

Install `probe-rs` and `dfu-util` using the cartridge-manager or installation method appropriate for the host operating system. The exact hardware flashing tools are host dependencies, not Cargo workspace members.

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
