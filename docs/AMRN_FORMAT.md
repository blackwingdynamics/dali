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

ABI v3 packages use AMRN format version `2`. This is a contract definition.
The default kernel loader still uses format version `1`, while the
feature-gated ABI v3 path validates, copies, and launches format version `2`
segments through the MPU/PSP transition. Format version `3` adds explicit
relocation metadata and format version `4` adds identity and selection
metadata; neither changes the ABI v3 calling convention.

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

The v3 launch frame, PSP bounds, SVC gateway, and documented fault cases have
host and F405 evidence for the feature-gated ABI v3 path. This evidence does
not establish complete sandboxing, DMA isolation, or multi-application
execution. A v1 package must continue to follow the single-image ABI v2 rules
above.

## Relocatable package contract (format version 3, feature-gated loader)

Format version `3` is reserved for movable ABI v3 applications. The default
kernel path does not accept it. The feature-gated kernel loader
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
with the kernel's explicit `abi-relocation` feature. The SD card contained
the standalone relocation fixture package. The USB CDC console reported:

```text
[INFO][LOADER] AMRN header and payload validated
[INFO][APP] Relocation fixture
```

This verifies the feature-gated format 3 stream, package validation, and
application entry path on the reference hardware. The fixture later executed
with a non-zero relocation delta in the manifest-declared second slot. Runtime
slot reservation exists for the single loaded application; concurrent
allocation, release after termination, and context switching remain future
work.

## Post-MVP extensions

## AMRN format version 4: package identity and selection metadata

Format version `4` extends the relocatable ABI v3 package contract for
multi-package storage. It does not introduce an ABI v4: the package continues
to declare ABI version `3`, and the code/data/relocation semantics remain those
of format version `3`.

The v4 header is fixed at 128 bytes. The first 80 bytes retain the v3 segment,
relocation, target, and ABI fields at the same offsets. The additional fields
are:

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x50` | package_id | 16 bytes | Opaque stable application identity; not derived from a filename |
| `0x60` | package_version_major | 2 bytes | Little-endian semantic package version component |
| `0x62` | package_version_minor | 2 bytes | Little-endian semantic package version component |
| `0x64` | package_version_patch | 2 bytes | Little-endian semantic package version component |
| `0x66` | minimum_kernel_major | 2 bytes | Minimum compatible kernel API version component |
| `0x68` | minimum_kernel_minor | 2 bytes | Minimum compatible kernel API version component |
| `0x6A` | minimum_kernel_patch | 2 bytes | Minimum compatible kernel API version component |
| `0x6C` | required_services | 4 bytes | Manifest-defined service capability bitset |
| `0x70` | slot_id | 1 byte | Explicit target-manifest slot identifier |
| `0x71` | flags | 1 byte | Must be zero until a flag is specified by this document |
| `0x72` | reserved | 2 bytes | Must be zero |
| `0x74` | crc32 | 4 bytes | CRC32 of header bytes after this field, payload, and relocation table |
| `0x78` | reserved | 8 bytes | Must be zero |

The v4 header retains the v3 fields through offset `0x4F`, including the
relocation metadata and ABI version. The v4 `crc32` field is moved to the
extension area so the integrity check covers the complete selection metadata;
the old v3 payload-only checksum rule is unchanged for format version `3`.
The exact CRC input order is the bytes from `0x78` through the end of the
package followed by the bytes from `0x00` through `0x73`, excluding the v4
`crc32` field itself. A builder and loader must use this order exactly.

Selection rules are bounded and explicit:

- `package_id` must be non-zero and must be unique among installed packages;
- package version and minimum kernel version use three unsigned, little-endian
  semantic components and must not overflow their field widths;
- `required_services` must be a subset of services declared by the kernel;
- `slot_id` must name a slot declared by the target manifest;
- target ID, ABI version, relocation contract, and memory ranges must still
  pass the format v3 validation rules;
- a missing identity, incompatible package, duplicate identity, or occupied
  slot is a rejection, never an implicit filename- or directory-order choice;
- format v3 packages remain valid under their existing single-package rules.

The v4 codec, CLI builder/inspection, feature-gated kernel selection/streaming
loader, and host streaming-loader fixtures are implemented without changing the
ABI v2/v3 package paths. A manifest-backed v4 relocation fixture is available
for hardware testing, and the F405 hardware path has accepted that fixture for
selection, relocation, and successful execution. v4-specific rejection and
recovery hardware tests remain open. Format v4 must not be advertised as
multi-application support yet.

Future revisions may add manifest data, kernel compatibility, required services,
memory declarations, encryption metadata, and rollback information. These
require a new format revision or an explicitly versioned extension area.

### Signature envelope contract

The hardware-neutral `dali-amrn::signature` module defines a bounded `DSIG`
envelope for a future versioned AMRN signature extension. The fixed envelope is
88 bytes:

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x00` | magic | 4 bytes | ASCII `DSIG` |
| `0x04` | envelope_version | 1 byte | Value is `1` |
| `0x05` | algorithm | 1 byte | Value `1` means Ed25519 |
| `0x06` | key_id_length | 1 byte | Value is `16` |
| `0x07` | signature_length | 1 byte | Value is `64` |
| `0x08` | key_id | 16 bytes | Opaque trust-anchor identifier |
| `0x18` | signature | 64 bytes | Algorithm-specific signature bytes |

