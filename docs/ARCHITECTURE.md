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
- a Rust SDK (`dali`);
- a package and device-management CLI (`dali` command, `dali-cli` package);
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
2. initialize the STM32F405 clock, status LED, SDIO, and logging;
3. initialize an SD card over the STM32 hardware SDIO interface;
4. read a FAT16/FAT32 filesystem;
5. discover an `.amrn` package in the SD card root directory;
6. validate its fixed 32-byte header, payload bounds, target, and CRC32 checksum;
7. copy its native ARM payload to a reserved SRAM region;
8. jump to its fixed ABI entry point;
9. observe a deterministic application LED pattern and three application log messages.

The baseline ABI v2 application is a RAM-loaded native module, not a sandboxed
process. The first application does not use interrupts or a scheduler; its
only kernel service is the bounded logging entry defined by ABI v2. The
feature-gated ABI v3 path adds processor-side isolation for one application;
its remaining limitations are recorded below.

The first post-MVP isolation milestone is a feature-gated single-application
F405 execution mode. It implements privileged kernel bootstrap, unprivileged
application Thread mode, PSP ownership, MPU regions, SVC-based services, and a
kernel-owned fault boundary. These processor-side mechanisms and their listed
fault-injection cases have F405 evidence, but the result must not be described
as a microkernel, secure boot, or complete sandbox: DMA ownership and
multi-application isolation remain open. The current lifecycle policy requires
manual reset after termination, does not provide rollback on read-only storage,
and arms the hardware watchdog only from the kernel-owned heartbeat path.

## 4. Reference platform

- MCU: STM32F405RGT6;
- board: WeAct Studio STM32F405RGT6 Core Board;
- CPU: ARM Cortex-M4F;
- target: `thumbv7em-none-eabihf`;
- clock target: 168 MHz from an 8 MHz HSE;
- status LED: PB2, active-high;
- SD interface: hardware SDIO, 4-bit mode;
- initial logging: RTT;
- runtime logging: USB CDC-ACM;
- debug logging: RTT when an SWD probe is connected.

The repository also contains a declarative BlackPill F411 board profile for
generator testing. It has no kernel backend and is not an accepted execution
target until its target contract and hardware implementation are specified.

The exact board wiring, voltage requirements, SPI startup speed, and clock configuration must be documented before hardware acceptance testing.

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

## 5. Runtime layers

```text
dali
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
STM32F405 hardware
```

The first implementation may keep the layers in one repository, but their contracts must remain explicit.

The filesystem backend is read-only. Its root scan does not invoke the
`embedded-sdmmc` volume close operation because that operation writes the FAT32
FSInfo sector even when the scan performed no application writes. The scan
releases its directory handle, transfers the volume to a raw handle, and lets
the manager lifetime end without write-back. This preserves the read-only
storage contract and treats a write request as unsupported rather than silently
discarding it.

AMRN root-directory matching uses the FAT long-file-name API and ASCII
case-insensitive comparison. A four-character `.amrn` extension cannot fit in
an 8.3 short entry, so short-name-only enumeration would miss packages. The
filesystem layer exposes a read-only stream for the single package selected by
the MVP loader policy. Zero matching files is `NotFound`; more than one
matching regular file is `Unsupported`. The loader never silently chooses
between multiple legacy application packages. The feature-gated v4 loader now
enumerates bounded root packages, validates their identity/slot metadata, and
reopens only the deterministic selection for the full load pass.

The post-v4 multi-application phase adds a bounded, hardware-neutral package
catalog at the loader boundary. It accepts only headers already validated
against a target manifest, rejects duplicate identities and occupied slots,
and selects by manifest slot rather than directory order or filename. The
loader can now copy each accepted identity package into its own declared slot
and retain bounded loaded-context metadata, but the runtime still enters only
the first context. This does not enable concurrent application execution or
context switching; legacy ABI paths keep the single-package policy.

The hardware-independent `dali-amrn` crate exposes header decoding and payload
validation separately as well as a contiguous-package convenience API. The
split form is the loader boundary for bounded filesystem reads: a loader can
validate a fixed header and stream a payload without allocating a complete
64 KiB package on the stack. The incremental validator must finish before any
payload copy to the executable SRAM region; a later loader may perform a
validation pass followed by a separate copy pass.

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

