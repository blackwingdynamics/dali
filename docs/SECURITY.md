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

Native application code runs in the kernel's address space and must therefore be treated as trusted. The MVP disables application-owned interrupts and provides no shared service or logging ABI.

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
