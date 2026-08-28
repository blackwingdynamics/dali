This document defines the design boundary for movable ABI v3 applications.
Format 3 is host-packagable and feature-gated in the kernel; it is not the
default production boot mode.

## Current state

AMRN format 2 remains fixed-address: code is linked for the target manifest's
active slot and data is linked for its data origin. Format 3 adds explicit
relocation metadata and a manifest-owned destination slot. Both formats remain
feature-gated in the kernel, and existing format 2 packages remain valid.

Compiler options such as `-C relocation-model=pic` or `-C relocation-model=rwpi`
are not, by themselves, a Dali relocation contract. A final linked ELF may
contain no relocation sections, and writable data, entry addresses, linker
symbols, and architecture-specific references still need an explicitly
defined treatment.

## Selected direction

Dali will use explicit relocation metadata for the first movable-application
implementation. The application build pipeline will produce a linked image,
retain the linker's relocation records, and convert only the supported records
into a bounded AMRN relocation table. The kernel loader will apply that table
after validating the package and before enabling the application's MPU
permissions or entering the application.

This direction keeps placement policy in the target manifest and loader,
makes unsupported references rejectable, and avoids depending on an implicit
ARM RWPI register convention in every application and SDK library. PIC/RWPI
may be evaluated later as an alternative ABI, but it is not part of this
contract.