The current parser validates only structure and bounded lengths; it does not
make unsigned v4 packages secure. The v5 loader adds feature-gated signature
verification and static target trust-anchor selection. The complete
multi-developer trust chain, metadata roles, and update policy are defined by
`docs/PACKAGE_DISTRIBUTION.md` and are not implemented by this envelope alone.
The `SignatureVerifier` trait is the hardware-neutral boundary for an audited
host signer and a target trust-store verifier; it does not provide a default
or bypass implementation. The `dali-crypto` facade now provides a bounded
incremental verifier backed by `ed25519-dalek 3.0.0`; its standard-Ed25519
chunked verification is host-tested and thumb-target checked. This backend
selection does not, by itself, enable kernel-side verification or Secure Boot.

### Signed package container (format version 5, feature-gated kernel path)

Format version `5` is the signed successor to the identity-aware relocatable
contract. It does not modify the v4 byte layout. A v5 package uses the v3/v4
image and identity fields in the first 128 bytes, extends the fixed header to
160 bytes, stores the image payload after that header, and appends one fixed
88-byte `DSIG` envelope:

```text
┌──────────────────────────────┐
│ AMRN v5 fixed header (160 B)  │
├──────────────────────────────┤
│ code + data + relocation data │ signed bytes
├──────────────────────────────┤
│ DSIG envelope (88 B)          │ not signed
└──────────────────────────────┘
```

The first 128 bytes retain the v4 field offsets. Their format-version field is
`5`, their header-size field is `160`, and the relocation offset is adjusted
from the v3 linked image to the v5 payload origin. The extension fields are:

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x80` | signature_offset | 4 bytes | Little-endian; exact start of the DSIG trailer |
| `0x84` | signature_size | 2 bytes | Little-endian; value is `88` |
| `0x86` | reserved | 2 bytes | Must be zero |
| `0x88` | signed_size | 4 bytes | Little-endian; equal to `signature_offset` |
| `0x8C` | reserved | 4 bytes | Must be zero |
| `0x90` | reserved | 16 bytes | Must be zero |

The signed byte range is exactly `package[0..signature_offset]`. It includes
the final header and package CRC32, but excludes the DSIG trailer. The package
CRC32 is calculated over the signed range with its own four bytes treated as
zero, so CRC construction is not circular with signature construction. The
signature envelope is structurally validated by `dali-amrn`; cryptographic
verification and trust-anchor lookup remain separate responsibilities.

The host codec currently encodes and parses this contract with boundary tests;
its v5 API can also parse the fixed header and DSIG trailer separately for a
bounded streaming loader.
The CLI can now produce format 5 when the application manifest declares
`signing_key_id` and the private seed is supplied through the external
`DALI_SIGNING_KEY_HEX` environment variable. The feature-gated kernel loader
now has a bounded v5 path that authenticates the signed range before copying
or relocating the image, using static trust anchors from the selected target
profile. F405 hardware has verified the configured release-anchor path. This
is not Secure Boot and is not the completed multi-developer distribution
contract.

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
