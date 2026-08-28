# 3. AMRN format version

The `.amrn` header contains its own `format_version`. The loader must reject a format version it does not understand.

An AMRN format version must change when any of the following changes:

- header field meaning or byte order;
- header size or field offsets;
- payload interpretation;
- checksum interpretation;
- load or entry-point rules;
- required validation semantics.

The current MVP format is:

```text
AMRN format version: 1
Header size: 32 bytes
Integrity: CRC32
Load address: 0x20008000
Target ID: 0x02 (STM32F405RGT6)
ABI version: 2
```

The current AMRN v1 target profile is STM32F405RGT6 (`0x02`). The STM32F411
BlackPill manifest is generator-only and does not authorize native application
execution.

Format version `1` must remain readable by every kernel that claims support for it.

## 4. Application ABI version

The application ABI is independent from the AMRN container format. A package may have a valid header and checksum while still being incompatible with a kernel ABI.

The MVP ABI is:

```text
ABI version: 2
Entry: unsafe extern "C" fn(*const ServiceTable) -> !
Target: thumbv7em-none-eabihf
Interrupt ownership: kernel-controlled
```

The ABI version must change when entry semantics, calling convention, memory ownership, interrupt ownership, lifecycle behavior, service-table layout, or shared data structures change.

The MPU and unprivileged execution boundary change entry semantics, service
dispatch, memory ownership, and fault handling. It therefore requires a new
ABI version and an AMRN compatibility decision; it must not be shipped as an
ABI v2-compatible implementation.

The package format carries the ABI version at header offset `0x18`.

The feature-gated ABI v3 uses an SVC service gateway, unprivileged Thread mode,
a PSP-backed application stack, and separate application code/data regions. It
is not compatible with ABI v2's direct service-table function pointer or its
single-region linker contract. ABI v3 packages must therefore be rejected by
ABI v2 kernels. The F405 implementation and hardware evidence are scoped to
the experimental feature-gated path; ABI v2 remains the default.

The ABI v3 package contract uses AMRN format version `2`. Format version `2`
has a fixed 64-byte header, separate code and initialized-data file segments,
explicit zero-data and PSP stack reservations, target-defined code/data
origins, and a CRC32 over both file segments. The current F405 linker contract
uses 32 KiB code and 32 KiB data regions. This contract is documented in
`docs/amrn-format/README.md`; it does not authorize the current v1 parser or builder
to accept or emit ABI v3 packages.
