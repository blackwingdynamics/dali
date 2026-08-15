# Dali Target Profiles

Target profiles are declared in repository-level TOML manifests under
`targets/`. The manifest is the source of truth for manufacturer-provided
board metadata and the Dali compatibility identifiers consumed by host tools.

## Ownership

The boundary is intentionally split:

- `targets/*.toml` declares board facts: MCU, clocks, memory regions, pins,
  alternate functions, USB, storage, AMRN target ID, and ABI version.
- `crates/dali-targets/build.rs` parses and validates every manifest at build
  time, then generates a typed `no_std` registry for host consumers.
- `crates/dali-cli` consumes the generated registry for target selection and
  diagnostics. CLI commands do not define board constants.
- `kernel/src/board/` maps the selected profile to typed HAL peripherals and
  owns the compile-time GPIO, RCC, DMA, and peripheral initialization.

The manifest does not perform runtime hardware autodetection. A kernel must
know its board mapping before clock and peripheral initialization. Future
device discovery may use explicit hardware identifiers, but it cannot infer
safe pin ownership from a generic TOML file at runtime.

## Manifest contract

Each supported target manifest contains these sections:

| Section | Purpose |
| --- | --- |
| `profile` | Stable profile name, board name, MCU, Rust target, AMRN target ID, and ABI version |
| `clock` | HSE, system, and USB clock frequencies in hertz |
| `memory` | Kernel, application, and runtime SRAM regions |
| `status_led` | Logical status LED port, pin, alternate function, and polarity |
| `usb` | USB controller and D-/D+ pins with alternate functions |
| `storage` | Storage controller, bus width, clock, command, and data pins with alternate functions |

Values in this manifest are configuration and board-definition data, not
implementation literals. Adding a profile requires a new manifest, a kernel
backend mapping, target checks, and the relevant hardware documentation and
acceptance evidence.

## Validation rules

The target registry build rejects:

- missing or empty profile identity fields;
- duplicate profile names;
- duplicate AMRN target identifiers;
- zero contract identifiers;
- storage widths outside the supported four-line representation;
- missing or malformed TOML fields.

The generated registry is not committed. Rebuilding `dali-targets` regenerates
it from the manifests and tracks each manifest with Cargo rerun directives.
