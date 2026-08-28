# dali cartridge

## Purpose

Create a contract-valid AMRN cartridge from an already linked native payload.

## Inputs

- --input <payload> — linked native payload binary;
- --output <cartridge> — destination AMRN file;
- --entry-offset <bytes> — word-aligned entry offset from the payload start.

The input is not Rust source and is not an ELF file. It must already be linked
for the documented target and application load address.

## Behavior

The command:

1. reads the payload;
2. validates payload and entry metadata through dali-amrn;
3. writes the documented AMRN header;
4. calculates the payload CRC32;
5. writes the payload after the fixed header.

It does not flash hardware, modify an SD card, or execute the payload.

## Example

~~~text
dali cartridge \
  --input <payload.bin> \
  --output <application.amrn> \
  --entry-offset <byte-offset>
~~~

## Failure cases

The command fails for unreadable input, empty or oversized payloads, invalid
entry offsets, cartridge-size overflow, and unwritable output paths.
