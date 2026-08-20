# Security Model

## Baseline ABI v2 guarantees

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

The feature-gated ABI v3 path provides a tested single-application processor
boundary on the STM32F405: unprivileged Thread mode, PSP ownership, MPU code
and data permissions, an SVC gateway, and kernel-owned fault recovery. This is
not enabled by default and does not turn the baseline ABI v2 path into a
sandbox.

The F405 evidence covers kernel/peripheral access rejection, execute-never
rejection, invalid PSP entry, SVC rejection, precise BusFault decoding,
repeated invalid-PSP recovery, and the no-frame HardFault handler boundary.
The no-frame trace proves the handler/recovery boundary, but not automatic
application restart or rollback.

The following remain outside the current guarantee boundary:

- DMA isolation and DMA ownership enforcement;
- multi-application package loading and application-to-application isolation;
- explicit watchdog-reset Safe Mode and recovery behavior;
- package authenticity, secure boot, confidentiality, and anti-rollback;
- production debug-lock and key-storage policy.

## Post-MVP security work

- signed kernel and application images;
- secure key storage;
- signature and certificate policy;
- anti-rollback counters;
- production debug lock;
- broader MPU-backed isolation and multi-application ownership where supported;
- hardware watchdog and timeout implementation;
- atomic update and rollback;
- security review of package parsing and storage access.

Security claims must be added only after the corresponding mechanism and test evidence exist.

The future multi-developer package ecosystem is defined in
`docs/PACKAGE_DISTRIBUTION.md`. That document is the source of truth for the
Dali root, repository metadata roles, developer delegations, signed trust-store
updates, revocation, rotation, rollback, and offline installation. The current
F405 static release trust-anchor path is a precursor to that design and must
not be described as a completed multi-developer registry.

## F405 isolation foundation (feature-gated, single-application hardware evidence)

The first isolation milestone is limited to one F405 application. It uses
the Cortex-M4 privilege model and MPU to prevent unprivileged application code
from accessing kernel RAM, kernel runtime stack, or ordinary peripheral
registers. Application services use an SVC gateway rather than direct
privileged function calls.

The feature-gated ABI v3 path now implements the single-application MPU map,
explicit system-fault exception enablement, unprivileged PSP launch, SVC
logging gateway, and kernel-owned fault recovery.
The fault-injection fixtures cover kernel-RAM reads and writes,
peripheral-MMIO reads and writes, execute-never instruction fetches, invalid
PSP exception-entry bounds, malformed SVC requests, and a precise BusFault.
F405 hardware evidence confirms processor-side rejection and kernel recovery
for each of those cases, including bounded fault-frame decoding for the
MemManage and BusFault paths.

This milestone still does not claim a secure kernel, complete sandbox,
complete fault isolation, DMA isolation, confidentiality, or authenticity.
The no-frame result is a handler/recovery-boundary trace rather than a complete
automatic restart or rollback. The current policy requires a manual reset
after application termination, keeps the read-only package boundary
rollback-free, and arms a watchdog only through the kernel heartbeat feed
owner or scheduler tick. A watchdog reset now selects Safe Mode before
storage/package loading, and F405 hardware evidence confirms the watchdog
reset log, recovery transition, skipped application loading, and return to the
kernel heartbeat. F405 feed-failure evidence remains tracked separately. The
initial valid-service
authorization policy and repeated invalid-PSP fault-status clearing are
hardware-tested. MPU protection applies to
processor accesses; DMA buffer ownership and kernel memory safety require
separate controls. The DMA contract now validates every F405 SDIO transfer
buffer against the target-declared DMA region and keeps the mutable borrow
exclusive for the active transfer. This is a kernel transport boundary, not
yet proof that arbitrary future DMA-capable peripherals or application-owned
DMA are isolated.
