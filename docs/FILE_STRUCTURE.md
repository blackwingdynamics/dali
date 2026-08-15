# Repository File Structure

This document describes the intended repository layout. Files are added in priority order as the roadmap progresses.

```text
dali-kernel/
├── CONTRIBUTING.md            # Contribution, commit, and review workflow
├── AGENTS.md                  # Coding-agent operating contract
├── CHANGELOG.md               # Project change history
├── SECURITY.md                # GitHub security reporting policy
├── .editorconfig              # Editor formatting defaults
├── .gitattributes              # Git text and binary handling
├── cliff.toml                 # Automatic changelog configuration
├── justfile                    # Build, validation, and flashing commands
├── .github/
│   ├── workflows/
│   │   ├── ci.yml              # Pull request and push validation
│   │   └── release.yml         # Automated changelog and release workflow
│   └── pull_request_template.md # Pull request checklist
├── lefthook.yml               # Pre-commit and commit-message hooks
├── scripts/
│   ├── check-commit-message.sh  # Conventional Commit validator
│   ├── archive-changelog.sh     # Release changelog archiver
│   ├── setup-arch.sh            # Complete Arch Linux environment setup
│   ├── setup-debian.sh          # Complete Debian environment setup
│   ├── setup-fedora.sh          # Complete Fedora environment setup
│   ├── setup-macos.sh           # Complete macOS environment setup
│   └── setup-windows.ps1        # Complete Windows environment setup
├── simulation/
│   ├── README.md                 # Simulator scope and evidence boundary
│   └── renode/
│       └── dali_blackpill.resc   # Renode STM32F4 development scenario
├── Cargo.toml                 # Virtual workspace metadata
├── Cargo.lock                 # Reproducible dependency resolution
├── rust-toolchain.toml        # Pinned Rust toolchain and embedded target
├── .cargo/config.toml         # Embedded build and runner configuration
├── kernel/
│   ├── Cargo.toml             # Embedded kernel package
│   ├── build.rs               # Kernel linker-script search path
│   ├── memory.x               # Kernel and application SRAM layout
│   └── src/
│       ├── main.rs            # Kernel entry point and bootstrap
│       ├── board/              # Compile-time board selection and backends
│       │   ├── mod.rs          # Common board facade
│       │   ├── blackpill_f411.rs # WeAct BlackPill STM32F411 backend
│       │   └── stm32f405_sd.rs  # WeAct STM32F405 SDIO backend
│       ├── drivers/             # Hardware-specific peripheral drivers
│       │   ├── mod.rs           # Driver module registry
│       │   └── sdio.rs          # SDIO block driver
│       ├── logging/              # Kernel-wide logging facade and backends
│       │   ├── mod.rs            # Stable logging API
│       │   ├── rtt.rs            # RTT logging backend
│       │   └── usb_cdc.rs        # Board-neutral USB CDC logging backend
│       ├── bootstrap/            # Kernel startup orchestration
│       │   ├── mod.rs            # Boot sequence
│       │   └── heartbeat.rs      # Status heartbeat loop
│       ├── logging.rs          # RTT and later serial logging
│       ├── storage/            # SD and filesystem subsystem
│       ├── loader/             # AMRN parsing and execution
│       └── runtime/            # Post-MVP task and service runtime
├── apps/
│   └── dali-app-hello/        # First independently built demo app
│       ├── Cargo.toml
│       ├── build.rs           # Application linker search path
│       ├── memory.x           # Reserved application SRAM layout
│       └── src/
│           ├── lib.rs         # SDK-facing application scaffold
│           └── main.rs        # Native validation payload entry point
├── crates/
│   ├── dali-amrn/             # AMRN format parser and validation
│   ├── dali-sdk/              # Future application SDK
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── dali-cli/              # Future package and device CLI
│       ├── Cargo.toml
│       └── src/main.rs
├── docs/
│   ├── ARCHITECTURE.md
│   ├── FILE_STRUCTURE.md
│   ├── DOCUMENTATION_INDEX.md
│   ├── ROADMAP.md
│   ├── CODING_STANDARDS.md
│   ├── AMRN_FORMAT.md         # Binary package specification
│   ├── ABI.md                 # Kernel/application execution contract
│   ├── HARDWARE.md            # Board wiring and electrical assumptions
│   ├── DEVELOPMENT.md         # Build, flash, and debug workflow
│   ├── APPLICATION_WORKFLOW.md # Application build and AMRN packaging workflow
│   ├── TESTING.md             # Host and hardware acceptance tests
│   ├── MVP_ACCEPTANCE.md      # Physical MVP acceptance procedure
│   ├── VERSIONING.md          # Component and compatibility versioning
│   ├── SECURITY.md            # Security model and future guarantees
│   └── changelog/             # Archived generated release changelogs
│       └── README.md
└── README.md                  # Public project introduction
```

