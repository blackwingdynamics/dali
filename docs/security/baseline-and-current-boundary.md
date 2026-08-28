# Baseline ABI v2 guarantees

The baseline can provide:

- fixed target validation;
- payload size and address bounds checks;
- CRC32-based corruption detection;
- deterministic rejection of malformed packages.

## MVP non-guarantees

The baseline ABI v2 MVP does not provide:

- sandboxing;
- memory isolation;
- privilege separation;
- package authenticity;
- secure boot;
- encryption;
- anti-rollback;
- application fault isolation.

Native ABI v2 application code runs in the kernel's address space and must
therefore be treated as trusted. The baseline disables application-owned
interrupts and provides only the bounded ABI v2 logging service.

## Current isolation boundary

The feature-gated ABI v3 path provides a tested processor boundary on the
STM32F405: unprivileged Thread mode, PSP ownership, MPU code and data
permissions, an SVC gateway, and kernel-owned fault recovery. F405 silicon
also hardware-verifies PSP/SysTick/PendSV context switching and slot-specific
MPU region switching for declared application contexts. This path is not
enabled by default and does not turn the baseline ABI v2 path into a sandbox
or complete production multi-application isolation.

The F405 evidence covers kernel/peripheral access rejection, execute-never
rejection, invalid PSP entry, SVC rejection, precise BusFault decoding,
repeated invalid-PSP recovery, and the no-frame HardFault handler boundary.
The no-frame trace proves the handler/recovery boundary, but not automatic
application restart or rollback.

The following remain outside the current guarantee boundary:

- complete arbitrary-peripheral DMA-controller isolation beyond the tested
  kernel-owned F405 SDIO path;
- complete multi-application lifecycle and application-to-application policy;
- pre-reset kernel-image Secure Boot and immutable boot-anchor enforcement;
- production key custody, debug-lock policy, and power-loss recovery;
- confidentiality for native application images.

The configured feature-gated F405 repository path provides AMRN v5
signed-package/repository verification and durable generation anti-rollback
checks. Trust-store revocation policy is implemented and host-tested; target-
side revoked-developer-key acceptance remains a separate hardware test. The
F405 evidence is limited to that configured path and must not be generalized
to every target or package mode.
