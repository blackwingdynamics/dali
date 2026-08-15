# dali inspect

## Purpose

Validate an existing AMRN package and display its decoded contract fields
without modifying the input file.

## Input

- --input <package> — AMRN package to validate.

## Validation

The command validates:

- fixed header presence and magic;
- format version and target identifier;
- fixed header size;
- ABI version and reserved fields;
- payload size and load address;
- word-aligned execution offset;
- exact file length with no trailing bytes;
- payload CRC32.

## Output

On success, output includes the format version, target ID, header and payload
sizes, load and entry addresses, ABI version, and CRC32.

## Example

~~~text
dali inspect --input <application.amrn>
~~~

## Failure cases

The command returns failure for unreadable input, malformed AMRN data,
unsupported compatibility fields, invalid bounds, CRC mismatches, and trailing
bytes. A successful inspection does not prove hardware execution.
