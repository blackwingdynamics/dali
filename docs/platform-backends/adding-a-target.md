# Adding a target

A target is a platform backend plus a typed target profile. The backend is the
only place that may depend on a vendor PAC/HAL, register map, linker layout,
interrupt table, clock tree, or board pin mapping.

## Required order

Contributors must follow this order:

1. Define and validate `targets/<profile>.toml` with the target identity,
   architecture, memory, clocks, capabilities, and backend-local resources.
2. Create a new backend directory for the board and keep all PAC/HAL,
   linker/memory, interrupt, and peripheral ownership code inside it.
3. Implement the hardware-neutral platform contracts supported by the target:
   reset, clock, GPIO, timer, logging, storage, USB, memory, watchdog, and
   protection behavior.
4. Generate or register the backend from typed target metadata. Do not edit
   existing kernel policy modules or another backend to select the new board.
5. Connect backend resources to the existing core interfaces without changing
   AMRN, ABI, boot, security, or storage policy.
6. Add manifest, host, embedded-target, and backend-isolation validation.
7. Document unsupported capabilities and update the relevant roadmap before
   expanding the compatibility contract.

## Directory-local ownership

An accepted backend should contain its implementation and target-specific
artifacts in one directory, for example:

```text
kernel/src/platform/<backend-id>/
├── mod.rs
├── board.rs
├── drivers/
├── interrupts.rs
└── memory.x
targets/<profile>.toml
```

The exact files depend on the platform, but a new board must not require
changes to existing backend directories or shared kernel policy. If a new
backend requires such a change, stop and refine the boundary before adding
the board.

## Required board documentation

Every new board must add a board-local documentation directory at
`docs/boards/<board-id>/`. The directory must contain the following minimum
set before the board can enter hardware acceptance:

```text
docs/boards/<board-id>/
├── README.md                 # Navigation index only
├── pinout-and-clocks.md      # Board identity, pins, clocks, and signals
├── memory-map.md             # Memory regions, linker ownership, and DMA limits
├── schematics.md             # Verified wiring record or schematic references
└── hardware-evidence.md      # Test setup, traces, results, and open limitations
```

These documents must state the board revision, MCU or programmable device,
target triple, power and ground requirements, debug connection, peripheral
pin mapping, clock sources, memory and DMA constraints, and supported or
unsupported capabilities. `hardware-evidence.md` must identify the firmware
revision, validation transport, expected and observed results, and the exact
evidence boundary for every claimed physical capability.

`README.md` is an index and must contain links to the board documents; it must
not become a second source of hardware facts. Additional focused documents
may be added when a board has special power, peripherals, FPGA constraints,
or bring-up procedures. Those documents must remain inside the same
`docs/boards/<board-id>/` directory and be linked from its README.

The board directory is authoritative for physical board facts. Generic
architecture, testing, and platform documents must link to it instead of
duplicating board-specific pinouts, wiring, clocks, or memory claims.

## Compile-time selection

The selected backend must be isolated at compile time. A backend-specific
Cargo feature or generated `cfg` value may select the implementation and its
PAC/HAL dependencies, but it must not introduce board policy branches into
shared kernel modules. Runtime discovery must never load or select hardware
implementations.

The selection mechanism must reject zero, multiple, or incompatible backend
selections at build time. Adding a board must remain a directory-local
backend/profile change from the perspective of shared kernel policy.

## Evidence gate

Each backend file or cohesive backend change is an independent checkpoint. Do
not mark it complete based only on compilation, host tests, simulation, or a
successful flash. For every checkpoint, record:

- the source revision and exact changed files;
- focused tests, target checks, formatting, Clippy, and `git diff --check`;
- the built firmware identity;
- board, power source, wiring, and transport;
- a real target boot trace and the expected result;
- comparison with the previously accepted baseline;
- unsupported capabilities and remaining limitations.

Hardware evidence is required for every claimed physical capability. If the
required board or measurement equipment is unavailable, the checkpoint stays
open and the limitation must be recorded. A new board is not accepted until
its own backend evidence exists.

## Boundary rule

Adding a target must not require editing unrelated core policy modules. The
kernel core must consume typed capabilities and hardware-neutral contracts;
the backend must provide the implementation. This rule is the portability
acceptance criterion, not merely a preferred code organization.
