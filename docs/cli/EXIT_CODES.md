# Dali CLI Exit Codes

## Current contract

| Code | Meaning |
| ---: | --- |
| 0 | The command completed successfully. |
| 1 | The command failed because of invalid arguments, input, package data, or host I/O. |

The current CLI uses a single non-zero failure code. More granular codes
require a documented compatibility decision before implementation.

## Scripting

Automation should treat zero as success and any non-zero result as failure.
Do not infer success from the presence of partial output.
