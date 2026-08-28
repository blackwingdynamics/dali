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
