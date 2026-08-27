# 11. Future kernel architecture

After the MVP, the platform can grow toward:

- priority-based tasks and periodic scheduling;
- typed channels and fixed-size event queues;
- service discovery and capability policy;
- watchdog heartbeats and deadline monitoring;
- application lifecycle management;
- production multi-application lifecycle, ownership, and resource policy on
  top of the current feature-gated MPU/context-switching capability;
- signed packages and secure boot;
- A/B updates and rollback;
- `dali` CLI workflows.

Safety-critical behavior such as emergency stop, watchdog policy, power handling, and actuator limits must remain kernel-owned even when mission logic is supplied by an application.

## Isolation design constraints

The STM32F405 Cortex-M4 provides eight unified MPU regions. The current
application region starts at `0x20008000` and spans 64 KiB, so an isolation
implementation must represent it with aligned power-of-two regions or change
the memory contract. Code, writable data, and the application PSP stack may
require different permissions and execute-never attributes.

The F405 also exposes CCM RAM at `0x10000000`; the current linker contract
places ordinary kernel runtime/static state there while SDIO and USB DMA
buffers remain in DMA-accessible SRAM. PIC compiler flags alone do not define a
relocatable raw AMRN contract. Explicit relocation metadata now defines the
implemented movable-package path; an alternative PIC/RWPI ABI remains future
work.

## 12. Architectural principles

- Keep the first reference platform narrow and fully testable.
- Prefer explicit contracts over implicit linker or memory assumptions.
- Keep realtime paths bounded and allocation-free where required.
- Treat native application execution as trusted until isolation exists.
- Use audited external implementations for cryptography and filesystems where appropriate.
- Make every security claim match an implemented mechanism.
- Keep the kernel, SDK, CLI, package format, and application APIs versioned independently.
