# ABI v3 kernel-memory fault test

`apps/dali-app-fault-kernel` is a non-production F405 test fixture. It first
uses the ABI v3 logging service, then performs one volatile read at the
kernel-reserved address declared by the generated F405 target metadata.

The expected result is a security fault record followed by the kernel-owned
recovery record. The test must not be used as an application template and is
not part of the default workspace build.

## Build

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
