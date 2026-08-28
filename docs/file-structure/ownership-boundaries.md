# Ownership boundaries

- `kernel/src/security/mpu/` owns MPU descriptors, memory-layout validation,
  and privileged activation of application regions.
- `kernel/src/platform/mod.rs` owns the hardware-neutral platform facade and
  backend selection. Each `kernel/src/platform/<backend-id>/` directory owns
  one target backend's PAC/HAL integration, clocks, pins, interrupts,
  linker/memory definitions, and peripheral resources. Backend ownership and
  contributor workflow are defined in `docs/platform-backends/README.md`.
- `kernel/src/platform/f405/sdio.rs` and `kernel/src/platform/f405/sdio_raw/`
  own the F405 PAC/HAL SDIO transport; bootstrap consumes it only through the
  platform facade. Other boards must provide their own backend-local storage
  adapter rather than modifying the F405 implementation.
- `kernel/src/platform/f405/watchdog/` owns the F405 IWDG and RCC reset-cause
  register adapter; watchdog policy remains in `kernel/src/runtime/watchdog/`.
  Equivalent adapters for other targets belong in their own backend
  directories.
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
  persistence state machine. Cartridge parsing remains in
  `crates/dali-amrn/` and bootstrap loading policy remains in
  `kernel/src/bootstrap/loading/` while AMRN execution contracts remain in
  `kernel/src/loader/`.
- `kernel/src/bootstrap/startup/` owns logging/banner and watchdog startup;
  `kernel/src/bootstrap/lifecycle/` owns heartbeat/status behavior;
  `kernel/src/bootstrap/storage/` owns storage initialization and the
  feature-gated durable-artifact acceptance path; and
  `kernel/src/bootstrap/loading/` owns cartridge-to-runtime handoff.
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
  processor-specific exception handlers and MPU switching belong to the
  selected backend and must not be implemented as F405 branches in shared
  runtime policy.
- `crates/dali-targets/` generates target metadata from `targets/*.toml`; no
  board profile should be duplicated in CLI or kernel policy code.
- `targets/<profile>.toml` and its generated typed profile select one backend;
  adding a target must add a backend directory and profile without changing
  existing core policy modules or backend directories.
- `crates/dali-cli/src/commands/` owns top-level dispatch and groups related
  commands by domain. `app/`, `device/`, `metadata/`, and `target/` each expose
  a `mod.rs` dispatcher; complex commands may contain focused helper modules.
  Standalone commands remain directly under `commands/`. Command
  documentation lives in `docs/cli/commands/`.
- `apps/` contains independently built validation applications and fixtures;
  these are not kernel modules.
- `docs/testing/` contains the categorized testing procedures and evidence
  records indexed by `docs/testing/README.md`.
