# Dali OS

Dali OS is a Rust-based embedded operating system for STM32 microcontrollers and future autonomous or industrial devices.

The project is named after Dali, the Georgian goddess of the hunt. Its application package format, `.amrn` (**Amiran Native**), references Amirani, Dali's son.

## Status

Dali OS is in the early MVP stage.

The first milestone targets an STM32F411CEU6 WeAct BlackPill and proves that the kernel can:

- boot as a `no_std` Rust binary;
- read an `.amrn` package from a FAT16/FAT32 SD card;
- validate its fixed header and CRC32 checksum;
- load its native ARM payload into reserved SRAM;
- transfer control to the documented application entry point.

The MVP application is trusted native code. Sandboxing, memory isolation, signed packages, and application fault isolation are not implemented yet.

## Architecture

```text
SD card: .amrn package
          |
          v
Dali OS loader
          |
          v
Dali OS kernel
          |
          v
STM32F411 hardware
```

The MVP uses a fixed application load address of `0x20008000`, a 32-byte package header, CRC32 integrity validation, and the following entry ABI:

```rust
unsafe extern "C" fn() -> !
```

## Workspace

```text
kernel/               Embedded kernel
apps/dali-app-hello/  Demo application scaffold
crates/dali-sdk/      Application SDK scaffold
crates/dali-cli/      Package and device CLI scaffold
docs/                 Architecture and development documentation
```

## Reference hardware

- MCU: STM32F411CEU6
- Board: WeAct BlackPill
- Target: `thumbv7em-none-eabihf`
- Clock target: 100 MHz
- Status LED: PC13
- SD interface: SPI1

The repository also includes a compile-time backend for the WeAct Studio
STM32F405RGT6 Core Board. Its on-board microSD socket uses the STM32 hardware
SDIO peripheral in 4-bit mode. The F411 BlackPill remains the MVP reference
board; the F405 board is currently intended for storage bring-up.

## Development

Install the embedded Rust target and Lefthook, then install the repository hooks:

```text
rustup target add thumbv7em-none-eabihf
lefthook install
```

Useful commands:

```text
cargo check-kernel
cargo build-kernel
cargo test -p dali-sdk -p dali-app-hello -p dali-cli
```

The complete development and hardware workflow is documented in [DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Documentation

Start with the [Documentation Index](docs/DOCUMENTATION_INDEX.md). Key documents are:

- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [AMRN Format](docs/AMRN_FORMAT.md)
- [Kernel–Application ABI](docs/ABI.md)
- [Coding Standards](docs/CODING_STANDARDS.md)
- [Contributing](CONTRIBUTING.md)

Changes are tracked in the [Changelog](CHANGELOG.md).

## Contributing

Contributions must follow [CONTRIBUTING.md](CONTRIBUTING.md) and [CODING_STANDARDS.md](docs/CODING_STANDARDS.md). All changes are expected to include appropriate tests, hardware evidence, or documentation updates.

## License

Dali OS is licensed under the [GNU Affero General Public License v3.0](LICENSE).
