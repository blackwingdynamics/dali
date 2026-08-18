# ABI v3 kernel-memory fault test

`apps/dali-app-fault-kernel` and `apps/dali-app-fault-kernel-write` are
non-production F405 test fixtures. They first use the ABI v3 logging service,
then perform one volatile read or write at the kernel-reserved address
declared by the generated F405 target metadata.

The expected result is a security fault record followed by the kernel-owned
recovery record. The test must not be used as an application template and is
not part of the default workspace build.

## Read test

From the fixture directory:

```text
cd apps/dali-app-fault-kernel
dali app build
```

The package is created at:

```text
target/thumbv7em-none-eabihf/debug/dali-app-fault-kernel.amrn
```

Copy that package to the root of the FAT32 SD card, safely unmount the card,
and insert it into the F405 board.

## Hardware procedure

Build the kernel with `abi-mpu`, connect the Pico as the SWD probe, and
flash the kernel through the probe. Keep RTT output visible:

```text
cargo build -p dali-kernel --no-default-features --features board-stm32f405-sd,usb-cdc,abi-current,abi-mpu --target thumbv7em-none-eabihf
dali device flash f405 --transport probe --input target/thumbv7em-none-eabihf/debug/dali-kernel
```

Expected output includes:

```text
[INFO][APP] Fault injection: kernel memory read
[ERROR][SECURITY] [SECURITY][FAULT] kind=MemManage
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

This proves only processor-side rejection of an unprivileged read into the
declared kernel region and return to the kernel recovery path. It does not
prove peripheral rejection, write rejection, DMA isolation, or complete
application isolation.

## Write test

Build and package the write fixture instead:

```text
cd apps/dali-app-fault-kernel-write
dali app build
```

Copy `target/thumbv7em-none-eabihf/debug/dali-app-fault-kernel-write.amrn` to
the SD-card root, then repeat the same kernel build and flash procedure above.
Expected application output is:

```text
[INFO][APP] Fault injection: kernel memory write
[ERROR][SECURITY] [SECURITY][FAULT] kind=MemManage
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

This proves only processor-side rejection of an unprivileged write into the
declared kernel region. It does not prove peripheral rejection, invalid
execution, PSP bounds, DMA isolation, or complete application isolation.

## Peripheral-access test

Build and package the peripheral fixture:

```text
cd apps/dali-app-fault-peripheral
dali app build
```

Copy `target/thumbv7em-none-eabihf/debug/dali-app-fault-peripheral.amrn` to
the SD-card root, then repeat the same kernel build and flash procedure.
Expected application output is:

```text
[INFO][APP] Fault injection: peripheral memory read
[ERROR][SECURITY] [SECURITY][FAULT] kind=MemManage
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

This proves only processor-side rejection of an unprivileged read from the
manifest-declared peripheral region. It does not prove peripheral writes,
invalid execution, PSP bounds, DMA isolation, or complete application
isolation.

## Cross-slot application-memory test

Build and package the slot1 fixture:

```text
cd apps/dali-app-fault-cross-slot
dali app build
```

Copy only this package to the SD-card root. The package declares `slot1` and
its fixture reads the code origin of the manifest-declared slot0. Build and
flash the MPU-enabled kernel:

```text
cargo build -p dali-kernel --no-default-features --features board-stm32f405-sd,usb-cdc,abi-relocation --target thumbv7em-none-eabihf
dali device flash f405 --transport probe --input target/thumbv7em-none-eabihf/debug/dali-kernel
```

Expected output is:

```text
[INFO][APP] Fault injection: slot1 read of slot0 memory
[ERROR][SECURITY] [SECURITY][FAULT] kind=MemManage
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

This proves that the active slot1 application cannot read the manifest-owned
slot0 code region through the processor MPU. It does not prove DMA isolation,
context-switch MPU reprogramming, faulted-context exclusion, or complete
multi-application isolation.

With `abi-context-switch` enabled and both slot fixtures present, the F405
acceptance run additionally verified that recovery retired slot1 and resumed
slot0. GDB hit the slot1 entry once, stopped in `recover_faulted_context`, and
the slot0 progress marker increased from `0x0000024C` to `0x01289A9C`,
`0x017BB673`, and `0x01ADDDF1` without a second slot1 entry. This is hardware
evidence for faulted-context exclusion and continued execution, not DMA
isolation or a complete multi-application security guarantee.

