# Relocatable cartridge contract (format version 3, feature-gated loader)

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
| `0x2C` | relocation_offset | 4 bytes | Absolute cartridge offset of the table |
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
the standalone relocation fixture cartridge. The USB CDC console reported:

```text
[INFO][LOADER] AMRN header and payload validated
[INFO][APP] Relocation fixture
```

This verifies the feature-gated format 3 stream, cartridge validation, and
application entry path on the reference hardware. The fixture later executed
with a non-zero relocation delta in the manifest-declared second slot. Runtime
slot reservation exists for the single loaded application. The feature-gated
ABI v3 scheduler and F405 backend separately provide declared context
switching; concurrent allocation, release after termination, and production
lifecycle policy remain future work.

## Post-MVP extensions
