# 3. MVP definition

The first milestone proves one complete path on a single reference board:

1. boot a `no_std` Rust kernel;
2. initialize the reference board clock, status LED, SDIO, and logging;
3. initialize an SD card over the STM32 hardware SDIO interface;
4. read a FAT16/FAT32 filesystem;
5. discover an `.amrn` cartridge in the SD card root directory;
6. validate its fixed 32-byte header, payload bounds, target, and CRC32 checksum;
7. copy its native ARM payload to a reserved SRAM region;
8. jump to its fixed ABI entry point;
9. observe a deterministic application LED pattern and three application log messages.

The baseline ABI v2 application is a RAM-loaded native module, not a sandboxed
process. The first application does not use interrupts or a scheduler; its
only kernel service is the bounded logging entry defined by ABI v2. The
feature-gated ABI v3 path adds processor-side isolation, PSP-based context
switching, and MPU region switching for declared application contexts; its
remaining limitations are recorded below.

The first post-MVP isolation milestone is a feature-gated F405 execution mode.
It implements privileged kernel bootstrap, unprivileged application Thread
mode, PSP ownership, SysTick/PendSV context switching, slot-specific MPU
region switching, SVC-based services, and a kernel-owned fault boundary. These
processor-side mechanisms and their listed fault-injection cases have F405
hardware evidence, but the result must not be described as a microkernel,
secure boot, complete sandbox, or production multi-application isolation:
arbitrary DMA-controller isolation, complete application lifecycle policy,
and application-to-application policy remain open. The current lifecycle
policy requires manual reset after termination, does not provide rollback on
read-only storage, and arms the hardware watchdog before opaque
storage-transport initialization, so a vendor HAL polling hang becomes
bounded Safe Mode recovery. Block reads, repository loading, and application
execution remain covered by the watchdog heartbeat and bounded
storage/verification progress hooks.

## 4. Reference platform

The MVP reference platform is the WeAct Studio STM32F405RGT6 Core Board with
an ARM Cortex-M4F and target `thumbv7em-none-eabihf`. Its physical pinout,
clock, wiring, and memory facts are maintained in the
[STM32F405 board documentation](../boards/stm32f405/README.md).

- initial logging: RTT;
- runtime logging: USB CDC-ACM;
- debug logging: RTT when an SWD probe is connected.

The repository also contains a declarative BlackPill F411 board profile for
generator testing. It has no kernel backend and is not an accepted execution
target until its target contract and hardware implementation are specified.

The exact board wiring, voltage requirements, SPI startup speed, and clock
configuration are recorded in the board documentation before hardware
acceptance testing.

Board support is selected at compile time. The kernel exposes one board facade,
while each supported board owns its pin mapping, clock setup, peripheral
ownership, and board-specific constants in a separate backend module. The
F405 backend is the current MVP selection. Runtime board autodetection
is not assumed because MCU pin mappings and safe clock initialization must be
known before kernel startup.

Manufacturer-supplied board facts and Dali target compatibility metadata are
declared in repository-level `targets/*.toml` manifests. A host build step
validates those manifests and generates a typed registry for the CLI. The
kernel still owns the mapping from the selected profile to typed HAL resources;
the manifest does not replace compile-time GPIO, RCC, DMA, or peripheral
ownership code.
