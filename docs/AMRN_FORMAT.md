# AMRN Package Format

`.amrn` means **Amiran Native package**. It is the Dali OS application package extension.

## MVP format

The MVP package is a fixed 32-byte binary header followed by one native ARM payload:

```text
┌──────────────────────┐
│ Fixed header         │
├──────────────────────┤
│ Native code payload  │
└──────────────────────┘
```

The first revision must use an explicitly documented byte layout. Rust struct layout must not be used as an implicit file format.

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x00` | magic | 4 bytes | ASCII `DALI` |
| `0x04` | format_version | 1 byte | MVP value is `1` |
| `0x05` | target_id | 1 byte | Current MVP profile is `0x02` (`STM32F405RGT6`) |
| `0x06` | header_size | 2 bytes | Little-endian; MVP value is `32` |
| `0x08` | payload_size | 4 bytes | Little-endian payload length |
| `0x0C` | load_address | 4 bytes | Little-endian; MVP value is `0x20008000` |
| `0x10` | execution_offset | 4 bytes | Little-endian offset from payload start |
| `0x14` | crc32 | 4 bytes | Little-endian CRC32 of the payload |
| `0x18` | abi_version | 1 byte | Current MVP value is `2` |
| `0x19` | flags | 1 byte | Reserved; must be zero in the MVP |
| `0x1A` | reserved | 2 bytes | Must be zero in the MVP |
| `0x1C` | reserved | 4 bytes | Must be zero in the MVP |

The exact header size is 32 bytes. The parser must read fields explicitly from bytes and must not infer the file format from Rust struct layout.

## Validation rules

The loader rejects a package when:

- the file is shorter than 32 bytes;
- `magic` is not `DALI`;
- `format_version` is unsupported;
- `target_id` is not `0x02`;
- `header_size` is not exactly 32;
- `abi_version` is unsupported;
- `flags` or any reserved field is non-zero;
- `payload_size` exceeds the file remainder;
- `payload_size` is zero or exceeds 64 KiB;
- `load_address` is not exactly `0x20008000`;
- `execution_offset` is not within the payload or is not word-aligned;
- the calculated entry address overflows;
- the CRC32 does not match the payload.

All arithmetic must be checked before the SD data is copied to SRAM.

## ABI v3 package contract (format version 2)

ABI v3 packages use AMRN format version `2`. This is a contract definition;
the default kernel loader still uses format version `1`, while the CLI emits
and inspects format version `2` packages. The feature-gated kernel path now
validates and copies format version `2` segments, but must not enter them until
the v3 privilege transition is implemented.

The v2 header is a fixed 64-byte header followed by two file segments in a
single payload:

```text
┌──────────────────────────┐
│ Fixed v2 header (64 bytes)
├──────────────────────────┤
│ Application code          │ code_size bytes
├──────────────────────────┤
│ Initialized application   │ data_init_size bytes
└──────────────────────────┘
```

The header is decoded explicitly. Rust struct layout is not part of the file
format.

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x00` | magic | 4 bytes | ASCII `DALI` |
| `0x04` | format_version | 1 byte | Value is `2` |
| `0x05` | target_id | 1 byte | Target manifest identifier |
| `0x06` | header_size | 2 bytes | Little-endian; value is `64` |
| `0x08` | code_size | 4 bytes | File code segment length |
| `0x0C` | data_init_size | 4 bytes | File initialized-data segment length |
| `0x10` | data_zero_size | 4 bytes | Runtime zero-initialized data length |
| `0x14` | stack_size | 4 bytes | Runtime PSP stack reservation |
| `0x18` | code_load_address | 4 bytes | Target-defined application code origin |
| `0x1C` | data_load_address | 4 bytes | Target-defined application data origin |
| `0x20` | execution_offset | 4 bytes | Word-aligned offset from code origin |
| `0x24` | crc32 | 4 bytes | CRC32 of code followed by initialized data |
| `0x28` | abi_version | 1 byte | Value is `3` |
| `0x29` | flags | 1 byte | Reserved; must be zero |
| `0x2A` | reserved | 2 bytes | Must be zero |
| `0x2C` | reserved | 4 bytes | Must be zero |
| `0x30` | reserved | 16 bytes | Must be zero |

The v2 linker contract for the current F405 isolation target is:

```text
code origin: 0x20008000, capacity 32 KiB
data origin: 0x20010000, capacity 32 KiB
```

These addresses and capacities are target-manifest properties, not values
chosen by an application. Code and read-only data are placed in the code
region. Initialized writable data is placed at the beginning of the data
region, followed by zero-initialized data and then the PSP stack reservation.
The runtime PSP starts at the top of the declared stack reservation and grows
downward. The loader must validate that the three data-region portions fit
without overlap before copying or launching the application.

The v2 payload is not position-independent. Relocation, dynamic linking, and
automatic slot selection remain unsupported. An application is linked for the
target manifest's declared code and data origins.

### v2 validation and launch rules

The v3 loader must reject a package when:

- the header is shorter than 64 bytes or has another header size;
- the format version, target ID, or ABI version is unsupported;
- a reserved field or unsupported flag is non-zero;
- the code size or stack size is zero, or any checked size addition overflows;
  initialized-data and zero-data sizes may independently be zero;
- the code segment exceeds the target code region;
- the initialized data, zero data, and stack do not fit in the target data
  region;
