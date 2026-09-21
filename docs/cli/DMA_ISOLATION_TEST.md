# F405 DMA isolation diagnostic fixture

`apps/dali-app-dma-denial` is a non-production AMRN format v4 / ABI v3
application fixture. It requests the test-only DMA service through the normal
SVC gateway. The kernel must reject the request and the fixture then logs the
target-visible acceptance message.

The F405 diagnostic feature publishes `DMA_TRACE_MARKER` in the
manifest-declared DMA section. The marker changes to `0xD1A00001` after DMA2
and SDIO are configured, immediately before the bounded receive loop. It
changes to `0xD1A00002` after the DMA transfer is stopped. These markers exist
only in the diagnostic build.

## Build

Build the real application cartridge from its fixture directory:

```text
cd apps/dali-app-dma-denial
cargo run --manifest-path ../../crates/dali-cli/Cargo.toml -- app build
```

Register the cartridge in the existing Binary v2 repository and keep it under
the repository `amrns/` directory with its matching metadata. This kernel
path does not scan the FAT root for arbitrary AMRN files. Safely unmount the
card before inserting it into the F405 board.

Build the diagnostic kernel:

```text
cargo build -p dali-firmware --bin dali-f405 --release --no-default-features \
  --features stm32f405,usb-cdc,abi-context-switch,abi-relocation,abi-authentication,repository-loader,storage-write,dma-test-fixture \
  --target thumbv7em-none-eabihf
```

## Active DMA capture

Start the probe GDB server with the explicit F405 selector:

```text
probe-rs gdb --chip STM32F405RGTx --protocol swd --speed 1000
```

In a second terminal, connect GDB and source:

```text
target remote :1337
source scripts/gdb/f405-dma-isolation.gdb
continue
```

The active capture is accepted when the marker is `0xD1A00001` and the
snapshot records DMA2 Stream 3 enabled with non-zero `NDTR`, `M0AR` inside the
target manifest DMA region, and SDIO `DCTRL.DMAEN` enabled. `DCOUNT` is a
storage-driver sequence observation and is not a requirement for the DMA
ownership policy evidence. The completion marker and a later zeroed Stream 3
register snapshot are recorded separately as lifecycle stop evidence.

## Application DMA denial

The console must contain both records:

```text
[ERROR][SECURITY] [SECURITY] Application DMA request rejected
[INFO][APP] DMA application request rejected
```

This proves the test-only application request was rejected at the kernel SVC
boundary. It does not grant applications a DMA API and does not claim future
peripheral backends are isolated until each backend is reviewed.

## Acceptance record

Record the board revision, wiring, firmware image hash, target manifest
revision, SD card/filesystem, power source, probe serial, SWD speed, marker
value, DMA2 register snapshot, SDIO register snapshot, console output, and
result. A source build or idle register snapshot is not sufficient for closing
DMA isolation.