## Reverse cross-slot application-memory test

Build and package the slot0 reverse-direction fixture:

```text
cd apps/dali-app-fault-cross-slot-reverse
dali app build
```

Use this package together with the existing slot1 fixture on the SD-card root.
The reverse fixture declares `slot0` and reads the manifest-declared slot1 code
origin. Build and flash the same MPU-enabled kernel. Expected application
output is:

```text
[INFO][APP] Fault injection: slot0 read of slot1 memory
[ERROR][SECURITY] [SECURITY][FAULT] kind=MemManage
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

This proves the reverse processor-side rejection direction. Together with the
slot1-to-slot0 result, it supports a bidirectional CPU-side application-memory
isolation claim; DMA isolation and complete multi-application security remain
open.

The F405 acceptance run paired this package with the relocation fixture in
slot1. GDB observed slot0 entry once, then slot1 execution at `0x200100A0`,
while the slot1 progress marker increased from `0x00000000` to `0x0048D887`
without a second slot0 entry. Together with the slot1-to-slot0 result above,
this is bidirectional CPU-side isolation evidence; it is not DMA isolation.

## Peripheral-write test

Build and package the dedicated peripheral-write fixture:

```text
cd apps/dali-app-fault-peripheral-write
dali app build
```

Copy `target/thumbv7em-none-eabihf/debug/dali-app-fault-peripheral-write.amrn`
to the SD-card root, then repeat the same kernel build and flash procedure.
Expected application output is:

```text
[INFO][APP] Fault injection: peripheral memory write
[ERROR][SECURITY] [SECURITY][FAULT] kind=MemManage
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

This fixture intentionally writes a named test value to the peripheral origin
declared by the F405 target metadata. The expected result proves only
processor-side rejection of an unprivileged peripheral-MMIO write. The output
was accepted on the F405 hardware; this does not prove DMA isolation or
multi-application isolation.

## BusFault address test

Build and package the deterministic BusFault fixture:

```text
cd apps/dali-app-fault-bus
dali app build
```

Copy `target/thumbv7em-none-eabihf/debug/dali-app-fault-bus.amrn` to the
SD-card root and flash the normal ABI v3 MPU kernel. Before the application
executes, use the SWD/GDB test harness to temporarily disable the MPU. This is
a decoder-only test: it allows the invalid external access to reach the
BusFault handler. The change exists only in the live debug session and is
never part of the production firmware.

```text
cargo build -p dali-kernel --no-default-features --features board-stm32f405-sd,usb-cdc,abi-current,abi-mpu --target thumbv7em-none-eabihf
dali device flash f405 --transport probe --input target/thumbv7em-none-eabihf/debug/dali-kernel
```

Expected output is:

```text
[INFO][APP] Fault injection: BusFault address
[ERROR][SECURITY] [SECURITY][FAULT] kind=BusFault pc=Some(...) lr=Some(...) address=Some(...)
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

The fixture reads the documented reserved F405 code-region address declared by
the target manifest (`0x00100000`).
The temporary MPU disable is a debugger-owned test setup in
`scripts/gdb/bus-fault-mpu.gdb`; it is not part of the production isolation
map or firmware image. The harness also clears the sticky fault-status
register before the access. This test does not prove MPU isolation.

Start `probe-rs gdb` and connect GDB as described in the SWD procedure. Set a
breakpoint at `dali_kernel::security::launch::enter`, then run:

```text
source scripts/gdb/bus-fault-mpu.gdb
continue
```

The script must be sourced only after the kernel has configured its normal MPU
regions and the breakpoint at `launch::enter` has stopped execution. Then set
breakpoints at `dali_kernel::security::fault::handle_hard_fault` and
`dali_kernel::security::fault::handle_with_frame`, continue, and inspect the
registers and `CFSR`, `HFSR`, and `BFAR` values when the fault is reached.
The decoded `PC`, `LR`, and address were observed on the target and are
recorded in `docs/TESTING.md`; this confirms the precise BusFault boundary.

## Invalid-execution test

Build and package the execute-never fixture:

```text
cd apps/dali-app-fault-execution
dali app build
```

Copy `target/thumbv7em-none-eabihf/debug/dali-app-fault-execution.amrn` to
the SD-card root, then repeat the same kernel build and flash procedure.
Expected application output is:

```text
[INFO][APP] Fault injection: execute-never memory
[ERROR][SECURITY] [SECURITY][FAULT] kind=MemManage
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

