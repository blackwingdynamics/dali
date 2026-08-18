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
│       ├── security/              # MPU, SCB MMIO, SVC, launch, and fault recovery
│       │   └── mpu/               # MPU contract, layout, and privileged programming
│       ├── platform/mod.rs         # Platform facade and target entry points
│       ├── platform/f405/          # F405-specific platform backend
│       │   ├── mod.rs              # F405 target profile and IRQ bindings
│       │   ├── board.rs            # F405 hardware resources and board API
│       │   ├── sdio.rs             # F405 SDIO transport implementation
│       │   └── sdio_raw.rs         # F405 SDIO register transport
│       ├── bootstrap/             # Startup, storage policy, status, heartbeat
│       ├── drivers/               # Hardware-neutral driver contracts/adapters
│       ├── loader/mod.rs           # AMRN dispatch and ABI services
│       ├── loader/contract.rs     # Hardware-neutral streaming loader contract
│       ├── loader/v3.rs           # Fixed-origin ABI v3 loader
│       ├── loader/v3_relocatable.rs # Feature-gated format 3 loader
│       ├── loader/v4.rs           # Identity-aware streaming loader
│       ├── logging/               # Facade, RTT, USB CDC backend
│       ├── runtime/               # Hardware-neutral application lifecycle state
│       └── storage/               # Read-only filesystem and storage policy
├── apps/
│   ├── dali-app-hello/
│   ├── dali-app-relocation-fixture/
│   ├── dali-app-svc-rejections/
│   ├── dali-app-fault-bus/
│   ├── dali-app-fault-execution/
│   ├── dali-app-fault-kernel/
│   ├── dali-app-fault-kernel-write/
│   ├── dali-app-fault-peripheral/
│   ├── dali-app-fault-peripheral-write/
│   ├── dali-app-fault-no-frame/
│   └── dali-app-fault-psp/
├── crates/
│   ├── dali-amrn/                 # AMRN format contracts and validation
│   ├── dali-cli/                  # Installed `dali` CLI
│   │   ├── src/main.rs
│   │   ├── src/commands/           # One focused module per command
│   │   └── templates/app/          # Generated application project files
│   ├── dali-device/               # Hardware-neutral device records
│   ├── dali-sdk/                  # Application ABI and SVC API
│   ├── dali-targets/              # TOML validation and generated registry
│   └── dali-usb/                  # Bounded USB delivery primitives
├── docs/
│   ├── ARCHITECTURE.md, ABI.md, AMRN_FORMAT.md, HARDWARE.md
│   ├── ROADMAP.md, TESTING.md, MVP_ACCEPTANCE.md, SECURITY.md
│   ├── CODING_STANDARDS.md, DEVELOPMENT.md, VERSIONING.md
│   ├── TARGET_MANIFEST.md, TARGET_PROFILES.md
│   ├── APPLICATION_WORKFLOW.md, DOCUMENTATION_INDEX.md
│   ├── RELOCATION.md, FILE_STRUCTURE.md
│   ├── cli/                        # CLI guides and command references
│   └── changelog/                  # Archived generated release changelogs
├── scripts/
│   ├── console.sh, archive-changelog.sh, check-commit-message.sh
│   ├── setup-arch.sh, setup-debian.sh, setup-fedora.sh
│   ├── setup-macos.sh, setup-windows.ps1
│   └── gdb/bus-fault-mpu.gdb
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
├── security/{mod.rs,mpu/{mod.rs},fault.rs,launch.rs,scb.rs,svc.rs}
├── platform/{mod.rs,f405/{mod.rs,board.rs,sdio.rs,sdio_raw.rs}}
├── bootstrap/{mod.rs,storage.rs,heartbeat.rs,status.rs}
├── drivers/{mod.rs,block.rs,sdio.rs}
├── loader/{mod.rs,contract.rs,v3.rs,v3_relocatable.rs,v4.rs}
├── logging/{mod.rs,rtt.rs,usb_cdc.rs}
├── runtime/{mod.rs,slots.rs}
├── security/{mod.rs,fault.rs,launch.rs,scb.rs,svc.rs}
└── storage/{mod.rs,filesystem.rs}

crates/dali-amrn/src/
├── lib.rs                         # Stable crate facade and legacy re-exports
├── compatibility/mod.rs           # ABI-to-format compatibility rules
├── legacy/                        # Original fixed-origin package contract
│   ├── mod.rs, builder.rs, stream.rs, tests.rs
├── v2/                            # Segmented ABI v3 package contract
│   ├── mod.rs, tests.rs
├── v3/                            # Relocatable ABI v3 package contract
│   ├── mod.rs, apply.rs, codec.rs, wire.rs, tests.rs
└── v4/                            # Identity and selection metadata extension
    ├── mod.rs, tests.rs

crates/dali-cli/src/
├── main.rs
└── commands/
    ├── mod.rs, app.rs, build.rs, init.rs, new.rs, new_tests.rs
    ├── app_artifacts.rs, app_linker.rs, app_package.rs, app_relocations.rs
    ├── package.rs, inspect.rs, inspect_v3_tests.rs
    ├── device.rs, device_attach.rs, device_cdc.rs, device_console.rs
    ├── device_flash.rs, device_flash_transport.rs, device_info.rs
    ├── doctor.rs, target.rs, target_info.rs

crates/dali-cli/templates/app/
├── Cargo.toml.template, build.rs.template, config.toml.template
├── dali.toml.template, lib.rs.template, main.rs.template
└── memory.x.template, memory.v3.x.template

crates/dali-device/src/lib.rs
crates/dali-sdk/src/{lib.rs,svc.rs,svc_log.rs}
crates/dali-targets/src/lib.rs
crates/dali-targets/build.rs
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
- `kernel/src/platform/f405/sdio*.rs` owns the F405 PAC/HAL SDIO transport;
  bootstrap consumes it only through the platform facade.
- `kernel/src/drivers/` owns hardware-neutral driver contracts and adapters;
  it must not import a board PAC or HAL.
- `kernel/src/drivers/sdio.rs` owns the generic SDIO transport contract and
  block-reader adapter; platform code supplies the concrete transport.
- `targets/*.toml` owns declarative target facts; hardware implementations must
  consume those facts through generated target metadata instead of copying
  board constants into kernel policy.
- `kernel/src/storage/` owns SD/filesystem policy; generic block contracts live
  in `kernel/src/drivers/`; package parsing remains in
  `crates/dali-amrn/` and loading policy remains in `kernel/src/loader/`.
- `kernel/src/security/` owns privileged SVC dispatch, launch frames, and
  fault recovery.
- `crates/dali-targets/` generates target metadata from `targets/*.toml`; no
  board profile should be duplicated in CLI or kernel policy code.
- `crates/dali-cli/src/commands/` contains command-specific implementation;
  command documentation lives in `docs/cli/commands/`.
- `apps/` contains independently built validation applications and fixtures;
  these are not kernel modules.

## Generated and local-only content

The following are intentionally absent from this tree document:

- Cargo `target/` directories and compiled `.bin`/`.amrn` artifacts;
- generated linker scripts inside application build directories;
- editor settings, mounted SD-card paths, and debugger sessions;
- generated target registry output under Cargo `OUT_DIR`.

When a new tracked module or documentation area is added, update this file in
the same change so it remains a description of the repository rather than an
aspirational creation order.
