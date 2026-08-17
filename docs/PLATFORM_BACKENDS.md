# Platform Backend Contract

This document defines how Dali OS can accept additional MCU and board targets
without duplicating hardware facts throughout the kernel or changing the
kernel's hardware-neutral policy.

## Ownership

The kernel core owns policy and contracts:

- AMRN parsing, validation, relocation policy, and ABI compatibility;
- filesystem and storage policy at the bounded block-reader boundary;
- logging, service dispatch, application lifecycle, and fault policy;
- target metadata consumption after a profile has been selected.

A platform backend owns hardware adaptation:

- reset-time peripheral acquisition and clock setup;
- GPIO, status LED, console, storage, USB, and interrupt wiring;
- linker memory and DMA placement for the selected MCU;
- MPU and fault-entry details that depend on the processor;
- the supported build, flash, and debug route.

The backend may depend on its MCU PAC and HAL. Core policy must not import a
concrete PAC, board identifier, pin number, or transport identifier.

## Target profile versus backend

`targets/*.toml` is the source of truth for declarative target facts: memory
ranges, clocks, pins, peripherals, package compatibility, and protection
capabilities. The generated target registry exposes typed metadata to host and
kernel build code.

The backend maps those facts to executable hardware operations. The
`kernel/src/platform.rs` facade defines the crate-private `Backend` contract
and exposes only stable kernel-facing operations;
backend resource structs, PAC types, pin tuples, and clock objects stay behind
that facade. It must not
silently duplicate configurable values in Rust. A value that differs between
targets belongs in the manifest or in a documented processor-specific
implementation rule.

## Adding a target

Contributors should follow this order:

1. Add and validate `targets/<profile>.toml`.
2. Add a backend selected by a Cargo feature and keep it behind the platform
   facade.
3. Implement reset, clock, GPIO, logging, storage, USB, memory, and protection
   behavior supported by the target.
4. Connect the backend resources to the existing core interfaces without
   changing AMRN or ABI policy accidentally.
5. Add manifest, host, and embedded-target validation; add hardware evidence
   for every claimed physical behavior.
6. Document unsupported capabilities and update the roadmap before expanding
   the compatibility contract.

Adding a target must not require editing unrelated core policy modules. If it
does, the boundary is incomplete and should be refined before adding more
boards.

## Compatibility rule

Adding a backend is additive: existing F405 behavior and its package contract
must remain unchanged. Changes to AMRN, ABI, memory layout, or security
semantics require an explicit contract update, tests, documentation, and a
versioning decision.
