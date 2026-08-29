# Target Profile Ownership and Boundaries

Target profiles are declared in repository-level TOML manifests under
`targets/`. The manifest is the source of truth for manufacturer-provided
board metadata and the Dali compatibility identifiers consumed by host tools.

## Ownership

The boundary is intentionally split:

- `targets/*.toml` declares board facts: MCU, clocks, memory regions, pins,
  alternate functions, USB, optional storage, application compatibility, and
  explicit capabilities.
- `crates/dali-targets/build.rs` parses and validates every manifest at build
  time, then generates a typed `no_std` registry for host consumers.
- `crates/dali-cli` consumes the generated registry for target selection and
  diagnostics. CLI commands do not define board constants.
- The target scaffold command can create a non-production backend template and
  documentation checklist from an existing profile without overwriting files.
  The template renders the declared board values as named constants before the
  typed HAL mapping is reviewed.
- `crates/dali-boards/<board-crate>/` maps the selected profile to typed HAL
  peripherals and owns the compile-time GPIO, RCC, DMA, and peripheral
  initialization. `kernel/src/platform/mod.rs` exposes the hardware-neutral
  facade consumed by kernel policy.

The manifest does not perform runtime hardware autodetection. A kernel must
know its board mapping before clock and peripheral initialization. Future
device discovery may use explicit hardware identifiers, but it cannot infer
safe pin ownership from a generic TOML file at runtime.