## Rust file creation order

The following order keeps each new file focused on one verifiable capability:

1. `kernel/src/board/mod.rs` — compile-time board selection facade.
2. `kernel/src/board/blackpill_f411.rs` — BlackPill constants, pins, clocks, and peripheral ownership.
3. `kernel/src/board/stm32f405_sd.rs` — STM32F405 clock, LED, and SDIO pin ownership.
4. `kernel/src/logging/mod.rs` — the stable kernel logging facade.
5. `kernel/src/logging/rtt.rs` — the RTT logging backend.
6. `kernel/src/bootstrap/mod.rs` — boot sequence orchestration.
7. `kernel/src/bootstrap/heartbeat.rs` — status heartbeat loop.
8. `kernel/src/storage/mod.rs` — storage subsystem types and ownership boundary.
9. `kernel/src/drivers/mod.rs` — hardware driver registry.
10. `kernel/src/drivers/sdio.rs` — SDIO initialization and block reads.
11. `kernel/src/drivers/spi_sd.rs` — STM32F411 SPI SD initialization and block reads.
12. `kernel/src/storage/filesystem.rs` — read-only FAT16/FAT32 access.
13. `kernel/src/loader/mod.rs` — package loader boundary and loader errors.
14. `kernel/src/loader/header.rs` — fixed `.amrn` header parser.
15. `kernel/src/loader/crc32.rs` — payload CRC32 calculation and validation.
16. `kernel/src/loader/exec.rs` — bounded SRAM copy and entry-point transfer.
17. `apps/dali-app-hello/src/main.rs` — first independently built application.
18. `apps/dali-app-hello/memory.x` — application linker memory layout.
19. `apps/dali-app-hello/build.rs` — application package preparation, if required.
20. `crates/dali-sdk/src/lib.rs` — SDK public API after the MVP ABI is stable.
21. `crates/dali-cli/src/main.rs` — CLI entry point after package rules are stable.

Post-MVP runtime files should be added only after the loader acceptance test passes:

1. `kernel/src/runtime/mod.rs` — runtime boundary.
2. `kernel/src/runtime/task.rs` — task representation.
3. `kernel/src/runtime/scheduler.rs` — scheduling policy.
4. `kernel/src/runtime/ipc.rs` — fixed-size channels and event delivery.
5. `kernel/src/runtime/services.rs` — service registry and capability policy.
6. `kernel/src/runtime/watchdog.rs` — heartbeat and fault policy.

Every implementation file should have a corresponding test or hardware evidence entry in `docs/TESTING.md` before the next layer is treated as complete.

## Priority order

1. `docs/ARCHITECTURE.md`
2. `docs/FILE_STRUCTURE.md`
3. `docs/DOCUMENTATION_INDEX.md`
4. `docs/ROADMAP.md`
5. `docs/AMRN_FORMAT.md`
6. `docs/ABI.md`
7. `docs/HARDWARE.md`
8. `docs/DEVELOPMENT.md`
9. `docs/TESTING.md`
10. `docs/SECURITY.md`
11. `README.md`
12. `kernel/src/` and application implementation

The structure is intentionally aspirational. A file should be created only when its corresponding milestone is ready to be specified or implemented.
