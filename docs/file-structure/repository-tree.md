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
│   ├── Cargo.toml
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
│       ├── platform/mod.rs         # Hardware-neutral platform facade
│       ├── bootstrap/             # Categorized kernel startup orchestration
│       │   ├── startup/           # Logging, boot banner, and watchdog setup
│       │   ├── lifecycle/         # Storage status and heartbeat behavior
│       │   ├── storage/           # Storage initialization and durable-artifact acceptance
│       │   └── loading/           # Cartridge validation and application launch handoff
│       ├── drivers/               # Hardware-neutral driver contracts/adapters
│       ├── loader/mod.rs          # AMRN dispatch and ABI services
│       ├── loader/repository/      # Repository chain, streaming selection, and tests
│       │   ├── mod.rs              # Bounded chain and durable install
│       │   ├── amrn.rs             # Streamed AMRN v5 validation
│       │   ├── chain/               # Repository chain capture, validation, and loading
│       │   │   ├── mod.rs           # Chain module boundary
│       │   │   ├── bundle.rs        # Bundle role handling
│       │   │   ├── loading.rs       # Repository loading orchestration
│       │   │   ├── roles.rs         # Role capture and replay
│       │   │   ├── streaming.rs     # Selective target scan and streaming
│       │   │   ├── types.rs         # Bounded parser workspace types
│       │   │   └── validation.rs    # Chain validation
│       │   ├── discovery.rs         # Target cartridge selection
│       │   ├── installation.rs      # Durable installation coordination
│       │   ├── io.rs                # Repository stream I/O helpers
│       │   ├── streaming.rs         # Repository streaming primitives
│       │   ├── trust.rs             # Root trust-anchor membership
│       │   └── tests.rs             # Repository loader contract tests
│       ├── loader/contract/       # Hardware-neutral streaming loader contract
│       │   ├── mod.rs              # Streaming validation API
│       │   ├── catalog.rs          # Cartridge catalog policy
│       │   └── tests.rs            # Hardware-neutral contract tests
│       ├── loader/pipeline/       # Responsibility-specific loading paths
│       │   ├── mod.rs             # Pipeline ownership and shared loader imports
│       │   ├── execution.rs       # Fixed-origin ABI execution path
│       │   ├── relocation.rs      # Relocation application path
│       │   ├── identity.rs        # Identity-aware cartridge loading
│       │   ├── discovery.rs       # Real root-cartridge catalog and selection
│       │   ├── services.rs        # Required-service validation
│       │   └── signed.rs          # Signed-cartridge loading path
│       ├── logging/              # Facade, RTT, USB CDC backend
│       ├── runtime/              # Hardware-neutral runtime contracts
│       │   ├── application/     # Lifecycle, active ownership, recovery policy
│       │   ├── memory/          # Manifest-owned slot allocation
│       │   └── scheduling/      # CPU records, tick budget, and context selection
│       └── storage/              # Filesystem and durable storage policy
│           └── filesystem/       # FAT files, repository adapter, and streaming
│               ├── repository.rs # FAT32/LFN RepositoryStreamStorage adapter
│               └── multi.rs      # Bounded multi-cartridge enumeration
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
│   ├── dali-boards/              # Hardware backend crates and board implementations
│   │   ├── src/lib.rs            # Hardware-neutral board-crate facade
│   │   └── dali-board-stm32f405/ # Current extracted F405 backend
│   │       ├── build.rs           # F405 linker artifact generation
│   │       └── src/              # F405 board, drivers, SDIO, MPU, and watchdog
│   ├── dali-amrn/                 # AMRN format contracts and validation
│   ├── dali-crypto/               # no_std Ed25519 signing/verification primitives
│   ├── dali-cli/                  # Installed `dali` CLI
│   │   ├── src/main.rs
│   │   ├── src/commands/           # Top-level dispatch and command domains
│   │   └── templates/app/          # Generated application project files
│   ├── dali-firmware/              # Private firmware composition and binaries
│   │   ├── Cargo.toml, build.rs
│   │   └── src/bin/dali-f405.rs
│   ├── dali-device/               # Hardware-neutral device records
│   ├── dali-driver-api/           # Hardware-neutral driver contracts and mocks
│   ├── dali-metadata/             # Binary v2 metadata and trust-store contracts
│   ├── dali-sdk/                  # Application ABI and SVC API
│   ├── dali-targets/              # TOML validation and generated registry
│   │   ├── src/lib.rs             # Public typed target-profile API
│   │   ├── build.rs               # Build-script orchestration
│   │   └── build/                 # Manifest loader, validation, and generators
│   └── dali-usb/                  # Bounded USB delivery primitives
├── docs/
│   ├── boards/                    # Board-local physical specifications and evidence
│   │   ├── README.md
│   │   └── stm32f405/
│   │       ├── README.md
│   │       ├── pinout-and-clocks.md
│   │       ├── memory-map.md
│   │       ├── hardware-evidence.md
│   │       └── schematics.md
│   ├── hardware/                   # Boards, memory, electrical, and acceptance docs
│   │   ├── README.md
│   │   ├── boards-and-memory.md
│   │   ├── f405-sdio-and-loader.md
│   │   ├── multi-slot-isolation.md
│   │   └── electrical-clock-and-acceptance.md
│   ├── abi/                        # Categorized kernel-application ABI docs
│   │   ├── README.md
│   │   ├── fault-boundary.md, isolation-overview.md
│   │   ├── memory-and-cartridge-contract.md, mvp-and-safety.md
│   │   └── runtime-foundations.md
│   ├── amrn-format/                # Categorized AMRN format documentation
│   │   ├── README.md
│   │   ├── abi-v3.md, identity-and-signatures.md
│   │   ├── mvp-format.md, payload-and-integrity.md
│   │   └── relocatable-format.md
│   ├── architecture/               # Categorized architecture and protocol docs
│   │   ├── README.md
│   │   ├── application-model.md, future-architecture.md
│   │   ├── mvp-and-platform.md, cartridges-and-execution.md
│   │   ├── rfc-ustari-amrn-ipc.md, runtime-and-kernel.md
│   │   ├── storage-and-repository.md, ustari_application_protocol.md
│   │   └── vision-and-scope.md
│   ├── development/                # Setup, build, flash, and debugging docs
│   │   ├── README.md
│   │   ├── build-flash-and-simulation.md
│   │   ├── debugging-and-troubleshooting.md
│   │   ├── setup-and-console.md
│   │   ├── workspace-and-commands.md
│   │   └── operations-and-release.md
│   ├── roadmap/                    # Roadmap gateway and detailed phase plans
│   │   ├── README.md
│   │   ├── current-status-and-execution.md
│   │   ├── safety-evidence-and-working-rules.md
│   │   ├── 01-kernel-core-and-security.md
│   │   ├── 02-hardware-drivers-and-subsystems.md
│   │   ├── 03-system-gui-and-launcher.md
│   │   ├── 04-first-stage-bootloader.md
│   │   ├── 05-interactive-shell-telemetry-and-control.md
│   │   └── 06-documentation-quality-and-enterprise-readiness.md
│   ├── security/
│   │   ├── README.md
│   │   ├── baseline-and-current-boundary.md
│   │   ├── remaining-work-and-production-trust.md
│   │   ├── f405-isolation-foundation.md
│   │   ├── disclosure-and-incident-response.md
│   │   └── threat-model-and-evidence.md
│   ├── mvp-acceptance/             # Categorized MVP acceptance documentation
│   │   ├── README.md
│   │   ├── recorded-evidence.md
│   │   └── scope-and-procedure.md
│   ├── versioning/
│   │   ├── README.md
│   │   ├── overview-and-semver.md
│   │   ├── amrn-and-abi.md
│   │   ├── compatibility-rules.md
│   │   ├── documentation-versioning.md
│   │   ├── release-tags-and-breaking-changes.md
│   │   ├── release-checklist.md
│   │   └── release-readiness.md
│   ├── coding-standards/           # Categorized coding policy documentation
│   │   ├── README.md
│   │   ├── additional-constraints.md
│   │   ├── comments-and-documentation.md
│   │   ├── language-hardcoding-and-discipline.md
│   │   ├── modules-naming-and-logging.md
│   │   ├── review-checklist.md
│   │   ├── safety-errors-and-realtime.md
│   │   ├── testing-review-and-prohibited-practices.md
│   │   └── terminology-and-style.md
│   ├── target-profiles/             # Target profile ownership and manifest docs
│   │   ├── README.md
│   │   ├── ownership-and-boundaries.md
│   │   ├── manifest-contract.md
│   │   └── validation-and-generation.md
│   ├── platform-backends/           # Platform backend ownership and onboarding
│   │   ├── README.md
│   │   ├── ownership-and-boundaries.md
│   │   ├── target-profile-versus-backend.md
│   │   ├── adding-a-target.md
│   │   └── compatibility-rule.md
│   ├── target-manifest/            # Categorized target manifest documentation
│   │   ├── README.md
│   │   ├── clock-bus-and-display.md, memory-pins-and-key.md
│   │   ├── overview-and-structure.md, profile-and-artifacts.md
│   │   ├── usb-storage-and-example.md
│   │   └── validation-and-generation.md
│   ├── application-workflow/
│   │   ├── README.md
│   │   ├── overview-and-layout.md
│   │   ├── build-cartridge-and-inspect.md
│   │   ├── cli-install-and-deploy.md
│   │   └── boundaries-and-troubleshooting.md
│   ├── README.md                    # Documentation navigation index
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
│   ├── cartridge-distribution/      # Categorized cartridge, trust, and acceptance docs
│   │   ├── README.md
│   │   ├── acceptance-and-trust-updates.md, frozen-profile-decisions.md
│   │   ├── goals-and-threat-model.md, keys-and-repository.md
│   │   ├── metadata-format.md, modes-validation-and-order.md
│   │   └── trust-hierarchy.md
│   ├── testing/                   # Categorized testing procedures and evidence
│   │   ├── README.md
│   │   ├── evidence-boundary.md, f405-silicon.md, host.md
│   │   ├── mvp-acceptance.md, requirements-and-evidence.md
│   │   ├── signed-cartridges.md
│   │   └── target.md
│   ├── ustari-protocol/            # Categorized Ustari protocol documentation
│   │   ├── README.md
│   │   ├── acceptance-and-compatibility.md, frames-and-messages.md
│   │   ├── implementation-phases.md, cartridge-and-safety.md
│   │   ├── security-and-authorization.md, status-and-contract.md
│   │   └── streaming-and-transport.md
│   ├── file-structure/             # Categorized repository structure documentation
│   │   ├── README.md
│   │   ├── generated-and-local-content.md, implementation-inventory.md
│   │   ├── ownership-boundaries.md
│   │   └── repository-tree.md
│   ├── cli/                        # CLI guides and command references
│   ├── sdk/                        # Application SDK contracts and compatibility
│   │   └── README.md
│   ├── drivers/                    # Driver contracts and boundaries
│   └── changelog/                  # Archived generated release changelogs
├── scripts/
│   ├── console.sh, archive-changelog.sh, check-commit-message.sh
│   ├── check-docs.py, format-docs.py
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
