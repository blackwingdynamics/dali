# Dali Application Relocation Contract

This document defines the design boundary for movable ABI v3 applications. It
does not change the current ABI v3 package format or make existing packages
relocatable.

## Current state

The current ABI v3 contract is fixed-address: code is linked for the target
manifest's code origin and data is linked for its data origin. The loader copies
those segments to the declared addresses and rejects packages whose addresses
do not match the target contract. Existing packages remain valid under this
contract.

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

## Package contract for a future format revision

The current AMRN v1 and v2 formats remain unchanged. A future format revision
will add a relocation-table descriptor containing:

- relocation table offset and byte length;
- relocation entry size and table version;
- the linked code base and linked data base;
- the preferred image base or segment-relative bases;
- the number of relocation entries;
- the relocation table checksum, covered by the package CRC.

The table is part of the authenticated-by-integrity package payload. The
loader must reject truncated tables, arithmetic overflow, unknown table
versions, unknown relocation kinds, entries outside declared segments, and
patches outside the declared patch width or alignment.

The final field layout, integer encoding, and header size belong in a new AMRN
format specification before implementation. No existing header field will be
repurposed silently.

## Relocation safety rules

The first implementation will support only relocation kinds emitted by the
pinned ARM toolchain and demonstrated by fixture artifacts. Every supported
kind will have a named decoder, an explicit patch width, alignment rule, signed
range rule, and overflow check. All other kinds are rejected.

Relocation targets must resolve only into the application's declared code,
initialized-data, zero-data, or stack reservations, unless a future ABI
explicitly defines a kernel service or shared-memory relocation. A relocation
must never manufacture a pointer to kernel RAM, peripheral space, an interrupt
vector, or an arbitrary external address.

The loader will calculate each final address from the selected slot metadata;
it will not infer a slot from a package filename or use a fixed application
address as a fallback. The selected code and data regions must satisfy the
target manifest's alignment, capacity, MPU, and non-overlap constraints.

## Build and load pipeline

1. The target manifest supplies the segment alignment, capacities, permitted
   slot ranges, and ABI/format compatibility.
2. The CLI builds the application with the pinned target toolchain and a
   relocation-aware linker configuration.
3. The packaging step extracts code/data segments and the retained relocation
   records, converts only the supported records, and writes the future AMRN
   revision.
4. Host validation checks every relocation's bounds, kind, alignment, range,
   and resulting address before the package is copied to storage.
5. The kernel validates the package, selects a free slot, applies relocations
   within the validated application regions, clears zero data, configures MPU
   regions, and enters the relocated entry point.
6. The kernel rejects the package without copying or jumping if any relocation
   or resulting address is invalid.

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

## Initial linker evidence

The standalone `apps/dali-app-relocation-fixture` builds with the pinned
`thumbv7em-none-eabihf` toolchain and linker `--emit-relocs` option. Its final
ELF contains both `.rel.dali_code` and `.rel.dali_data` sections. The first
fixture produced these application relocation kinds:

- `R_ARM_THM_CALL`;
- `R_ARM_THM_MOVW_ABS_NC`;
- `R_ARM_THM_MOVT_ABS`;
- `R_ARM_ABS32`.

This proves that relocation records can be retained and that both code and
writable-data references are observable. It does not yet approve any kind for
the package format: each kind still needs a bounded decoder, patch test, and
rejection test before it can be accepted by the host or kernel.

Build and inspect the fixture from its directory:

```text
cargo build --offline --features embedded-payload,abi-v3 \
  --target thumbv7em-none-eabihf
readelf -rW target/thumbv7em-none-eabihf/debug/dali-app-relocation-fixture
```

## Non-goals

This design does not add dynamic linking, shared libraries, arbitrary function
pointers into the kernel, application-owned interrupts, signatures, secure
boot, or multiple applications. Those require separate contracts and roadmap
tasks.
