<p align="center">
  <img src="docs/assets/dali-logo.svg" alt="Dali OS logo" width="220">
</p>

# Dali OS

Dali OS is a Rust-based embedded operating system for STM32 microcontrollers and future autonomous or industrial devices.

The project is named after Dali, the Georgian goddess of the hunt. Its application cartridge format, `.amrn` (**Amiran Native**), references Amirani, Dali's son.

## Status

Dali OS `0.1.0-alpha.1` is the first accepted F405 MVP release. The project
also contains a feature-gated ABI v3 processor-isolation and multi-context
execution path, plus AMRN format v4 identity/relocation and format v5 signed
cartridge paths; these remain post-MVP platform features.

The accepted baseline targets the WeAct Studio STM32F405RGT6 Core Board and
proves that the kernel can:

- boot as a `no_std` Rust binary;
- read an `.amrn` cartridge from a FAT16/FAT32 SD card;
- validate its fixed header and CRC32 checksum;
- load its native ARM payload into reserved SRAM;
- transfer control to the documented application entry point.

The baseline ABI v2 application is trusted native code. The feature-gated F405
path provides hardware-evidenced ABI v3 PSP/SysTick/PendSV context switching,
MPU region switching, faulted-context retirement, bounded kernel-owned SDIO
DMA policy, AMRN v5 signed cartridge/repository verification, anti-rollback
admission, watchdog Safe Mode recovery, and bounded storage-card recovery.
These are scoped implementation claims, not complete sandboxing, arbitrary DMA
isolation, pre-reset Secure Boot, production multi-application lifecycle
policy, or general multi-target portability.

The `dali.secure-boot.v1` image contract is implemented and host-tested, but
pre-reset ROM/first-stage bootloader enforcement remains future work.

## Architecture

```text
SD card: .amrn cartridge
          |
          v
Dali OS loader
          |
          v
Dali OS kernel
          |
          v
STM32F405 hardware
```

The baseline ABI v2 uses a fixed application load address of `0x20008000`, a
32-byte cartridge header, CRC32 integrity validation, and the following entry
ABI:

```rust
unsafe extern "C" fn(*const ServiceTable) -> !
```

## Workspace

```text
kernel/               Embedded kernel
apps/dali-app-hello/  Demo application scaffold
crates/dali-sdk/      Application SDK scaffold
crates/dali-cli/      Cartridge and device CLI scaffold
docs/                 Architecture and development documentation
```

## Reference hardware

- MCU: STM32F405RGT6
- Board: WeAct Studio STM32F405RGT6 Core Board
- Target: `thumbv7em-none-eabihf`
- Clock target: 168 MHz
- Status LED: PB2
- SD interface: hardware SDIO, 4-bit mode

The F411 profile is generator-only and is not a current kernel execution
target. The Pico 2 is used as an external CMSIS-DAP SWD probe, not as a Dali
kernel target.

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
cargo test -p dali -p dali-app-hello -p dali-cli
```

The complete development and hardware workflow is documented in [development documentation](docs/development/README.md).

## Documentation

Start with the [Documentation Index](docs/README.md). Key documents are:

- [Architecture](docs/architecture/README.md)
- [Roadmap](docs/roadmap/README.md)
- [AMRN Format](docs/amrn-format/README.md)
- [Kernel–Application ABI](docs/abi/README.md)
- [Coding Standards](docs/coding-standards/README.md)
- [Contributing](CONTRIBUTING.md)

Changes are tracked in the [Changelog](CHANGELOG.md).

## Contributing

Contributions must follow [CONTRIBUTING.md](CONTRIBUTING.md) and [coding standards](docs/coding-standards/README.md). All changes are expected to include appropriate tests, hardware evidence, or documentation updates.

## License

Dali OS is licensed under the [GNU Affero General Public License v3.0](LICENSE).
