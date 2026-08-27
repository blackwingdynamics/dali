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

## Remaining security work

- pre-reset ROM/first-stage bootloader verification for the kernel image;
- production key custody and debug-lock enforcement;
- broader MPU-backed isolation and multi-application ownership where supported;
- arbitrary DMA-controller/peripheral isolation;
- cross-target watchdog failure semantics and timeout evidence;
- power-loss-safe update and rollback acceptance;
- security review of package parsing and storage access.

Security claims must be added only after the corresponding mechanism and test evidence exist.

The future multi-developer package ecosystem is defined in
`docs/package-distribution/README.md`. That document is the source of truth for the
Dali root, repository metadata roles, developer delegations, signed trust-store
updates, revocation, rotation, rollback, and offline installation. The current
F405 static release trust-anchor path is a precursor to that design and must
not be described as a completed multi-developer registry.

## Production trust contract slice

The hardware-neutral `dali-metadata` policy now defines production root-key
custody requirements: at least three distinct root public keys, a two-of-three
root threshold, and no private key material on the target. It also defines the
`dali.secure-boot.v1` kernel-image descriptor and admission checks for target,
version, length, SHA-256, and root-role signatures.

This is a policy and codec boundary only for the kernel image. The current F405
startup path does not yet verify a signed kernel image before reset-vector
execution, so pre-reset Secure Boot and production root custody remain
unaccepted hardware claims. Separately, the F405 repository boot path verifies
the signed `bundle.manifest` through the Root-declared Bundle role before
applying its generation admission policy. Existing trust-store verification
continues to apply rotation overlap, signed revocation state, metadata expiry
when trusted time is available, and durable generation rollback checks.

## F405 isolation foundation (feature-gated, context-switching hardware evidence)

The first isolation milestone is limited to the F405 backend and its declared
application contexts. It uses the Cortex-M4 privilege model and MPU to
prevent unprivileged application code from accessing kernel RAM, kernel
runtime stack, or ordinary peripheral registers. Application services use an
SVC gateway rather than direct privileged function calls.

The feature-gated ABI v3 path now implements the per-context MPU map, explicit
system-fault exception enablement, unprivileged PSP launch, SysTick/PendSV
context switching, slot-specific MPU region switching, SVC logging gateway,
and kernel-owned fault recovery.
The fault-injection fixtures cover kernel-RAM reads and writes,
peripheral-MMIO reads and writes, execute-never instruction fetches, invalid
PSP exception-entry bounds, malformed SVC requests, and a precise BusFault.
F405 hardware evidence confirms processor-side rejection and kernel recovery
for each of those cases, including bounded fault-frame decoding for the
MemManage and BusFault paths.

This milestone still does not claim a secure kernel, complete sandbox,
complete fault isolation, arbitrary DMA-controller isolation, confidentiality,
or pre-reset kernel-image authenticity.
The no-frame result is a handler/recovery-boundary trace rather than a complete
automatic restart or rollback. The current policy requires a manual reset
after application termination, keeps the read-only package boundary
rollback-free, and arms a watchdog before opaque SDIO initialization. After
transport initialization returns, the watchdog is serviced only by the kernel
heartbeat, scheduler tick, or a bounded valid-progress hook. A watchdog reset
now selects Safe Mode before
storage/package loading, and F405 hardware evidence confirms the watchdog
reset log, recovery transition, skipped application loading, and return to the
kernel heartbeat. The F405 test-only no-feed path has also been hardware-tested;
cross-target feed-failure semantics remain tracked separately. The
initial valid-service
authorization policy and repeated invalid-PSP fault-status clearing are
hardware-tested. MPU protection applies to
processor accesses; DMA buffer ownership and kernel memory safety require
separate controls. The DMA contract now validates every F405 SDIO transfer
buffer against the target-declared DMA region and keeps the mutable borrow
exclusive for the active transfer. This is a kernel transport boundary, not
yet proof that arbitrary future DMA-capable peripherals or application-owned
DMA are isolated.
