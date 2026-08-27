# Repository File Structure

This document describes the files currently tracked in the repository. Build
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
│   ├── ABI.md, AMRN_FORMAT.md, HARDWARE.md
│   ├── architecture/             # Categorized architecture and protocol docs
│   ├── development/              # Setup, build, flash, and debugging docs
│   ├── ROADMAP.md, MVP_ACCEPTANCE.md, SECURITY.md
│   ├── CODING_STANDARDS.md, VERSIONING.md
│   ├── TARGET_MANIFEST.md, TARGET_PROFILES.md
│   ├── APPLICATION_WORKFLOW.md, DOCUMENTATION_INDEX.md
│   ├── METADATA_BINARY_V2.md, RELOCATION.md, FILE_STRUCTURE.md
│   ├── package-distribution/      # Categorized package, trust, and acceptance docs
│   ├── testing/                   # Categorized testing procedures and evidence
│   ├── ustari-protocol/            # Categorized Ustari protocol documentation
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

## Exact implementation file inventory

The directory tree above groups related files. The current tracked source
files inside those groups are:

```text
kernel/src/
├── lib.rs
├── main.rs
├── abi.rs
├── security/{mod.rs,
│   fault/{mod.rs,persistent.rs,scb.rs},launch/{mod.rs},
│   mpu/{mod.rs,descriptor.rs,layout.rs,hardware.rs,tests.rs},
│   privilege/{mod.rs,svc.rs},scheduling/{mod.rs}}
├── platform/{mod.rs,f405/{mod.rs,board.rs,board/{acceptance.rs,config.rs,initialization.rs,input.rs,resources.rs,scheduler.rs,services.rs,usb.rs},drivers/{mod.rs,gpio.rs,i2c.rs,i2c/operations.rs,interrupt.rs,probe.rs,spi.rs,timeout.rs,timer.rs,uart.rs},sdio.rs,sdio_raw/{mod.rs,dma.rs,init.rs,status.rs,write.rs},watchdog/mod.rs}}
├── bootstrap/{mod.rs,
│   startup/{mod.rs,logging.rs,watchdog.rs},
│   lifecycle/{mod.rs,heartbeat.rs,status.rs},
│   storage/{mod.rs,initialization.rs,acceptance.rs},
│   loading/{mod.rs,package.rs}}
├── drivers/{mod.rs,block.rs,sdio.rs}
├── loader/{mod.rs,repository/{mod.rs,amrn.rs,chain.rs,discovery.rs,io.rs,streaming.rs,trust.rs,tests.rs},contract/{mod.rs,catalog.rs,tests.rs},pipeline/{mod.rs,execution.rs,relocation.rs,identity.rs,discovery.rs,services.rs,signed.rs}}
├── logging/{mod.rs,rtt.rs,usb_cdc.rs}
├── runtime/{mod.rs,application/{mod.rs,lifecycle.rs,owner.rs,policy.rs},memory/{mod.rs,dma.rs,slots.rs},scheduling/{mod.rs,context_switch.rs,saved_state.rs,record.rs,context_table.rs,scheduler.rs,storage.rs,tick.rs},watchdog/mod.rs}
└── storage/{mod.rs,durable.rs,durable/{journal.rs,coordinator.rs},filesystem/{mod.rs,artifacts.rs,multi.rs,read.rs,tests.rs,write.rs},repository.rs}

crates/dali-amrn/src/
├── lib.rs                         # Stable crate facade and legacy re-exports
├── compatibility/mod.rs           # ABI-to-format compatibility rules
├── legacy/                        # Original fixed-origin package contract
│   ├── mod.rs, builder.rs, stream.rs, tests.rs
├── v2/                            # Segmented ABI v3 package contract
│   ├── mod.rs, tests.rs
├── v3/                            # Relocatable ABI v3 package contract
│   ├── mod.rs, apply.rs, codec.rs, wire.rs, tests.rs
├── v4/                            # Identity and selection metadata extension
│   ├── mod.rs, tests.rs
└── v5/                            # Signed container extension; kernel-gated
    ├── mod.rs, codec.rs, tests.rs

crates/dali-cli/src/
├── main.rs
└── commands/
    ├── mod.rs, doctor.rs, inspect.rs, key.rs, package.rs
    ├── app/
    │   ├── mod.rs, artifacts.rs, build.rs, init.rs, linker.rs
    │   ├── new.rs, new_tests.rs, package.rs, relocations.rs
    ├── device/
    │   ├── mod.rs, attach.rs, cdc.rs, console.rs, flash.rs
    │   ├── flash_transport.rs, info.rs
    ├── metadata/
    │   ├── mod.rs
    │   ├── delegation/mod.rs
    │   ├── repository/{mod.rs,common.rs,init.rs,add_developer.rs,
    │   │   manifest.rs,publish.rs,register_package.rs}
    │   └── bundle/{mod.rs,common.rs,generate.rs,verify.rs}
    └── target/{mod.rs,info.rs}

crates/dali-cli/templates/app/
├── Cargo.toml.template, build.rs.template, config.toml.template
├── dali.toml.template, lib.rs.template, main.rs.template
└── memory.x.template, memory.v3.x.template

crates/dali-device/src/lib.rs
crates/dali-metadata/src/
├── codec/
│   ├── mod.rs, binary/mod.rs
│   └── bundle.rs, delegation.rs, envelope.rs, revocation.rs, root.rs,
│       signatures.rs, snapshot.rs, targets.rs, timestamp.rs, writer.rs
├── parser/
│   ├── mod.rs, tests.rs, binary/mod.rs
│   ├── canonical/
│   │   ├── mod.rs, cursor.rs, bundle.rs, envelope.rs, signatures.rs
│   │   └── delegation.rs, revocation.rs, root.rs, snapshot.rs,
│   │       targets.rs, timestamp.rs
│   └── streaming/
│       ├── mod.rs, chain.rs
│       └── delegation.rs, revocation.rs, root.rs, snapshot.rs
└── model.rs, validation.rs, verification.rs, chain.rs, trust_store.rs
crates/dali-sdk/src/{lib.rs,svc.rs,svc_log.rs}
crates/dali-targets/src/lib.rs
crates/dali-targets/build.rs
crates/dali-targets/build/{loader.rs,manifest.rs,render.rs,
│   generator/{mod.rs,authentication.rs,hardware.rs,profile.rs},
│   validation/{mod.rs,memory.rs,security.rs}}
crates/dali-usb/src/{lib.rs,tests.rs}
```