The hardware-neutral `dali-device` crate defines normalized host discovery
records, transport/state/capability vocabulary, and deterministic
deduplication. It does not enumerate devices or depend on a host USB, DFU, or
debug-probe implementation. Those adapters remain CLI-owned and are subject
to the device discovery contract.

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

Native execution in a shared address space is intentionally a baseline ABI v2
limitation. ABI v3 provides a separate feature-gated processor-side boundary
for one application; it does not provide DMA isolation or multi-application
isolation.

## 8. `.amrn` package format

The first format revision uses a fixed 32-byte binary header followed by the native payload. The payload is linked for the fixed SRAM address `0x20008000`; relocation and dynamic linking are not part of the MVP.

The header specification must define every byte, field width, byte order, and alignment. The proposed logical fields are:

| Offset | Size | Field | Purpose |
| --- | ---: | --- | --- |
| `0x00` | 4 | Magic | ASCII `DALI` |
| `0x04` | 1 | Format version | MVP value is `1` |
| `0x05` | 1 | Target ID | Current MVP value is `0x02` (`STM32F405RGT6`) |
| `0x06` | 2 | Header size | Little-endian; MVP value is `32` |
| `0x08` | 4 | Payload size | Little-endian payload length |
| `0x0C` | 4 | Load address | Little-endian; MVP value is `0x20008000` |
| `0x10` | 4 | Execution offset | Little-endian offset from payload start |
| `0x14` | 4 | CRC32 | Little-endian payload checksum |
| `0x18` | 1 | ABI version | Current MVP value is `2` |
| `0x19` | 1 | Flags | Must be zero in the MVP |
| `0x1A` | 2 | Reserved | Must be zero in the MVP |
| `0x1C` | 4 | Reserved | Must be zero in the MVP |

The loader must reject:

- an invalid magic value or unsupported version;
- a target ID other than `0x02`;
- an ABI version other than `2`;
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

The current STM32F405 board is the only AMRN execution target. The F411
profile is metadata-only and is not silently treated as compatible with the
F405 target.

## 10. Storage subsystem

The storage subsystem is responsible for:

- SDIO initialization and SD-card communication;
- SD-card initialization at a safe low SPI frequency;
- block reads;
- FAT16/FAT32 read-only filesystem access;
- root-directory package discovery;
- bounded reads into loader-owned buffers.

The F405 SDIO block-read path keeps HAL card initialization but owns the data
FIFO drain in a board-local module. It uses the documented DMA2 Stream 3,
Channel 4 receive request and an aligned word buffer, then maps SDIO timeout,
CRC, overrun, and DMA errors to typed storage errors. MMIO access is
centralized there. The read command is issued without waiting for its response
so DMA can begin receiving as soon as the card asserts data activity. USB
servicing remains available while the storage transfer is blocking. SDIO
hardware flow control remains disabled because the STM32F405/F40x device
errata report clock glitches and CRC errors when it is enabled.
The DMA controller is reset before each transfer setup to remove stale stream
state, and a completed SDIO command with no active receive transfer is treated
as a bounded transport failure. DMA FIFO mode uses a full threshold and
incremental four-word bursts so the final receive words are not stranded below
the direct-mode request threshold. If the SDIO data counter reaches zero while
DMA has a bounded tail pending, that tail is drained directly from the FIFO.
The kernel exposes this transport through the `sdio` capability feature; board
features enable capabilities and select pins/clocks separately.

The MVP does not need package installation, deletion, hot swap, or write support. SD-card replacement requires a reboot. The first acceptance application proves execution through a deterministic LED pattern and bounded application logging.

### Board-agnostic repository and durable-storage boundary

Repository loading and trust-store installation are defined above the board
storage implementation. The kernel exposes three logical boundaries:

```text
BlockDevice
    -> filesystem / durable-artifact adapter
    -> DurableStorageAdapter and RepositoryStreamStorage
    -> persistence coordinator and repository loader
```

`BlockDevice` owns fixed-block transfer only. `DurableStorageAdapter` owns the
kernel's bounded Slot A, Slot B, and commit-journal artifacts.
`RepositoryStreamStorage` owns logical metadata-document and content-addressed
AMRN streams. Neither
trait mentions SDIO, FAT, STM32, pins, clocks, DMA, or a physical filename.

The F405 implementation is one adapter chain below these contracts:

```text
STM32F405 SDIO -> F405 block adapter -> FAT32 adapter -> logical storage traits
```

