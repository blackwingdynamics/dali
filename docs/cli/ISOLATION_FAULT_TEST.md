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

Build the kernel with `abi-v3-mpu`, connect the Pico as the SWD probe, and
flash the kernel through the probe. Keep RTT output visible:

```text
cargo build -p dali-kernel --no-default-features --features board-stm32f405-sd,usb-cdc,abi-v3,abi-v3-mpu --target thumbv7em-none-eabihf
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

Copy `target/thumbv7em-none-eabihf/debug/dali-app-fault-psp.amrn` to the
SD-card root, then repeat the same kernel build and flash procedure. Expected
application output is:

```text
[INFO][APP] Fault injection: invalid PSP bounds
[ERROR][SECURITY] [SECURITY][FAULT] kind=MemManage
[ERROR][SECURITY] [SECURITY][FAULT] Application terminated; kernel recovery active
```

The fixture places PSP four bytes inside the declared data boundary before
issuing `SVC`, so exception stacking must cross the MPU boundary. Depending on
the processor's exception-entry path, the fault is reported as `MemManage` or
as a no-frame `HardFault`; both must reach kernel recovery. This proves only
the selected PSP-boundary behavior and does not prove context switching, DMA
isolation, or multi-application isolation.

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