This proves only processor-side rejection of instruction fetch from the
manifest-declared application-data region. It does not prove invalid vector
handling, PSP bounds, DMA isolation, or complete application isolation.

## Invalid-PSP test

Build and package the invalid-PSP fixture:

```text
cd apps/dali-app-fault-psp
dali app build
```

Build the kernel with the explicitly test-only service enabled:

```text
cargo build -p dali-kernel \
  --no-default-features \
  --features board-stm32f405-sd,usb-cdc,abi-current,abi-test-fixtures \
  --target thumbv7em-none-eabihf
```

Copy `target/thumbv7em-none-eabihf/debug/dali-app-fault-psp.amrn` to the
SD-card root, then repeat the same kernel build and flash procedure. Expected
application output is:

```text
[INFO][APP] Fault injection: invalid PSP bounds
[ERROR][SECURITY] [SECURITY][FAULT] kind=MemManage
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

The fixture requests a named test-only SVC service. The test-only kernel
handler then derives a PSP value four bytes inside the manifest-declared data
boundary and writes it before returning from SVC, so exception return must
cross the MPU boundary. The application cannot change PSP itself from
unprivileged Thread mode. Depending on the processor's exception-entry path,
the fault may be reported as `MemManage`, `UsageFault` with `INVPC`, or a
no-frame `HardFault`; each must reach kernel recovery. An `INVPC` record must
not decode the invalid PSP contents as an application frame. This proves only
the selected PSP-boundary behavior and does not prove context switching, DMA
isolation, or multi-application isolation.

The F405 hardware run on 2026-08-17 produced:

```text
[INFO][APP] Fault injection: invalid PSP bounds
[ERROR][SECURITY] [SECURITY][FAULT] kind=UsageFault status=0x00040000 pc=Some(...) lr=Some(...) address=None
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

This is successful hardware evidence for invalid-PSP exception-return
rejection and kernel recovery. The `0x00040000` status is the Cortex-M4
`INVPC` UsageFault bit. The diagnostic decoder reports no application frame for
this no-valid-frame condition.

## No-frame HardFault test

Build and package the no-frame fixture:

```text
cd apps/dali-app-fault-no-frame
dali app build
```

Copy `target/thumbv7em-none-eabihf/debug/dali-app-fault-no-frame.amrn` to the
SD-card root and build the test kernel with the same `abi-test-fixtures`
features shown above. The fixture emits:

```text
[INFO][APP] Fault injection: no-frame HardFault
```

The F405 test was verified through the Pico CMSIS-DAP probe and GDB. The
execution reached `HardFault`, `handle_hard_fault`,
`handle_with_frame(kind=HardFault)`, and `recover` in order. The SCB reported
`CFSR=0x00040000` (`INVPC`) with UsageFault disabled, and recovery entered the
kernel-owned `WFI` loop. This is SWD/GDB evidence for the no-frame handler and
recovery boundary; the
debugger's post-fault unwind output is not itself a pass criterion.

## SVC rejection-matrix test

Build and package the non-production SVC fixture:

```text
cd apps/dali-app-svc-rejections
dali app build
```

Copy `target/thumbv7em-none-eabihf/debug/dali-app-svc-rejections.amrn` to the
SD-card root, then repeat the same kernel build and flash procedure above.
The fixture sends an unknown service ID, kernel and peripheral pointers, an
oversized message length, and invalid UTF-8 through the raw ABI v3 SVC gateway.
Each result is logged only when the kernel returns the documented rejection
status; an accepted malformed request therefore produces no matching success
line.

Expected output is:

```text
[INFO][APP] SVC rejected unknown service
[INFO][APP] SVC rejected kernel pointer
[INFO][APP] SVC rejected peripheral pointer
[INFO][APP] SVC rejected oversized message
[INFO][APP] SVC rejected invalid UTF-8
[INFO][APP] SVC rejection matrix complete
```

The application remains alive after the matrix. This proves bounded rejection
of the selected malformed service requests; it does not prove authorization,
DMA isolation, or multi-application isolation.
