# Dali OS Architecture

## 1. Identity and vision

Dali OS is a small, modular, real-time-oriented embedded platform written in Rust for STM32 microcontrollers and, eventually, industrial machines and autonomous vehicles.

The name comes from Dali, the Georgian hunting goddess who could take the form of a bird or animal. The application package format is named `.amrn` — Amiran Native package — after Amirani, Dali's son. The project uses this mythology as part of its identity while keeping the technical platform understandable to an international audience.

The central architectural idea is:

> The kernel is the stable platform. Applications are independently built native packages loaded by the kernel.

## 2. Scope

### Long-term goals

- `no_std` Rust kernel for a focused STM32 platform;
- predictable scheduling and bounded real-time services;
- hardware services for sensors, motors, communication, storage, and power;
- independently built and deployed applications;
- a Rust SDK (`dali-sdk`);
- a package and device-management CLI (`dali-cli`);
- signed packages, compatibility checks, rollback, and safe recovery.

### Explicit non-goals for the first version

- desktop or general-purpose operating-system features;
- Linux compatibility;
- a full MMU-based process model;
- support for every STM32 family at once;
- C ABI compatibility;
- claiming memory isolation before an MPU or another isolation mechanism is implemented.

## 3. MVP definition

The first milestone proves one complete path on a single reference board:

1. boot a `no_std` Rust kernel;
2. initialize the STM32F411 clock, status LED, and logging;
3. initialize an SD card over SPI1;
4. read a FAT16/FAT32 filesystem;
5. discover an `.amrn` package in the SD card root directory;
6. validate its fixed 32-byte header, payload bounds, target, and CRC32 checksum;
7. copy its native ARM payload to a reserved SRAM region;
8. jump to its fixed ABI entry point;
9. observe a deterministic application LED pattern.

The MVP application is a RAM-loaded native module, not a sandboxed process. Kernel and application isolation is a later capability. The first application does not use interrupts, kernel services, shared logging, or a scheduler.

## 4. Reference platform

- MCU: STM32F411CEU6;
- board: WeAct BlackPill;
- CPU: ARM Cortex-M4F;
- target: `thumbv7em-none-eabihf`;
- clock target: 100 MHz;
- status LED: PC13;
- SD interface: SPI1;
  - SCK: PA5;
  - MISO: PA6;
  - MOSI: PA7;
- CS: PA4;
- initial logging: RTT;
- runtime logging: USB CDC-ACM;
- debug logging: RTT when an SWD probe is connected.

The repository also contains a secondary compile-time backend for the WeAct
Studio STM32F405RGT6 Core Board. This board exposes an on-board microSD socket
through the STM32 hardware SDIO peripheral in 4-bit mode (`PC12`, `PD2`, and
`PC8`–`PC11`) and an onboard status LED on `PB2`. It is intended for storage
bring-up and hardware experiments;
the F411 BlackPill remains the MVP reference target until AMRN target
compatibility is explicitly extended.

The exact board wiring, voltage requirements, SPI startup speed, and clock configuration must be documented before hardware acceptance testing.

Board support is selected at compile time. The kernel exposes one board facade,
while each supported board owns its pin mapping, clock setup, peripheral
ownership, and board-specific constants in a separate backend module. The
BlackPill F411 backend is the default MVP selection. Runtime board autodetection
is not assumed because MCU pin mappings and safe clock initialization must be
known before kernel startup.

## 5. Runtime layers

```text
dali-cli
    |
    v
.amrn package + CRC32 integrity check
    |
    v
Dali OS application loader
    |
    v
Dali OS kernel services and runtime
    |
    v
STM32 HAL and board support
    |
    v
STM32F411 hardware
```

The first implementation may keep the layers in one repository, but their contracts must remain explicit.

USB CDC logging has two deliberately separate ownership boundaries:

- `crates/dali-usb/` contains only `no_std`, transport-neutral bounded delivery
  primitives: fixed-size log records, FIFO queueing, partial-write handling,
  link state, overflow accounting, and host tests;
- `kernel/src/logging/usb_cdc.rs` owns the `usb-device` backend, STM32 USB
  resources, endpoint memory, USB identifiers, and device servicing policy.

The production backend services the `usb-device` state machine from the
STM32F4 `OTG_FS` interrupt. Main-context logging enters a bounded critical
section only to append a record to the queue; it never polls or writes the USB
endpoint. This keeps enumeration, CDC control requests, reconnects, and log
delivery serviceable while SDIO operations are inside HAL busy loops. The
interrupt strategy is target-checked but remains subject to physical USB
acceptance evidence.

CDC delivery has two progress points: accepting bytes into the `usbd-serial`
software buffer and flushing that buffer to the USB IN endpoint. The interrupt
backend must perform both operations and retry pending flushes on later USB
service events. Transport backpressure must not be treated as host delivery.
When main-context logging appends a record, it pends the same `OTG_FS`
interrupt so the USB owner can service the queue even when the host has not
generated a new bus event. This software-pended interrupt is a wake-up signal,
not a second USB owner or a timing workaround.
Boot-log draining additionally requires the CDC terminal's host-open signal
(`DTR`) so enumeration alone cannot consume records before the terminal is
ready to receive them.

The shared crate is not a hardware driver, application API, or interrupt
implementation. It must not depend on an MCU HAL or contain board-specific
values. The kernel remains the owner of logging policy and USB resource
ownership.

## 6. Kernel responsibilities

The kernel owns:

- startup and hardware initialization;
- clock, timer, interrupt, and watchdog policy;
- task execution and future scheduling primitives;
- fixed-size queues and event delivery;
- storage access;
- package discovery and validation;
- application loading and lifecycle;
- fault reporting and safe recovery;
- the boundary between applications and hardware services.

