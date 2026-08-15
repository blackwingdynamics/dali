# Application Workflow

This document describes how a Dali OS application becomes an AMRN package and
how the kernel currently consumes that package.

The current workflow validates package construction and loader input. The MVP
loader does not yet copy the payload into application SRAM or transfer control
to its entry point. Those steps are explicitly described as future stages
below.

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
        | dali-cli package
        v
AMRN package (.amrn)
        |
        | copy to the SD-card root
        v
kernel filesystem scan and bounded AMRN validation
        |
        | future loader stage
        v
copy to application SRAM and call the native entry point
```

The `.amrn` file contains native machine code and the AMRN header. It does not
contain Rust source code and it is not a Cargo project.

## Application layout

The initial application is `apps/dali-app-hello`. It has two purposes:

- `src/lib.rs` remains the host-testable application scaffold;
- `src/main.rs` is the `no_std` native payload used by the embedded build.

The embedded payload is enabled explicitly with the `embedded-payload` Cargo
feature. This prevents the native entry binary from being built as part of
ordinary host workspace tests.

The application linker script reserves the documented application region and
places the native entry section first:

- load address: `0x20008000`;
- application region: 64 KiB;
- entry ABI: `unsafe extern "C" fn() -> !`.

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
cargo run -p dali-cli -- package \
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

The next implementation stage must add validated SRAM copying and the ABI
entry transfer. It must preserve the existing bounds checks, keep unsafe code
centralized, and add host tests plus the physical LED acceptance evidence
before the application is considered executable.

## Troubleshooting boundaries

- `FormatError` or `DeviceError(Unsupported)` means the card layout or
  filesystem is outside the currently supported FAT contract.
- `Root scan found 0 AMRN file(s)` means the package is not in the mounted
  filesystem root, the wrong card was installed, or the file was not flushed
  before removal.
- `Root scan found multiple AMRN file(s)` means the current exact-one-package
  MVP policy was violated.
- `AMRN validation failed` means the package header, bounds, entry metadata,
  length, or CRC32 is invalid.

Do not rename a source file to `.amrn`. The package must be produced from a
linked native payload with `dali-cli package`.
