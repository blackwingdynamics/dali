# Target application memory contract

The target manifest owns an ordered slot table. The current F405 manifest
decomposes its 64 KiB application pool into two aligned slots for v3:

```text
0x20008000 - 0x2000BFFF   Slot 0 code, 16 KiB, read/execute
0x2000C000 - 0x2000FFFF   Slot 0 data/PSP, 16 KiB, read/write, XN
0x20010000 - 0x20013FFF   Slot 1 code, 16 KiB, read/execute
0x20014000 - 0x20017FFF   Slot 1 data/PSP, 16 KiB, read/write, XN
```

The v3 application linker contract places the current application in the
manifest's active slot (slot 0). The loader and MPU validate that slot before
copying or launching. Slot 1 is declared and validated by the target contract.
The feature-gated ABI v3 scheduler can execute declared contexts and switch
their PSP/MPU state on F405; production lifecycle, replacement, restart, and
application-to-application policy remain future work. The v2 single-image
linker contract remains unchanged for existing packages.

The initial eight-region budget is:

| Region | Planned use | Unprivileged permission |
| ---: | --- | --- |
| 0 | Kernel SRAM | no access |
| 1 | Kernel runtime and stack SRAM | no access |
| 2 | Application code and read-only data | read/execute |
| 3 | Application data and PSP | read/write, XN |
| 4 | Ordinary peripheral registers | no access |
| 5 | Kernel Flash policy boundary | no access unless explicitly revised |
| 6 | Future shared service memory | reserved |
| 7 | Future shared service memory | reserved |

The MPU encoding treats application SRAM as normal cacheable,
bufferable memory and the ordinary peripheral window as shareable device
memory. These attributes are encoded by the board descriptor; writing the MPU
registers and selecting the unprivileged context remain separate steps.

This map is enabled only by the explicit `abi-mpu` feature and has been
hardware-tested for the documented per-context fault cases and slot-specific
region switching. It is not the default kernel configuration, and the
privileged background map and default-memory attributes must continue to
prevent bypass of the no-access boundaries.

## ABI v3 package and linker contract

ABI v3 packages use AMRN format version `2`; the format revision is required
because the v1 fixed header cannot represent separate code/data segments and
runtime stack reservations. The v2 package contract is defined in
`docs/amrn-format/README.md`. The `dali-amrn` crate provides host-side parsing and
construction. The CLI can build and inspect ABI v3 packages, and the kernel has
a feature-gated streaming validator/copy path. The default kernel remains
ABI v2-only; the feature-gated path is experimental and its hardware evidence
is complete for the documented F405 context-switch and single-context fault
cases. It does not provide the complete production multi-application
lifecycle or claim the remaining security guarantees.

For a target selected through the manifest, the linker must emit:

- code and read-only data within the active slot's declared code region;
- initialized data within the active slot's declared data region;
- zero-initialized data followed by the PSP stack within that same data
  region;
- a word-aligned entry offset relative to the code origin;
- metadata sufficient to validate code size, initialized-data size,
  zero-data size, and PSP stack size.

The package payload stores code followed by initialized data. Zero data and
the PSP stack are reservations, not file bytes. The loader must copy the two
file segments only after validating every region bound, then clear the
zero-data range and construct the PSP launch frame. This is a fixed-address
single-application contract. Explicit relocation is defined by AMRN format
version 3 and hardware-tested separately; compiler PIC or RWPI alone is not
treated as an AMRN relocation contract. Multiple-package execution remains
deferred.

Format version 3 is the movable ABI v3 contract. Its host-side header,
relocation-entry validation, CLI extraction, package emission, and inspection
are defined in docs/amrn-format/README.md and implemented in dali-amrn::v3 and the
CLI. The kernel accepts and applies it only with the explicit
`abi-relocation` feature; the default kernel path remains unchanged. The
current feature-gated loader still uses the target manifest's declared origins
and does not select multiple application slots. The relocation table is
intentionally a new format revision rather than an interpretation of v2
address fields.

The launch frame is kernel-generated. Its PC is the validated Thumb entry,
its PSP is within the declared stack bounds, its unused argument registers are
cleared, and its link value cannot return into kernel code. The package cannot
provide an exception-return value or a privileged function pointer. ABI v2
packages remain on the existing direct `ServiceTable` entry path and must not
be interpreted as ABI v3 packages.

Kernel SRAM, kernel runtime/stack SRAM, and ordinary peripheral registers are
not application-accessible. The v3 design does not claim DMA isolation,
confidentiality of readable Flash, package authenticity, or recovery from
arbitrary native faults.
