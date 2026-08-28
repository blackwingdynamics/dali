# Payload rules

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
