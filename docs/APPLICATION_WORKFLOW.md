# Application Workflow

This document describes how a Dali OS application becomes an AMRN package and
how the kernel currently consumes that package.

The current workflow validates package construction and loader input. The
loader now contains the bounded SRAM-copy and ABI entry-transfer path for the
F405 MVP target. The first physical loader and PB2 LED execution result has
been observed; the complete MVP acceptance procedure remains broader than this
single execution-path test.

## Workflow overview

```text
Rust application source
        |
        v
target-native ELF linked for the application region
        |
        | cargo objcopy
        v
raw native payload binary
        |
        | dali package
        v
AMRN package (.amrn)
        |
        | copy to the SD-card root
        v
kernel filesystem scan and bounded AMRN validation
        |
        | validated loader execution path
        v
copy to application SRAM and call the native entry point
```

The `.amrn` file contains native machine code and the AMRN header. It does not
contain Rust source code and it is not a Cargo project.

## Application layout

The initial application is `apps/dali-app-hello`. It has two purposes:

- `src/lib.rs` remains the host-testable application scaffold;
- `src/main.rs` is the `no_std` native payload used by the embedded build.

The current payload configures the F405 board's active-high PB2 status LED,
submits three messages through the ABI v2 kernel logging service, and produces
three short logical flashes followed by a longer pause. The pattern uses named,
bounded busy-loop constants; the relative phase lengths are the contract, while
the exact wall-clock duration depends on the selected embedded build and clock.
The direct GPIO proof is board-specific application code; the logging call
crosses the documented service boundary.

The embedded payload is enabled explicitly with the `embedded-payload` Cargo
feature. This prevents the native entry binary from being built as part of
ordinary host workspace tests.

The application linker script reserves the documented application region and
places the native entry section first:

- load address: `0x20008000`;
- application region: 64 KiB;
- entry ABI: `unsafe extern "C" fn(*const ServiceTable) -> !`.

These values belong to the ABI and memory-layout contracts. An application must
not choose a different load address or entry convention independently.

## Build a native payload

From the repository root, build the validation application for the embedded
target:

```text
cargo build -p dali-app-hello \
  --features embedded-payload \
  --target thumbv7em-none-eabihf
```

Convert the linked ELF into a raw binary payload:

```text
cargo objcopy -p dali-app-hello \
  --features embedded-payload \
  --target thumbv7em-none-eabihf \
  --bin dali-app-hello \
  -- -O binary target/thumbv7em-none-eabihf/debug/dali-app-hello.bin
```

The binary is the linked native image, not source code. The entry offset is
currently zero because the linker places the declared entry section at the
beginning of the payload.

The equivalent repository recipe is:

```text
just app-build
```

## Create an AMRN package

The package command adds the documented AMRN v1 header, records the payload
length and entry metadata, and calculates the CRC32 over the payload:

```text
cargo run -p dali-cli --bin dali -- package \
  --input target/thumbv7em-none-eabihf/debug/dali-app-hello.bin \
  --output target/thumbv7em-none-eabihf/debug/hello.amrn \
  --entry-offset 0
```

The command rejects an empty payload, an oversized payload, an output that is
too small, or an invalid entry offset. It does not infer an entry symbol from
the ELF file; the caller supplies the byte offset explicitly.

The equivalent recipe, including the application build and binary extraction,
is:

```text
just package-hello
```

The generated package is written under the target directory and is ignored by
Git as a build artifact.

## Inspect an AMRN package

Use the host CLI to validate an existing package against the AMRN contract and
print its decoded fields:

```text
cargo run -p dali-cli --bin dali -- inspect \
  --input target/thumbv7em-none-eabihf/debug/hello.amrn
```

The command validates the header, target, ABI version, payload bounds,
execution entry, CRC32, and exact file length. It does not modify the package.

## Install the CLI

Install the host CLI from the repository with Cargo:

```text
cargo install --path crates/dali-cli --locked
```

After installation, the user-facing executable is `dali`:

```text
dali inspect --input target/thumbv7em-none-eabihf/debug/hello.amrn
```

The Cargo package remains `dali-cli`; `dali` is the installed executable name.

## Install a package on the SD card

1. Build the package with `just package-hello`.
2. Insert the SD card into the host card reader.
3. Identify the card device with a size-aware command such as:

   ```text
   lsblk -o NAME,PATH,SIZE,FSTYPE,LABEL,MOUNTPOINTS,TRAN
   ```

4. Mount the card's existing FAT partition using the device path reported by
   the host. Do not guess the device name.
5. Copy the generated `.amrn` file into the filesystem root:

   ```text
   cp target/thumbv7em-none-eabihf/debug/hello.amrn <mounted-sd-root>/hello.amrn
   sync
   ```

6. Unmount the card cleanly before removing it:

   ```text
   umount <mounted-sd-root>
   ```

The kernel scans the root directory for `.amrn` files and does not require the
basename `hello`. The current MVP policy requires exactly one root AMRN file;
zero files and multiple files are reported as package-selection failures.

## Flash and observe loader validation

Flash the kernel for the selected board using the board-specific repository
recipe. For the STM32F405 board, for example:

```text
just flash-probe f405
```

Open the USB CDC console after the runtime device appears:

```text
just console
```

Successful package discovery and validation currently produce log records
equivalent to:

```text
[STORAGE] Read block 0 successfully
[LOADER] AMRN header and payload validated
```

This proves that the board read the card, found one AMRN file, validated its
header and exact length, and verified its payload CRC32. It does not yet prove
that the payload was copied to SRAM or executed.

## Current boundary and next loader stage

The current loader is intentionally read-only and bounded:

1. select exactly one root AMRN file;
2. read and validate the fixed header;
3. verify the declared package length;
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
- `No AMRN package found; entering kernel heartbeat` means the card is valid
  but no application package is present in its root; the kernel remains in its
  idle heartbeat state and does not treat this as a boot failure.
- `No storage medium detected; entering kernel heartbeat` means no configured
  storage medium is available; the kernel remains in its idle heartbeat state
  and does not expose a transport timeout as an application failure.
- `Root scan found multiple AMRN file(s)` means the current exact-one-package
  MVP policy was violated.
- `AMRN validation failed` means the package header, bounds, entry metadata,
  length, or CRC32 is invalid.

Do not rename a source file to `.amrn`. The package must be produced from a
linked native payload with `dali package`.
