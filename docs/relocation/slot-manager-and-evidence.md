# Slot-manager prerequisite

The target memory contract provides an ordered slot table. The current F405
manifest uses two 16 KiB
code/data pairs, keeps 32 KiB of DMA-accessible SRAM for kernel transport
buffers, and moves kernel runtime/static state/stack to the 64 KiB CCM region.
The final 32 KiB remains the target's runtime/shared-memory reserve.

The manifest-owned slot table now drives the application linker, package
contract, loader, and MPU slot boundaries. The slot manager must never infer
slots from arithmetic on addresses. The feature-gated ABI v3 scheduler can
switch declared contexts on F405; concurrent allocation, restart, and
production lifecycle policy remain future work.

## Required evidence before implementation

The implementation must not start until fixtures prove that the selected
toolchain can retain and identify the intended relocation records for:

- a code reference whose destination moves with the code segment;
- a writable global in initialized data;
- a zero-initialized global;
- the application entry point;
- an unsupported or overflowing relocation that is rejected.

The fixture must be built at two distinct linked bases, and host tests must
show that applying the same relocation metadata produces equivalent runtime
addresses at two valid slots. Hardware execution is required after the host
contract tests pass.

The relocation fixture currently links against slot 0, declares `slot =
"slot1"` in its application manifest, and produces a format 3 package whose
code/data load addresses target slot 1. The CLI can build and inspect this
package on the host. Slot 1 execution on the F405 remains a separate hardware
acceptance step.