Each application fixture has its own `Cargo.toml`, `build.rs`, `src/lib.rs`,
and `src/main.rs`. The fault and SVC fixtures additionally have a local
`.cargo/config.toml`, `Cargo.lock`, and `dali.toml`. The relocation fixture has
its own `Cargo.lock` but uses the root build configuration. `dali-app-hello`
The kernel build script generates its linker `memory.x` from the selected
target manifest; the generated script is not tracked.

The CLI documentation files currently tracked under `docs/cli/commands/` are:

```text
app-build.md, app-init.md, app-new.md, app-package.md,
device-attach.md, device-console.md, device-flash.md, device-info.md,
doctor.md, inspect.md, package.md, target-info.md, target-list.md,
target-scaffold.md
```

## Ownership boundaries

- `kernel/src/security/mpu/` owns MPU descriptors, memory-layout validation,
  and privileged activation of application regions.
- `kernel/src/platform/mod.rs` and `kernel/src/platform/` own the platform facade
  and target-specific entry points; backend ownership and contributor workflow are defined in
  `docs/PLATFORM_BACKENDS.md`.
- `kernel/src/platform/f405/sdio.rs` and `kernel/src/platform/f405/sdio_raw/`
  own the F405 PAC/HAL SDIO transport;
  bootstrap consumes it only through the platform facade.
- `kernel/src/platform/f405/watchdog/` owns the F405 IWDG and RCC reset-cause
  register adapter; watchdog policy remains in `kernel/src/runtime/watchdog/`.
- `kernel/src/drivers/` owns hardware-neutral driver contracts and adapters;
  it must not import a board PAC or HAL.
- `kernel/src/drivers/sdio.rs` owns the generic SDIO transport contract and
  block-reader adapter; platform code supplies the concrete transport.
- `kernel/src/runtime/memory/dma.rs` owns the target-independent DMA buffer
  ownership and range-validation contract; SDIO, USB, SPI, ADC, and future
  peripheral backends may consume it without changing the contract.
- `targets/*.toml` owns declarative target facts; hardware implementations must
  consume those facts through generated target metadata instead of copying
  board constants into kernel policy.
- `kernel/src/storage/` owns SD/filesystem policy and durable repository
  coordination. `storage/durable.rs` defines the board-agnostic block and
  durable-adapter contracts; `storage/durable/journal.rs` owns the 104-byte
  DALI-CMT codec; and `storage/durable/coordinator.rs` owns the bounded
  persistence state machine. Package parsing remains in
  `crates/dali-amrn/` and bootstrap loading policy remains in
  `kernel/src/bootstrap/loading/` while AMRN execution contracts remain in
  `kernel/src/loader/`.
- `kernel/src/bootstrap/startup/` owns logging/banner and watchdog startup;
  `kernel/src/bootstrap/lifecycle/` owns heartbeat/status behavior;
  `kernel/src/bootstrap/storage/` owns storage initialization and the
  feature-gated durable-artifact acceptance path; and
  `kernel/src/bootstrap/loading/` owns package-to-runtime handoff.
- `kernel/src/security/` owns privileged SVC dispatch, launch frames, fault
  recovery, and MPU protection. The hardware-neutral MPU descriptors and
  layout builder are separated from privileged register programming.
- `kernel/src/runtime/application/` owns application lifecycle state, active
  context ownership, and restart/rollback/watchdog policy decisions.
- `kernel/src/runtime/memory/` owns manifest-declared slot allocation and range
  containment.
- `kernel/src/runtime/watchdog/` owns the hardware-neutral watchdog feed-owner,
  timing, and reset-cause contracts; target backends implement the hardware
  adapter separately.
- `kernel/src/runtime/scheduling/` owns hardware-neutral saved CPU state,
  manifest-slot-bound scheduler records, and bounded context selection;
  PendSV/SysTick handlers and MPU switching are implemented by the F405
  backend; equivalent implementations for other MCU families remain future
  platform work.
- `crates/dali-targets/` generates target metadata from `targets/*.toml`; no
  board profile should be duplicated in CLI or kernel policy code.
- `crates/dali-cli/src/commands/` owns top-level dispatch and groups related
  commands by domain. `app/`, `device/`, `metadata/`, and `target/` each expose
  a `mod.rs` dispatcher; complex commands may contain focused helper modules.
  Standalone commands remain directly under `commands/`. Command
  documentation lives in `docs/cli/commands/`.
- `apps/` contains independently built validation applications and fixtures;
  these are not kernel modules.
- `docs/testing/` contains the categorized testing procedures and evidence
  records indexed by `docs/testing/README.md`.

## Generated and local-only content

The following are intentionally absent from this tree document:

- Cargo `target/` directories and compiled `.bin`/`.amrn` artifacts;
- generated linker scripts inside application build directories;
- editor settings, mounted SD-card paths, and debugger sessions;
- generated target registry output under Cargo `OUT_DIR`.

When a new tracked module or documentation area is added, update this file in
the same change so it remains a description of the repository rather than an
aspirational creation order.