The kernel must not allow SD-card access, logging, or application work to block a future safety-critical control task. Those guarantees become enforceable as the scheduler and service model are implemented.

## 7. Application model

An `.amrn` package is an independently built native ARM application package. In the MVP it contains a payload linked for a predefined SRAM execution address and a fixed entry-point ABI.

The initial lifecycle is:

```text
discovered -> header_validated -> crc32_validated -> loaded -> started
                                                        |
                                                        v
                                                   running -> failed
```

An MVP application must:

- be built for the exact reference target;
- use `no_std` and the documented ABI;
- avoid assumptions about kernel-private symbols;
- fit within the declared payload and memory limits;
- report success through a visible, deterministic action.

Native execution in a shared address space is intentionally an MVP limitation. It does not provide sandboxing, privilege separation, or fault isolation.

## 8. `.amrn` package format

The first format revision uses a fixed 32-byte binary header followed by the native payload. The payload is linked for the fixed SRAM address `0x20008000`; relocation and dynamic linking are not part of the MVP.

The header specification must define every byte, field width, byte order, and alignment. The proposed logical fields are:

| Offset | Size | Field | Purpose |
| --- | ---: | --- | --- |
| `0x00` | 4 | Magic | ASCII `DALI` |
| `0x04` | 1 | Format version | MVP value is `1` |
| `0x05` | 1 | Target ID | MVP value is `0x01` (`STM32F411CEU6`) |
| `0x06` | 2 | Header size | Little-endian; MVP value is `32` |
| `0x08` | 4 | Payload size | Little-endian payload length |
| `0x0C` | 4 | Load address | Little-endian; MVP value is `0x20008000` |
| `0x10` | 4 | Execution offset | Little-endian offset from payload start |
| `0x14` | 4 | CRC32 | Little-endian payload checksum |
| `0x18` | 1 | ABI version | MVP value is `1` |
| `0x19` | 1 | Flags | Must be zero in the MVP |
| `0x1A` | 2 | Reserved | Must be zero in the MVP |
| `0x1C` | 4 | Reserved | Must be zero in the MVP |

The loader must reject:

- an invalid magic value or unsupported version;
- a target ID other than `0x01`;
- an ABI version other than `1`;
- a header size other than 32 bytes;
- non-zero flags or reserved fields;
- integer overflow while calculating offsets or sizes;
- an empty payload or a payload larger than 64 KiB;
- a payload outside the package or reserved executable SRAM region;
- a load address other than `0x20008000`;
- an unaligned or out-of-range execution offset;
- a CRC32 mismatch;
- an entry point outside the payload.

Digital signatures, encryption, manifests, version compatibility, and anti-rollback are post-MVP features. The format should reserve versioning space for them without pretending they already exist.

## 9. RAM loading and execution

The kernel reserves an explicit SRAM region for the MVP application:

```text
0x20000000 - 0x20007FFF   Kernel reserved RAM
0x20008000 - 0x20017FFF   Application region, 64 KiB
0x20018000 - 0x2001FFFF   Kernel runtime and stack RAM
```

The linker script must reserve this region from kernel code, data, stack, and future allocator use.

The loader must:

1. read and validate the header;
2. validate the complete package size before copying;
3. validate the load address and entry offset;
4. copy the payload into the reserved SRAM region;
5. derive the entry address from the load address and execution offset;
6. ensure the Cortex-M Thumb bit is set;
7. disable application-owned interrupts for the MVP;
8. transfer control through the documented `unsafe` entry ABI.

The application entry ABI, return behavior, panic behavior, and reset behavior are specified in `ABI.md`. Interrupt ownership is intentionally deferred.

## 10. Storage subsystem

The storage subsystem is responsible for:

- SPI1 initialization and SD-card communication;
- SD-card initialization at a safe low SPI frequency;
- block reads;
- FAT16/FAT32 read-only filesystem access;
- root-directory package discovery;
- bounded reads into loader-owned buffers.

The F405 SDIO block-read path keeps HAL card initialization but owns the data
FIFO drain in a board-local module. It consumes every available FIFO word,
including the final partial FIFO level, and maps SDIO timeout, CRC, and
overrun flags to typed storage errors. MMIO access is centralized there.
The kernel exposes this transport through the `sdio` capability feature; board
features enable capabilities and select pins/clocks separately.

The MVP does not need package installation, deletion, hot swap, or write support. SD-card replacement requires a reboot. The first acceptance application proves execution through a deterministic LED pattern rather than a shared logging API.

## 11. Future kernel architecture

After the MVP, the platform can grow toward:

- priority-based tasks and periodic scheduling;
- typed channels and fixed-size event queues;
- service discovery and capability policy;
- watchdog heartbeats and deadline monitoring;
- application lifecycle management;
- MPU-backed memory protection where supported;
- signed packages and secure boot;
- A/B updates and rollback;
- `dali-sdk` and `dali-cli` workflows.

Safety-critical behavior such as emergency stop, watchdog policy, power handling, and actuator limits must remain kernel-owned even when mission logic is supplied by an application.

## 12. Architectural principles

- Keep the first reference platform narrow and fully testable.
- Prefer explicit contracts over implicit linker or memory assumptions.
- Keep realtime paths bounded and allocation-free where required.
- Treat native application execution as trusted until isolation exists.
- Use audited external implementations for cryptography and filesystems where appropriate.
- Make every security claim match an implemented mechanism.
- Keep the kernel, SDK, CLI, package format, and application APIs versioned independently.