The repository loader consumes `RepositoryStreamStorage` through bounded
two-pass role verification. The complete Root -> Timestamp -> Snapshot ->
Targets -> Delegation -> Revocation -> Package -> AMRN chain is assembled before
the execution loader receives a package. The loader does not enumerate
directories or choose packages from filenames. This keeps future board adapters
replaceable without changing trust policy or loader logic.

When the `repository-loader` feature is enabled, the F405 boot path constructs
the concrete FAT adapter, requests the board target profile, selects every
matching executable Binary v2 Targets record within the bounded execution
capacity, opens each lowercase content-addressed `packages/<sha256>.amrn`
object, and passes the verified streams to the existing slot/relocation loader.
The target memory contract is resolved from each manifest-owned slot at the
dependency-injection boundary; the generic repository loader contains no F405
or SDIO types. The default MVP build keeps
the legacy root-package path because repository boot remains feature-gated until
hardware acceptance.

The concrete F405 adapter is `FatRepositoryStorage<D>`. Its constructor accepts
an explicit `RepositoryMetadataFormat` (`JsonV1` or `BinaryV2`) and resolves the
board-agnostic logical documents through `metadata/`,
`metadata/delegations/`, and `packages/`, including FAT long filenames, while durable artifacts remain
the kernel-owned root files `DALI-ACT.BIN`, `DALI-CAN.BIN`, and `DALI-CMT.BIN`.
The adapter also exposes `with_content_addressed_package()`, which opens only
the lowercase SHA-256 package filename under `packages/` and hands the file to
the existing bounded AMRN execution loaders.

The adapter now exposes the streaming contract directly. The shared Binary v2
envelope parser validates fragmented envelopes without retaining their body;
typed Root, Timestamp, Snapshot, Delegation, and Revocation parsers are
organized under `dali-metadata/src/parser/streaming/`, and
`verify_binary_role_envelope` provides the parse-and-replay second pass. The
kernel repository loader now also contains a bounded two-pass AMRN v5 validator
under `loader/repository/amrn.rs`, a generic role-stream capture/replay helper
under `loader/repository/chain.rs`, Root anchor membership wiring through the
target manifest, and the board-agnostic `load_binary_repository()` chain
assembler. `load_repository_package()` is the boot handoff: it injects the F405
adapter into the generic chain and then opens only the digest-selected package
for the existing execution pipeline.

### Future multi-application package selection

The current read-only filesystem contract intentionally accepts exactly one
root `.amrn` package and rejects ambiguous selection. Before multiple
applications can be loaded into independent slots, Dali must define package
identity and selection independently from filenames, an explicit mapping from
the selected package to a manifest-owned slot, and bounded behavior for
missing, duplicate, incompatible, or already-reserved packages. The slot
manager must consume that validated selection; it must not infer ownership from
directory order or package names.

## 11. Future kernel architecture

After the MVP, the platform can grow toward:

- priority-based tasks and periodic scheduling;
- typed channels and fixed-size event queues;
- service discovery and capability policy;
- watchdog heartbeats and deadline monitoring;
- application lifecycle management;
- extension of the current MPU boundary to multiple applications where supported;
- signed packages and secure boot;
- A/B updates and rollback;
- `dali` CLI workflows.

Safety-critical behavior such as emergency stop, watchdog policy, power handling, and actuator limits must remain kernel-owned even when mission logic is supplied by an application.

### Isolation design constraints

The STM32F405 Cortex-M4 provides eight unified MPU regions. The current
application region starts at `0x20008000` and spans 64 KiB, so an isolation
implementation must represent it with aligned power-of-two regions or change
the memory contract. Code, writable data, and the application PSP stack may
require different permissions and execute-never attributes.

The F405 also exposes CCM RAM at `0x10000000`; the current linker contract
places ordinary kernel runtime/static state there while SDIO and USB DMA
buffers remain in DMA-accessible SRAM. PIC compiler flags alone do not define a
relocatable raw AMRN contract. Explicit relocation metadata now defines the
implemented movable-package path; an alternative PIC/RWPI ABI remains future
work.

## 12. Architectural principles

- Keep the first reference platform narrow and fully testable.
- Prefer explicit contracts over implicit linker or memory assumptions.
- Keep realtime paths bounded and allocation-free where required.
- Treat native application execution as trusted until isolation exists.
- Use audited external implementations for cryptography and filesystems where appropriate.
- Make every security claim match an implemented mechanism.
- Keep the kernel, SDK, CLI, package format, and application APIs versioned independently.
