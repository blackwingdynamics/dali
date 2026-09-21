# Format 3 cartridge contract

The current AMRN v1 and v2 formats remain unchanged. Format 3 adds a
relocation-table descriptor containing:

- relocation table offset and byte length;
- relocation entry size and table version;
- the linked code base and linked data base;
- the preferred image base or segment-relative bases;
- the number of relocation entries;
- the relocation table checksum, covered by the cartridge CRC.

The table is part of the authenticated-by-integrity cartridge payload. The
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

The loader calculates each final address from the manifest slot whose declared
code and data load addresses match the cartridge header. It does not infer a
slot from a cartridge filename or use a fixed application address as a fallback.
The selected code and data regions must satisfy the target manifest's
alignment, capacity, MPU, and non-overlap constraints.
