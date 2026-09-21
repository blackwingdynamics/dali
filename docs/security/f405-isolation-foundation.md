# F405 isolation foundation (feature-gated, context-switching hardware evidence)

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
after application termination, keeps the read-only cartridge boundary
rollback-free, and arms a watchdog before opaque SDIO initialization. After
transport initialization returns, the watchdog is serviced only by the kernel
heartbeat, scheduler tick, or a bounded valid-progress hook. A watchdog reset
now selects Safe Mode before
storage/cartridge loading, and F405 hardware evidence confirms the watchdog
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
