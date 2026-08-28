# This document describes the files currently tracked in the repository. Build

outputs under `target/` and application-local generated files are intentionally
omitted.

```text
dali-kernel/
├── AGENTS.md, CONTRIBUTING.md, LICENSE, README.md, SECURITY.md
├── Cargo.toml, Cargo.lock, rust-toolchain.toml, justfile, lefthook.yml
├── cliff.toml, .editorconfig, .gitattributes, .gitignore
├── .cargo/config.toml
├── .github/
│   ├── pull_request_template.md
│   └── workflows/ci.yml, release.yml
├── targets/
│   ├── f405.toml                 # Supported STM32F405 target metadata
│   └── f411.toml                 # Generator-only board metadata
├── kernel/
│   ├── Cargo.toml, build.rs
│   └── src/
│       ├── lib.rs                 # Hardware-independent core test surface
│       ├── main.rs                # Kernel entry and bootstrap call
│       ├── abi.rs                 # Central active ABI selector
│       ├── security/              # Privilege, fault, launch, MPU, and scheduling boundaries
│       │   ├── fault/             # Fault records, handlers, recovery, and SCB access
│       │   │   ├── mod.rs         # Fault classification and recovery boundary
│       │   │   ├── persistent.rs  # Retained fault record storage across reset
│       │   │   └── scb.rs         # System Control Block register access
│       │   ├── launch/            # Validated application frame and entry transition
│       │   │   └── mod.rs         # Launch-frame materialization and recovery entry
│       │   ├── mpu/               # MPU contract, layout, and privileged programming
│       │   ├── privilege/         # Application privilege and service gateway
│       │   │   ├── mod.rs         # Privilege module boundary
│       │   │   └── svc.rs         # SVC dispatch and frame validation
│       │   └── scheduling/        # Kernel-owned scheduler initialization boundary
│       │       └── mod.rs         # Target-profile scheduler storage initialization
│       ├── platform/mod.rs        # Platform facade and target entry points
│       ├── platform/f405/          # F405-specific platform backend
│       │   ├── mod.rs              # F405 target profile and IRQ bindings
│       │   ├── board.rs            # Small F405 board backend facade
│       │   ├── board/              # Board configuration and resource ownership
│       │   │   ├── config.rs       # Target metadata and typed board aliases
│       │   │   ├── resources.rs    # Board resources and ownership methods
│       │   │   ├── initialization.rs # Singleton acquisition and setup
│       │   │   ├── acceptance.rs   # Hardware acceptance probe logging
│       │   │   ├── services.rs     # LED and delay services
│       │   │   ├── input.rs        # Board-local input polling
│       │   │   ├── scheduler.rs    # Board-local SysTick scheduler
│       │   │   └── usb.rs          # USB FS resource ownership
│       │   ├── drivers/            # F405 hardware driver adapters
│       │   │   ├── mod.rs          # Platform driver exports
│       │   │   ├── gpio.rs         # F405 GPIO adapter
│       │   │   ├── interrupt.rs    # F405 EXTI adapter
│       │   │   ├── uart.rs         # Bounded F405 UART adapter
│       │   │   ├── spi.rs          # Bounded F405 SPI adapter
│       │   │   ├── i2c.rs          # Bounded F405 I2C adapter
│       │   │   ├── timeout.rs      # F405 polling timeout policy
│       │   │   └── probe.rs        # F405 driver acceptance probe
│       │   ├── sdio.rs             # F405 SDIO transport implementation
│       │   ├── sdio_raw/           # F405 SDIO register transport
│       │       ├── mod.rs          # DMA-backed raw block reader
│       │       ├── init.rs         # Bounded SDIO card initialization
│       │       ├── status.rs       # SDIO status and interrupt helpers
│       │       └── write.rs        # Bounded CPU/FIFO block writes
│       │   └── watchdog/            # F405 watchdog register adapter
│       │       └── mod.rs          # IWDG and reset-cause implementation
│       ├── bootstrap/             # Categorized kernel startup orchestration
│       │   ├── startup/           # Logging, boot banner, and watchdog setup
│       │   ├── lifecycle/         # Storage status and heartbeat behavior
│       │   ├── storage/           # Storage initialization and durable-artifact acceptance
│       │   └── loading/           # Package validation and application launch handoff
│       ├── drivers/               # Hardware-neutral driver contracts/adapters
│       ├── loader/mod.rs          # AMRN dispatch and ABI services
│       ├── loader/repository/      # Repository chain, streaming selection, and tests
│       │   ├── mod.rs              # Bounded chain and durable install
│       │   ├── amrn.rs             # Streamed AMRN v5 validation
│       │   ├── chain.rs            # Generic role capture and replay
│       │   ├── discovery.rs        # Target package selection
│       │   ├── io.rs               # Repository stream I/O helpers
│       │   ├── streaming.rs        # Binary v2 selective target scan
│       │   ├── trust.rs            # Root trust-anchor membership
│       │   └── tests.rs            # Repository loader contract tests
│       ├── loader/contract/       # Hardware-neutral streaming loader contract
│       │   ├── mod.rs              # Streaming validation API
│       │   ├── catalog.rs          # Package catalog policy
│       │   └── tests.rs            # Hardware-neutral contract tests
│       ├── loader/pipeline/       # Responsibility-specific loading paths
│       │   ├── mod.rs             # Pipeline ownership and shared loader imports
│       │   ├── execution.rs       # Fixed-origin ABI execution path
│       │   ├── relocation.rs      # Relocation application path
│       │   ├── identity.rs        # Identity-aware package loading
│       │   ├── discovery.rs       # Real root-package catalog and selection
│       │   ├── services.rs        # Required-service validation
│       │   └── signed.rs          # Signed-package loading path
│       ├── logging/              # Facade, RTT, USB CDC backend
│       ├── runtime/              # Hardware-neutral runtime contracts
│       │   ├── application/     # Lifecycle, active ownership, recovery policy
│       │   ├── memory/          # Manifest-owned slot allocation
│       │   └── scheduling/      # CPU records, tick budget, and context selection
│       └── storage/              # Filesystem and durable storage policy
│           └── filesystem/       # FAT files, repository adapter, and streaming
│               ├── repository.rs # FAT32/LFN RepositoryStreamStorage adapter
│               └── multi.rs      # Bounded multi-package enumeration
├── apps/
│   ├── dali-app-hello/
│   ├── dali-app-slot0-fixture/       # AMRN v4 relocation fixture for slot 0
│   ├── dali-app-relocation-fixture/
│   ├── dali-app-svc-rejections/
│   ├── dali-app-fault-bus/
│   ├── dali-app-fault-execution/
│   ├── dali-app-fault-hard/
│   ├── dali-app-fault-kernel/
│   ├── dali-app-fault-kernel-write/
│   ├── dali-app-fault-cross-slot/       # Slot1-to-slot0 MPU fault fixture
│   ├── dali-app-fault-cross-slot-reverse/ # Slot0-to-slot1 MPU fault fixture
│   ├── dali-app-fault-peripheral/
│   ├── dali-app-fault-peripheral-write/
│   ├── dali-app-fault-no-frame/
│   ├── dali-app-fault-psp/
│   └── dali-app-fault-usage/
├── crates/
│   ├── dali-amrn/                 # AMRN format contracts and validation
│   ├── dali-crypto/               # no_std Ed25519 signing/verification primitives
│   ├── dali-cli/                  # Installed `dali` CLI
│   │   ├── src/main.rs
│   │   ├── src/commands/           # Top-level dispatch and command domains
│   │   └── templates/app/          # Generated application project files
│   ├── dali-device/               # Hardware-neutral device records
│   ├── dali-sdk/                  # Application ABI and SVC API
│   ├── dali-targets/              # TOML validation and generated registry
│   │   ├── src/lib.rs             # Public typed target-profile API
│   │   ├── build.rs               # Build-script orchestration
│   │   └── build/                 # Manifest loader, validation, and generators
│   └── dali-usb/                  # Bounded USB delivery primitives
├── docs/
│   ├── HARDWARE.md
│   ├── abi/                        # Categorized kernel-application ABI docs
│   │   ├── README.md
│   │   ├── fault-boundary.md, isolation-overview.md
│   │   ├── memory-and-package-contract.md, mvp-and-safety.md
│   │   └── runtime-foundations.md
│   ├── amrn-format/                # Categorized AMRN format documentation
│   │   ├── README.md
│   │   ├── abi-v3.md, identity-and-signatures.md
│   │   ├── mvp-format.md, payload-and-integrity.md
│   │   └── relocatable-format.md
│   ├── architecture/               # Categorized architecture and protocol docs
│   │   ├── README.md
│   │   ├── application-model.md, future-architecture.md
│   │   ├── mvp-and-platform.md, packages-and-execution.md
│   │   ├── rfc-ustari-amrn-ipc.md, runtime-and-kernel.md
│   │   ├── storage-and-repository.md, ustari_application_protocol.md
│   │   └── vision-and-scope.md
│   ├── development/                # Setup, build, flash, and debugging docs
│   │   ├── README.md
│   │   ├── build-flash-and-simulation.md
│   │   ├── debugging-and-troubleshooting.md
│   │   ├── setup-and-console.md
│   │   └── workspace-and-commands.md
│   ├── ROADMAP.md
│   ├── security/
│   │   ├── README.md
│   │   ├── baseline-and-current-boundary.md
│   │   ├── remaining-work-and-production-trust.md
│   │   └── f405-isolation-foundation.md
│   ├── mvp-acceptance/             # Categorized MVP acceptance documentation
│   │   ├── README.md
│   │   ├── recorded-evidence.md
│   │   └── scope-and-procedure.md
│   ├── versioning/
│   │   ├── README.md
│   │   ├── overview-and-semver.md
│   │   ├── amrn-and-abi.md
│   │   ├── compatibility-rules.md
│   │   ├── release-tags-and-breaking-changes.md
│   │   └── release-checklist.md
│   ├── coding-standards/           # Categorized coding policy documentation
│   │   ├── README.md
│   │   ├── additional-constraints.md
│   │   ├── comments-and-documentation.md
│   │   ├── language-hardcoding-and-discipline.md
│   │   ├── modules-naming-and-logging.md
│   │   ├── safety-errors-and-realtime.md
│   │   └── testing-review-and-prohibited-practices.md
│   ├── TARGET_PROFILES.md
│   ├── target-manifest/            # Categorized target manifest documentation
│   │   ├── README.md
│   │   ├── clock-bus-and-display.md, memory-pins-and-key.md
│   │   ├── overview-and-structure.md, profile-and-artifacts.md
│   │   ├── usb-storage-and-example.md
│   │   └── validation-and-generation.md
│   ├── application-workflow/
│   │   ├── README.md
│   │   ├── overview-and-layout.md
│   │   ├── build-package-and-inspect.md
│   │   ├── cli-install-and-deploy.md
│   │   └── boundaries-and-troubleshooting.md
│   ├── DOCUMENTATION_INDEX.md
│   ├── relocation/
│   │   ├── README.md
│   │   ├── overview-and-direction.md
│   │   ├── format-and-safety.md
│   │   ├── build-and-load-pipeline.md
│   │   ├── slot-manager-and-evidence.md
│   │   └── linker-evidence-and-non-goals.md
│   ├── metadata-binary-v2/          # Categorized Metadata Binary v2 docs
│   │   ├── README.md
│   │   ├── common-prefix-and-role-bodies.md
│   │   ├── decision-and-envelope.md
│   │   ├── f405-streaming-profile.md
│   │   └── migration-and-compatibility.md
│   ├── package-distribution/      # Categorized package, trust, and acceptance docs
│   │   ├── README.md
│   │   ├── acceptance-and-trust-updates.md, frozen-profile-decisions.md
│   │   ├── goals-and-threat-model.md, keys-and-repository.md
│   │   ├── metadata-format.md, modes-validation-and-order.md
│   │   └── trust-hierarchy.md
│   ├── testing/                   # Categorized testing procedures and evidence
│   │   ├── README.md
│   │   ├── evidence-boundary.md, f405-silicon.md, host.md
│   │   ├── mvp-acceptance.md, signed-packages.md
│   │   └── target.md
│   ├── ustari-protocol/            # Categorized Ustari protocol documentation
│   │   ├── README.md
│   │   ├── acceptance-and-compatibility.md, frames-and-messages.md
│   │   ├── implementation-phases.md, package-and-safety.md
│   │   ├── security-and-authorization.md, status-and-contract.md
│   │   └── streaming-and-transport.md
│   ├── file-structure/             # Categorized repository structure documentation
│   │   ├── README.md
│   │   ├── generated-and-local-content.md, implementation-inventory.md
│   │   ├── ownership-boundaries.md
│   │   └── repository-tree.md
│   ├── cli/                        # CLI guides and command references
│   ├── drivers/                    # Driver contracts and boundaries
│   ├── roadmap/                    # Detailed phase plans
│   └── changelog/                  # Archived generated release changelogs
├── scripts/
│   ├── console.sh, archive-changelog.sh, check-commit-message.sh
│   ├── setup-arch.sh, setup-debian.sh, setup-fedora.sh
│   ├── setup-macos.sh, setup-windows.ps1
│   ├── prepare-f405-binary-v2-sd.sh # Build, sign, verify, and copy an F405 bundle
│   └── gdb/                        # Reusable F405 fault and loader diagnostics
│       ├── bus-fault-mpu.gdb
│       ├── f405-exception-capture.gdb
│       ├── f405-persistent-capture.gdb
│       ├── f405-repository-capture.gdb
│       ├── f405-stacked-fault.gdb
│       ├── fault-diagnostics.gdb
│       └── repository-loader-stack-trace.gdb
└── simulation/
    ├── README.md
    └── renode/dali_blackpill.resc
```
