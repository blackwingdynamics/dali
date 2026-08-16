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
├── targets/
│   ├── f405.toml                  # Declarative F405 board and target metadata
│   └── f411.toml                  # Generator-only BlackPill board metadata
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
│       │   └── stm32f405_sd.rs   # WeAct STM32F405 SDIO backend
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
│   ├── dali-app-hello/        # First independently built demo app
│   ├── dali-app-fault-kernel/ # Standalone F405 kernel-read fault fixture
│   └── dali-app-fault-kernel-write/ # Standalone F405 kernel-write fault fixture
│   └── dali-app-fault-peripheral/ # Standalone F405 peripheral fault fixture
│   └── dali-app-fault-execution/ # Standalone F405 execute-never fault fixture
│   └── dali-app-fault-psp/ # Standalone F405 invalid-PSP fault fixture
│   └── dali-app-svc-rejections/ # Standalone F405 SVC rejection fixture
│       ├── Cargo.toml
│       ├── build.rs           # Application linker search path
│       ├── memory.x           # Reserved application SRAM layout
│       ├── src/
│           ├── lib.rs         # SDK-facing application scaffold
│           └── main.rs        # Native validation payload entry point
├── crates/
│   ├── dali-amrn/             # AMRN format parser and validation
│   ├── dali-device/           # Hardware-neutral discovery records
│   ├── dali-targets/          # Build-time generated target registry
│   ├── dali-sdk/              # Future application SDK
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── dali-cli/              # Future package and device CLI
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs
│       │   └── commands/
│       │       ├── mod.rs
│       │       ├── app.rs
│       │       ├── new.rs
│       │       ├── package.rs
│       │       └── inspect.rs
│       └── templates/
│           └── app/
│               ├── Cargo.toml.template
│               ├── dali.toml.template
│               ├── build.rs.template
│               ├── memory.x.template
│               ├── lib.rs.template
│               └── main.rs.template
├── docs/
│   ├── ARCHITECTURE.md
│   ├── FILE_STRUCTURE.md
│   ├── DOCUMENTATION_INDEX.md
│   ├── TARGET_PROFILES.md
│   ├── TARGET_MANIFEST.md
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
│   ├── cli/                   # Production CLI documentation
│   │   ├── README.md
│   │   ├── INSTALLATION.md
│   │   ├── QUICKSTART.md
│   │   ├── COMMANDS.md
│   │   ├── WORKFLOWS.md
│   │   ├── OUTPUT.md
│   │   ├── ERRORS.md
│   │   ├── EXIT_CODES.md
│   │   ├── COMPATIBILITY.md
│   │   ├── TROUBLESHOOTING.md
│   │   ├── TESTING.md
│   │   ├── CONTRIBUTING.md
│   │   ├── APPLICATION_PROJECT.md
│   │   ├── DEVICE_DISCOVERY.md
│   │   └── commands/
│   │       ├── package.md
│   │       ├── inspect.md
│   │       ├── app-new.md
│   │       └── target-info.md
│   ├── boards/                # Generated board review checklists
│   └── changelog/             # Archived generated release changelogs
│       └── README.md
└── README.md                  # Public project introduction
```

## Rust file creation order

The following order keeps each new file focused on one verifiable capability:

1. `kernel/src/board/mod.rs` — compile-time board selection facade.
2. `kernel/src/board/stm32f405_sd.rs` — STM32F405 clock, LED, and SDIO pin ownership.
4. `kernel/src/logging/mod.rs` — the stable kernel logging facade.
5. `kernel/src/logging/rtt.rs` — the RTT logging backend.
6. `kernel/src/bootstrap/mod.rs` — boot sequence orchestration.
7. `kernel/src/bootstrap/heartbeat.rs` — status heartbeat loop.
8. `kernel/src/storage/mod.rs` — storage subsystem types and ownership boundary.
9. `kernel/src/drivers/mod.rs` — hardware driver registry.
10. `kernel/src/drivers/sdio.rs` — SDIO initialization and block reads.
11. A future board backend may add a board-owned storage driver after its
    manifest and hardware contract are accepted.
12. `kernel/src/storage/filesystem.rs` — read-only FAT16/FAT32 access.
13. `kernel/src/loader/mod.rs` — package loader boundary and loader errors.
14. `kernel/src/loader/header.rs` — fixed `.amrn` header parser.
15. `kernel/src/loader/crc32.rs` — payload CRC32 calculation and validation.
16. `kernel/src/loader/exec.rs` — bounded SRAM copy and entry-point transfer.
17. `apps/dali-app-hello/src/main.rs` — first independently built application.
18. `apps/dali-app-hello/memory.x` — application linker memory layout.
19. `apps/dali-app-hello/build.rs` — application package preparation, if required.
20. `crates/dali-sdk/src/lib.rs` — SDK public API after the MVP ABI is stable.
21. `crates/dali-cli/src/main.rs` — CLI process entry point.
22. `crates/dali-cli/src/commands/mod.rs` — command dispatch and shared flag parsing.
23. `crates/dali-cli/src/commands/package.rs` — AMRN package construction command.
24. `crates/dali-cli/src/commands/inspect.rs` — AMRN package inspection command.
25. `crates/dali-cli/src/commands/app.rs` — application command group dispatch.
26. `crates/dali-cli/src/commands/new.rs` — application scaffold creation.
27. `crates/dali-cli/src/commands/init.rs` — existing directory initialization.
28. `crates/dali-cli/src/commands/build.rs` — native application payload build.
29. `crates/dali-cli/templates/app/config.toml.template` — standalone Cargo linker configuration.
30. `crates/dali-cli/src/commands/app_package.rs` — application payload packaging.
31. `crates/dali-cli/src/commands/doctor.rs` — host and toolchain diagnostics.
32. `crates/dali-cli/src/commands/target.rs` — supported application target profiles.
33. `targets/*.toml` — declarative manufacturer and compatibility metadata.
34. `crates/dali-targets/` — validated generated target registry shared by host tooling.
27. `crates/dali-cli/templates/app/` — versioned application scaffold assets.

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
