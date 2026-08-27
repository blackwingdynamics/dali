# Fault boundary

MemManage, BusFault, UsageFault, and invalid exception-return paths must enter
a privileged kernel fault boundary. The boundary records a bounded fault
record, marks the current application terminated, and returns to a kernel-owned
control path. It must not unwind or reuse an application PSP as a kernel stack.
The recovery assembly uses a kernel-stack frame and does not reuse the
application PSP.

The feature-gated kernel now defines a bounded `FaultRecord` and diagnostic
MemManage, BusFault, and UsageFault handlers. These handlers report the SCB
status and return through a kernel-stack recovery frame to a bounded
kernel-owned recovery loop. They retire the faulted context rather than
returning to it; the feature-gated scheduler may then select another declared
ready context. They do not change ABI v2 behavior.

The additional kernel feature `abi-mpu` programs the descriptor-backed MPU
map during bootstrap and provides the feature-gated PendSV transition into the
prepared PSP frame. MPU activation is two-phase: while the privileged loader
copies the validated code and data segments, both application regions are
kernel-only and non-executable; after copying and zero-initialization complete,
the kernel changes the code region to unprivileged read/execute and the data
region to unprivileged read/write, execute-never, before entering the PSP
context. This ordering prevents the protection map from blocking a valid
kernel-owned load. The default kernel does not enable this path. It provides
feature-gated F405 processor-side context and MPU-switching evidence, but it
must not be treated as complete production application isolation because DMA
isolation, lifecycle policy, and application-to-application policy remain
open.

The default loader and SDK use ABI v2 packages with the direct `ServiceTable`
entry contract. The feature-gated ABI v3 loader validates and copies the
separate segments, prepares a kernel-owned launch frame, and materializes its
basic exception frame inside the validated application stack reservation. Only
the explicitly enabled `abi-mpu` path selects PSP, activates the descriptor
backed MPU map, and enters through PendSV; the default kernel does none of
these. ABI v3 host packages cannot be treated as production-isolated because
complete lifecycle, DMA, and application-to-application policy remain open.
