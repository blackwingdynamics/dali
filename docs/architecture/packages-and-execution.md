# 8. `.amrn` package format

The first format revision uses a fixed 32-byte binary header followed by the native payload. The payload is linked for the fixed SRAM address `0x20008000`; relocation and dynamic linking are not part of the MVP.

The header specification must define every byte, field width, byte order, and alignment. The proposed logical fields are:

| Offset | Size | Field | Purpose |
| --- | ---: | --- | --- |
| `0x00` | 4 | Magic | ASCII `DALI` |
| `0x04` | 1 | Format version | MVP value is `1` |
| `0x05` | 1 | Target ID | Current MVP value is `0x02` (`STM32F405RGT6`) |
| `0x06` | 2 | Header size | Little-endian; MVP value is `32` |
| `0x08` | 4 | Payload size | Little-endian payload length |
| `0x0C` | 4 | Load address | Little-endian; MVP value is `0x20008000` |
| `0x10` | 4 | Execution offset | Little-endian offset from payload start |
| `0x14` | 4 | CRC32 | Little-endian payload checksum |
| `0x18` | 1 | ABI version | Current MVP value is `2` |
| `0x19` | 1 | Flags | Must be zero in the MVP |
| `0x1A` | 2 | Reserved | Must be zero in the MVP |
| `0x1C` | 4 | Reserved | Must be zero in the MVP |

The loader must reject:

- an invalid magic value or unsupported version;
- a target ID other than `0x02`;
- an ABI version other than `2`;
- a header size other than 32 bytes;
- non-zero flags or reserved fields;
- integer overflow while calculating offsets or sizes;
- an empty payload or a payload larger than 64 KiB;
- a payload outside the package or reserved executable SRAM region;
- a load address other than `0x20008000`;
- an unaligned or out-of-range execution offset;
- a CRC32 mismatch;
- an entry point outside the payload.

AMRN v5 digital signatures, manifests, version compatibility, revocation, and
anti-rollback exist in the feature-gated post-MVP repository path; they are not
part of the baseline ABI v2 package contract. F405 hardware evidence covers
signed package verification and anti-rollback, while target-side revoked-key
acceptance, production root-key custody enforcement, and pre-reset
kernel-image Secure Boot remain future work.

## 9. RAM loading and execution

The kernel reserves an explicit SRAM region for the MVP application:

```text
0x20000000 - 0x20007FFF   Kernel reserved RAM
0x20008000 - 0x20017FFF   Application region, 64 KiB
0x20018000 - 0x2001FFFF   Kernel runtime and stack RAM
```

The linker script must reserve this region from kernel code, data, stack, and future allocator use.

The loader must:

1. read and validate the header;
2. validate the complete package size before copying;
3. validate the load address and entry offset;
4. copy the payload into the reserved SRAM region;
5. derive the entry address from the load address and execution offset;
6. ensure the Cortex-M Thumb bit is set;
7. disable application-owned interrupts for the MVP;
8. transfer control through the documented `unsafe` entry ABI.

The application entry ABI, return behavior, panic behavior, and reset behavior are specified in `ABI.md`. Interrupt ownership is intentionally deferred.

The current STM32F405 board is the only AMRN execution target. The F411
profile is metadata-only and is not silently treated as compatible with the
F405 target.
