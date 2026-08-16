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

## F405 isolation foundation (feature-gated, first hardware evidence recorded)

The first isolation milestone is limited to one F405 application. It will use
the Cortex-M4 privilege model and MPU to prevent unprivileged application code
from accessing kernel RAM, kernel runtime stack, or ordinary peripheral
registers. Application services will use an SVC gateway rather than direct
privileged function calls.

The feature-gated ABI v3 path now implements the single-application MPU map,
explicit system-fault exception enablement, unprivileged PSP launch, SVC
logging gateway, and kernel-owned fault recovery.
The `apps/dali-app-fault-kernel` fixture provides the first controlled fault
injection. This milestone still does not claim a secure kernel, complete
sandbox, complete fault isolation, DMA isolation, confidentiality, or
authenticity. A first F405 run confirmed rejection of an unprivileged
kernel-RAM read and return to kernel recovery, but fault-frame decoding, write
and peripheral rejection, PSP bounds, invalid execution, and multi-application
isolation remain unverified. MPU protection applies to processor accesses;
DMA buffer ownership and kernel memory safety require separate controls.
