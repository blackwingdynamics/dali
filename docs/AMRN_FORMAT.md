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
| `0x05` | target_id | 1 byte | MVP value identifies STM32F411 |
| `0x06` | header_size | 2 bytes | Little-endian; MVP value is `32` |
| `0x08` | payload_size | 4 bytes | Little-endian payload length |
| `0x0C` | load_address | 4 bytes | Little-endian; MVP value is `0x20008000` |
| `0x10` | execution_offset | 4 bytes | Little-endian offset from payload start |
| `0x14` | crc32 | 4 bytes | Little-endian CRC32 of the payload |
| `0x18` | flags | 2 bytes | Reserved; must be zero in the MVP |
| `0x1A` | reserved | 2 bytes | Must be zero in the MVP |
| `0x1C` | reserved | 4 bytes | Must be zero in the MVP |

The exact header size is 32 bytes. The parser must read fields explicitly from bytes and must not infer the file format from Rust struct layout.

## Validation rules

The loader rejects a package when:

- the file is shorter than `header_size`;
- `magic` is not `DALI`;
- `format_version` is unsupported;
- `target` is unsupported;
- `header_size` is smaller than the required revision size;
- `payload_size` exceeds the file remainder;
- `load_address` is outside the reserved application SRAM region;
- `execution_offset` is outside the payload;
- the calculated entry address overflows;
- the CRC32 does not match the payload.

All arithmetic must be checked before the SD data is copied to SRAM.

## Post-MVP extensions

Future revisions may add manifest data, kernel compatibility, required services, memory declarations, signatures, encryption metadata, and rollback information. These require a new format revision or an explicitly versioned extension area.

## Payload rules

- The payload is linked for `0x20008000`.
- The maximum payload size is 64 KiB.
- The payload contains native `thumbv7em-none-eabihf` code.
- Relocations and dynamic linking are not supported.
- The execution entry is `load_address + execution_offset`.
- The entry address must have the Cortex-M Thumb bit set before the jump.
