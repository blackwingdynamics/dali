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
| `0x18` | abi_version | 1 byte | MVP value is `1` |
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

## Post-MVP extensions

Future revisions may add manifest data, kernel compatibility, required services, memory declarations, signatures, encryption metadata, and rollback information. These require a new format revision or an explicitly versioned extension area.

## Payload rules

- The payload is linked for `0x20008000`.
- The maximum payload size is 64 KiB.
- The payload contains native `thumbv7em-none-eabihf` code.
- AMRN v1 execution is defined for the STM32F405RGT6 current MVP target.
  STM32F411 support remains a separate board profile and is not implied by the
  F405 target ID.
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
