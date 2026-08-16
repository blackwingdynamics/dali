# dali inspect

## Purpose

Validate an existing AMRN package and display its decoded contract fields
without modifying the input file.

This is a generic package operation, not an application-project command. It
does not build or package an application. Use `dali app build` for the full
application workflow, then use this command to validate the resulting AMRN
artifact.

## Input

- `--input <package>` — AMRN package to validate explicitly.

The input flag is optional. Without it, the command first checks the current
directory for `dali.toml` and derives the package path from the application
name, target profile, and Cargo profile. If no manifest exists, it accepts the
single `.amrn` file in the current directory. Multiple packages require an
explicit input path.

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

For AMRN format version 2, the command resolves the target identifier through
the repository target registry and validates the declared code/data regions,
zero-initialized data, PSP stack reservation, segment layout, and ABI v3 CRC32.
It reports the v2 segment and launch fields separately. Inspection does not
enable ABI v3 execution or create an ABI v3 package.

## Output

On success, output includes the format version, target ID, header and payload
sizes, load and entry addresses, ABI version, and CRC32. Format version 2 also
includes code size, initialized-data size, zero-data size, and stack size.

## Example

~~~text
dali inspect
~~~

For an explicit package or a package in another directory:

~~~text
dali inspect --input /path/to/application.amrn
~~~

## Failure cases

The command returns failure for unreadable input, malformed AMRN data,
unsupported compatibility fields, invalid bounds, CRC mismatches, and trailing
bytes. A successful inspection does not prove hardware execution.
