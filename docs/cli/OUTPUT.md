# Dali CLI Output

## Streams

- Successful command results are written to standard output.
- Command failures are written by the executable entry point to standard
  error.
- The CLI does not mix diagnostic messages into a successful inspection
  report.

## Human-readable output

The current output is intended for developers and operators. Dali inspect
reports:

- AMRN format version;
- target identifier;
- header size;
- payload size;
- load address;
- execution offset;
- calculated entry address;
- ABI version;
- stored CRC32.

Hexadecimal fields are prefixed with 0x. Decimal fields remain decimal.

## Stability

The output is not yet a machine-readable compatibility format. Scripts should
use exit status for success/failure and should not depend on presentation
spacing or field order until a structured output contract is introduced.
