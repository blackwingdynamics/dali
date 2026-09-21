# Application Workflow Overview and Layout

This document describes how a Dali OS application payload becomes an AMRN
cartridge and how the kernel currently consumes that cartridge.

The current workflow validates cartridge construction and loader input. The
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
        | dali cartridge
        v
AMRN cartridge (.amrn)
        |
        | copy to the SD-card root
        v
kernel filesystem scan and bounded AMRN validation
        |
        | validated loader execution path
        v
copy to application SRAM and call the native entry point
```

The `.amrn` cartridge contains native machine code and the AMRN header. It does not
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