- the payload does not contain exactly the code and initialized-data segments;
- the execution offset is outside the code segment or is not word-aligned;
- either segment address differs from the target manifest;
- the calculated entry address or PSP bounds are invalid;
- the CRC32 does not match the concatenated file segments.

Before entering Thread mode, the privileged kernel creates a validated launch
frame. The frame selects the application PSP, contains the Thumb entry point,
uses a kernel-defined non-returning link value, and clears unused argument
registers. The exception-return encoding is generated by the kernel and is
never accepted from the package. The application starts unprivileged and can
reach kernel services only through the ABI v3 SVC gateway.

The v3 launch frame, PSP bounds, SVC gateway, and fault policy must be tested
before a kernel may claim ABI v3 support. A v1 package must continue to follow
the single-image ABI v2 rules above.

## Relocatable package contract (format version 3, feature-gated loader)

Format version `3` is reserved for movable ABI v3 applications. The current
the default kernel path does not accept it. The feature-gated kernel loader
validates and applies its bounded relocation table against the target
manifest's current code and data origins. The host-side `dali-amrn::v3` module
defines the byte contract used by both sides.

The fixed 80-byte header is followed by code, initialized data, and a bounded
relocation table:

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x00` | magic | 4 bytes | ASCII `DALI` |
| `0x04` | format_version | 1 byte | Value is `3` |
| `0x05` | target_id | 1 byte | Target manifest identifier |
| `0x06` | header_size | 2 bytes | Little-endian; value is `80` |
| `0x08` | code_size | 4 bytes | File code segment length |
| `0x0C` | data_init_size | 4 bytes | File initialized-data length |
| `0x10` | data_zero_size | 4 bytes | Runtime zero-data length |
| `0x14` | stack_size | 4 bytes | Runtime PSP stack reservation |
| `0x18` | linked_code_base | 4 bytes | Code origin used by the linker |
| `0x1C` | linked_data_base | 4 bytes | Data origin used by the linker |
| `0x20` | code_load_address | 4 bytes | Selected slot code origin |
| `0x24` | data_load_address | 4 bytes | Selected slot data origin |
| `0x28` | execution_offset | 4 bytes | Word-aligned offset from code origin |
| `0x2C` | relocation_offset | 4 bytes | Absolute package offset of the table |
| `0x30` | relocation_count | 4 bytes | Number of 16-byte entries |
| `0x34` | relocation_entry_size | 2 bytes | Value is `16` |
| `0x36` | reserved | 2 bytes | Must be zero |
| `0x38` | crc32 | 4 bytes | CRC32 of code, data, and table |
| `0x3C` | abi_version | 1 byte | Value is `3` |
| `0x3D` | flags | 1 byte | Reserved; must be zero |
| `0x3E` | reserved | 2 bytes | Must be zero |
| `0x40` | reserved | 16 bytes | Must be zero |

Each relocation entry is 16 bytes:

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x00` | segment | 1 byte | `0` code, `1` initialized data |
| `0x01` | kind | 1 byte | Dali relocation kind, not a raw ELF value |
| `0x02` | reserved | 2 bytes | Must be zero |
| `0x04` | patch_offset | 4 bytes | Patch offset within the segment |
| `0x08` | linked_target | 4 bytes | Target address in the linked image |
| `0x0C` | addend | 4 bytes | Signed relocation addend |

The initial kind identifiers are `Abs32`, `ThmCall`, `ThmMovwAbsNc`, and
`ThmMovtAbs`. They are not accepted by the kernel yet. The host and kernel
must reject unknown kinds, invalid segment/patch bounds, alignment errors,
overflow, targets outside linked code/data/zero-data/stack reservations, and
any table that is truncated or not at the canonical payload end.

## Format 3 hardware evidence

On 2026-08-17, an STM32F405RGT6 was flashed through a Pico CMSIS-DAP probe
with the kernel's explicit `abi-v3-relocation` feature. The SD card contained
the standalone relocation fixture package. The USB CDC console reported:

```text
[INFO][LOADER] AMRN header and payload validated
[INFO][APP] Relocation fixture
```

This verifies the feature-gated format 3 stream, package validation, and
application entry path on the reference hardware. The fixture used the
manifest's current code/data origins, so the relocation delta was zero. A
non-zero relocation, slot selection, and execution from a second slot remain
unverified until the slot manager exists.

## Post-MVP extensions

Future revisions may add manifest data, kernel compatibility, required services, memory declarations, signatures, encryption metadata, and rollback information. These require a new format revision or an explicitly versioned extension area.

## Payload rules

- The payload is linked for `0x20008000`.
- The maximum payload size is 64 KiB.
- The payload contains native `thumbv7em-none-eabihf` code.
- AMRN v1 execution is defined for the STM32F405RGT6 current MVP target.
  The STM32F411 manifest is generator-only and is not implied by the F405
  target ID.
- Relocations and dynamic linking are not supported.
- The execution entry is `load_address + execution_offset`.
- The entry address must have the Cortex-M Thumb bit set before the jump.

## Canonical CRC32

The checksum is CRC-32/ISO-HDLC over the payload bytes only. Its parameters are
fixed for format version 1:

```text
width   = 32
poly    = 0x04C11DB7
init    = 0xFFFFFFFF
refin   = true
refout  = true
xorout  = 0xFFFFFFFF
check   = 0xCBF43926 for the ASCII string "123456789"
```

The stored value is encoded as a little-endian `u32`. Header bytes are not
included in the checksum.
