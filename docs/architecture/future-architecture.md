# 11. Future kernel architecture

After the MVP, the platform can grow toward:

- priority-based tasks and periodic scheduling;
- typed channels and fixed-size event queues;
- service discovery and capability policy;
- watchdog heartbeats and deadline monitoring;
- application lifecycle management;
- production multi-application lifecycle, ownership, and resource policy on
  top of the current feature-gated MPU/context-switching capability;
- signed cartridges and secure boot;
- A/B updates and rollback;
- `dali` CLI workflows.

Safety-critical behavior such as emergency stop, watchdog policy, power handling, and actuator limits must remain kernel-owned even when mission logic is supplied by an application.

## Isolation design constraints

The STM32F405 protection and memory constraints are described in the
[STM32F405 memory map](../boards/stm32f405/memory-map.md). The portable
architecture must model such constraints through typed backend capabilities;
it must not make the F405 layout universal. PIC compiler flags alone do not
define a relocatable raw AMRN contract. Explicit relocation metadata now
defines the implemented movable-cartridge path; an alternative PIC/RWPI ABI
remains future work.

## 12. Architectural principles

- Keep each reference platform narrow and fully testable without coupling the
  kernel core to that platform.
- Prefer explicit contracts over implicit linker or memory assumptions.
- Keep realtime paths bounded and allocation-free where required.
- Treat native application execution as trusted until isolation exists.
- Use audited external implementations for cryptography and filesystems where appropriate.
- Make every security claim match an implemented mechanism.
- Keep the kernel, SDK, CLI, cartridge format, and application APIs versioned independently.

## 13. Universal platform boundary

Dali OS is organized around a hardware-neutral kernel core and independently
cartridged platform backends. The core owns boot policy, runtime policy, loader
policy, security policy, and hardware-neutral driver contracts. A platform
backend owns vendor PAC/HAL dependencies, register access, interrupt vectors,
linker and memory definitions, clock setup, pin mapping, and peripheral
ownership.

A new board must be implemented in its own backend directory and target
profile. It must not require edits to existing kernel policy modules or to an
existing backend. Backend discovery, capability selection, and generated
profiles must be driven by typed target metadata and must reject ambiguous or
unsupported configurations at build time.

The STM32F405 backend remains the first reference implementation, not the
definition of the kernel architecture. Future Pico, FPGA SoC, and other MCU
backends must consume the same hardware-neutral contracts while keeping their
hardware-specific unsafe code and memory model inside their own boundaries.

## 14. Compile-time backend and protection model

Backend selection is a compile-time concern. A target profile selects exactly
one backend through a dedicated backend feature or an equivalent generated
configuration value. The selection mechanism must isolate unselected PAC/HAL
code without introducing runtime hardware discovery or board-specific policy
branches in the shared kernel.

Memory placement is backend-owned. Static buffers, `#[link_section]`
attributes, linker symbols, DMA regions, and processor-specific addresses
must remain in the selected backend and its linker/memory definition. Shared
policy may consume typed regions from the target profile but must not assume
CCM, SRAM, or any other processor-specific memory map.

Security policy must describe typed protection regions and permissions through
a hardware-neutral `MemoryProtectionProvider` boundary. ARM MPU, ARM
protection extensions, RISC-V PMP, and other register programming belong to
their respective backend. The core must never write processor-specific
protection registers directly.
