# Current boundary and next loader stage

The current loader is intentionally read-only and bounded:

1. select exactly one root AMRN file;
2. read and validate the fixed header;
3. verify the declared cartridge length;
4. stream the payload in bounded chunks;
5. verify the CRC32;
6. report success or a typed failure.

The loader's execution path performs a second bounded read pass after the CRC
pass, copies only the validated payload into the reserved SRAM region, and
transfers control through the validated ABI entry address. It preserves the
existing bounds checks and keeps the unsafe operations centralized in the
loader. Physical F405 testing has now shown the LED acceptance behavior once;
repeatable reset and complete MVP acceptance evidence remain separate gates
before the application is considered accepted.

## Troubleshooting boundaries

- `FormatError` or `DeviceError(Unsupported)` means the card layout or
  filesystem is outside the currently supported FAT contract.
- `No AMRN cartridge found; entering kernel heartbeat` means the card is valid
  but no application cartridge is present in its root; the kernel remains in its
  idle heartbeat state and does not treat this as a boot failure.
- `No storage medium detected; entering kernel heartbeat` means no configured
  storage medium is available; the kernel remains in its idle heartbeat state
  and does not expose a transport timeout as an application failure.
- `Root scan found multiple AMRN file(s)` means the current exact-one-cartridge
  MVP policy was violated.
- `AMRN validation failed` means the cartridge header, bounds, entry metadata,
  length, or CRC32 is invalid.

Do not rename a source file to `.amrn`. The cartridge must be produced from a
linked native payload with `dali cartridge`.
