# Security Model

## MVP guarantees

The MVP can provide:

- fixed target validation;
- payload size and address bounds checks;
- CRC32-based corruption detection;
- deterministic rejection of malformed packages.

## MVP non-guarantees

The MVP does not provide:

- sandboxing;
- memory isolation;
- privilege separation;
- package authenticity;
- secure boot;
- encryption;
- anti-rollback;
- application fault isolation.

Native application code runs in the kernel's address space and must therefore be treated as trusted. The MVP disables application-owned interrupts and provides only the bounded ABI v2 logging service; it provides no memory isolation or fault isolation.

## Post-MVP security work

- signed kernel and application images;
- secure key storage;
- signature and certificate policy;
- anti-rollback counters;
- production debug lock;
- MPU-backed isolation where supported;
- watchdog and recovery policy;
- atomic update and rollback;
- security review of package parsing and storage access.

Security claims must be added only after the corresponding mechanism and test evidence exist.

## Planned F405 isolation foundation (not implemented)

The first isolation milestone is limited to one F405 application. It will use
the Cortex-M4 privilege model and MPU to prevent unprivileged application code
from accessing kernel RAM, kernel runtime stack, or ordinary peripheral
registers. Application services will use an SVC gateway rather than direct
privileged function calls.

This milestone will not claim a secure kernel, complete sandbox, DMA isolation,
confidentiality, authenticity, or fault isolation until the corresponding
mechanisms and F405 fault-injection evidence exist. MPU protection applies to
processor accesses; DMA buffer ownership and kernel memory safety require
separate controls.
