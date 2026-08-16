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
│       ├── main.rs                # Kernel entry and bootstrap call
│       ├── abi.rs                 # Central active ABI selector
│       ├── board/                 # Shared MPU descriptors
│       ├── platform.rs            # Platform facade and target entry points
│       ├── platform/stm32f405.rs  # F405 target profile and IRQ bindings
│       ├── platform/stm32f405_board.rs # F405 hardware backend
│       ├── platform/stm32f405_sdio.rs # F405 SDIO block-device adapter
│       ├── platform/stm32f405_sdio_raw.rs # F405 SDIO register transport
│       ├── bootstrap/             # Startup, storage status, heartbeat
│       ├── loader.rs              # AMRN v1/v2/v3 dispatch and ABI services
│       ├── loader/v3.rs           # Fixed-origin ABI v3 loader
│       ├── loader/v3_relocatable.rs # Feature-gated format 3 loader
│       ├── logging/               # Facade, RTT, USB CDC backend
│       ├── security/              # MPU, SVC, launch, and fault recovery
│       └── storage/               # Block types and read-only filesystem
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
│   └── dali-app-fault-psp/
├── crates/
│   ├── dali-amrn/                 # AMRN v1, v2, and v3 contracts
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
├── main.rs
├── abi.rs
├── board/{mod.rs,mpu.rs}
├── platform.rs, platform/{stm32f405.rs,stm32f405_board.rs,stm32f405_sdio.rs,stm32f405_sdio_raw.rs}
├── bootstrap/{mod.rs,heartbeat.rs,status.rs}
├── loader.rs
├── loader/{v3.rs,v3_relocatable.rs}
├── logging/{mod.rs,rtt.rs,usb_cdc.rs}
├── security/{mod.rs,fault.rs,launch.rs,svc.rs}
└── storage/{mod.rs,filesystem.rs}

crates/dali-amrn/src/
├── lib.rs, builder.rs, stream.rs, tests.rs
├── v2.rs, v2_tests.rs
└── v3.rs, v3_apply.rs, v3_codec.rs, v3_tests.rs, v3_wire.rs

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

- `kernel/src/board/` owns processor-neutral MPU descriptors and activation
  helpers shared by the selected platform backend.
- `kernel/src/platform.rs` and `kernel/src/platform/` own the platform facade
  and target-specific entry points; backend ownership and contributor workflow are defined in
  `docs/PLATFORM_BACKENDS.md`.
- `kernel/src/platform/stm32f405_sdio*.rs` owns the F405 PAC/HAL SDIO transport;
  bootstrap consumes it only through the platform facade.
- `targets/*.toml` owns declarative target facts; hardware implementations must
  consume those facts through generated target metadata instead of copying
  board constants into kernel policy.
- `kernel/src/storage/` owns SD/filesystem access; package parsing remains in
  `crates/dali-amrn/` and loading policy remains in `kernel/src/loader.rs`.
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
