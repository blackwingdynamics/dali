# Platform Backend Ownership and Boundaries

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

## Backend isolation contract

Each backend is an independently owned platform package. Its directory owns
all target-specific implementation details, including:

- PAC/HAL dependencies and register access;
- clock, reset, GPIO, pin, interrupt, and peripheral setup;
- linker script, memory regions, DMA placement, and processor fault entry;
- backend-local driver adapters and resource ownership;
- target-specific build, flash, debug, and hardware-evidence procedures.

The shared kernel may consume only typed capabilities and hardware-neutral
driver contracts. It must not contain conditional branches for individual
boards, duplicate backend constants, or fallback behavior that silently
selects a different board.

Adding a board must therefore add a new backend directory and target profile,
then let typed metadata drive backend discovery and selection. Existing core
policy modules and existing backend directories must remain unchanged. If a
new board cannot satisfy this rule, the platform boundary is incomplete and
the architecture must be reviewed before implementation continues.

## Change and evidence boundary

Backend implementation changes must remain separate from policy changes. A
backend change must preserve the kernel's boot order, ABI, loader behavior,
security policy, storage contract, and public driver APIs. Unsupported
capabilities must be declared in the target profile and returned through
typed errors rather than hidden fallbacks.

Every backend change is an independent acceptance checkpoint. Its record must
include the exact revision, changed files, validation commands, firmware
identity, board and wiring details, expected trace, observed trace, and
remaining limitations. Compilation, host tests, simulation, and successful
flashing are necessary development checks but are not hardware acceptance.

## Memory and protection ownership

Static buffers, `#[link_section]` attributes, linker symbols, DMA regions, and
processor-specific memory addresses belong to the selected backend. Shared
kernel policy may use typed target regions but must not assume an F405 CCM/SRAM
layout or any other processor memory map.

Shared security policy consumes typed protection regions and permissions
through a `MemoryProtectionProvider` boundary. F405 MPU, ARM protection
extensions, RISC-V PMP, and equivalent register programming must remain in the
corresponding backend.
