# 5. Runtime layers

```text
dali
    |
    v
.amrn cartridge + CRC32 integrity check
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
an 8.3 short entry, so short-name-only enumeration would miss cartridges. The
filesystem layer exposes a read-only stream for the single cartridge selected by
the MVP loader policy. Zero matching files is `NotFound`; more than one
matching regular file is `Unsupported`. The loader never silently chooses
between multiple legacy application cartridges. The feature-gated v4 loader now
enumerates bounded root cartridges, validates their identity/slot metadata, and
reopens only the deterministic selection for the full load pass.

The post-v4 multi-application phase adds a bounded, hardware-neutral cartridge
catalog at the loader boundary. It accepts only headers already validated
against a target manifest, rejects duplicate identities and occupied slots,
and selects by manifest slot rather than directory order or filename. The
loader can copy each accepted identity cartridge into its own declared slot and
retain bounded loaded-context metadata. The feature-gated ABI v3 F405 path
also provides hardware-verified SysTick/PendSV context switching and
slot-specific MPU region switching for those declared contexts. This remains
an experimental execution path: the default ABI v2 path is single-cartridge,
and production multi-application lifecycle, replacement, restart, and
application-to-application policy are not complete.

The hardware-independent `dali-amrn` crate exposes header decoding and payload
validation separately as well as a contiguous-cartridge convenience API. The
split form is the loader boundary for bounded filesystem reads: a loader can
validate a fixed header and stream a payload without allocating a complete
64 KiB cartridge on the stack. The incremental validator must finish before any
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

Artifact installation uses a separate bounded protocol in `crates/dali-usb/`.
Its CRC-protected frames and lifecycle state machine are transport-neutral and
do not consume the logging queue. A future USB receive adapter must route only
installation frames to that state machine and keep CDC logging ownership
separate; protocol acceptance alone does not publish a Flash artifact.

The current frame contract is protocol version 1: a four-byte `DINS` magic,
one version byte, one command byte, a little-endian `u16` payload length, a
little-endian `u32` offset, a little-endian `u32` total artifact length, a
payload of at most 512 bytes, and an IEEE CRC32 trailer over the header and
payload. `Begin`, `Data`, `Validate`, `Commit`, and `Abort` are the only
commands. The state machine accepts only contiguous data offsets and requires
complete receipt before validation; target capacity remains a backend policy.

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
- cartridge discovery and validation;
- application loading and lifecycle;
- fault reporting and safe recovery;
- the boundary between applications and hardware services.

The kernel must not allow SD-card access, logging, or application work to block a future safety-critical control task. Those guarantees become enforceable as the scheduler and service model are implemented.
