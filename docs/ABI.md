# Kernel–Application ABI

## MVP status

The MVP uses a deliberately small native entry contract. It is not a stable public SDK ABI yet.

```rust
pub type EntryPoint = unsafe extern "C" fn(*const ServiceTable) -> !;

#[repr(C)]
pub struct ServiceTable {
    pub log: unsafe extern "C" fn(*const u8, usize) -> u32,
}
```

The exact ABI must be identical in the kernel and demo application. The application is linked for `0x20008000` and its complete image is copied to the reserved SRAM region before the jump.

The MVP ABI version is `2`. AMRN packages must declare this version in the
header, and the kernel must reject packages requiring another ABI version.

## Rules

- target: `thumbv7em-none-eabihf`;
- target ID: `0x02` (`STM32F405RGT6`);
- ABI version: `2`;
- architecture: ARM Cortex-M4F;
- entry address: `load_address + execution_offset`, with the Thumb bit set;
- application return: forbidden in the MVP;
- panic behavior: the application must not rely on a kernel panic handler;
- direct register access: allowed for the demo application only;
- kernel-private symbols: not available to applications;
- interrupts: disabled and not owned by applications in the MVP;
- service table: passed as the entry-point argument;
- logging service: bounded UTF-8 message submission through the kernel logger;
- shared memory: not available until an explicit layout is documented.

The current execution target is the STM32F405RGT6 board. The F411 BlackPill
manifest is generator-only and is not implied to be compatible with target ID
`0x02`.

The entry offset must be word-aligned and point inside the payload. The kernel
sets the Cortex-M Thumb bit before calling the entry point and passes a valid
service-table pointer. The application must never return from the entry point.

The first application proves execution with a deterministic LED pattern and
submits bounded messages through the logging service. The application never
accesses USB CDC or RTT directly.

## Safety boundary

Calling the entry point is `unsafe` because the kernel cannot prove that the loaded native code obeys the ABI. A malformed or incompatible application may corrupt kernel state or stop execution. The MVP therefore provides package validation and integrity checking, but not sandboxing or fault isolation.

## Future ABI work

Before `dali` is published, specify application context, task creation,
service discovery, shutdown, health reporting, capability handles, version
compatibility, and error representation. The v2 logging service is intentionally
the smallest initial service surface.

## Planned isolation ABI (not implemented)

The F405 isolation milestone will introduce a new ABI version rather than
silently changing ABI v2. The planned boundary is:

- application code runs in unprivileged Thread mode using a PSP;
- the kernel retains privileged Handler mode and MSP ownership;
- application services cross an SVC gateway with versioned service identifiers;
- applications cannot call privileged kernel functions through ordinary
  function pointers;
- MemManage, BusFault, UsageFault, and invalid exception returns are handled by
  a kernel-owned fault boundary;
- shared memory and application scheduling remain unsupported until their
  layouts and ownership rules are separately specified.

The exact SVC frame, service identifier encoding, PSP layout, fault recovery
state, and application memory regions must be specified and tested before an
ABI version increment or package compatibility change.

## Feature-gated ABI v3 boundary

ABI v3 is feature-gated and is not the default kernel execution path. ABI v2
remains the default until the implementation receives complete F405 hardware
fault-injection evidence.

The source code uses the central `abi-current` selector and the version-neutral
`abi-mpu` and `abi-relocation` capabilities. These are the only ABI-related
Cargo features; version names are not repeated in feature names. A future ABI
version changes the selector and adds only the implementation-specific contract
code; ordinary kernel modules do not need a version-name replacement.
The selector is enabled by `kernel/Cargo.toml`, while
`kernel/src/abi.rs` is the single source that maps the selected implementation
to its numeric package ABI version.

The repository contains a feature-gated SDK SVC call, kernel SVC frame
validator, bounded log dispatcher, MPU map, PSP transition, and kernel-owned
fault recovery behind the ABI v3 and `abi-mpu` features. These mechanisms
are available for controlled F405 testing but are not enabled by default.

### Execution mode

- Kernel bootstrap starts privileged and owns the MSP.
- An application runs in unprivileged Thread mode and owns a PSP-backed stack.
- Exception handlers execute privileged code on the MSP.
- The application never receives a pointer to kernel code or kernel-owned
  writable memory.
- The application entry point and return behavior must be replaced by an
  explicit v3 launch frame; the v2 entry function is not compatible.

### SVC gateway

All application services use one SVC gateway. The SVC immediate is a named
gateway value reserved for Dali services; the service identifier is passed in
the stacked `r0` slot. The initial service ABI is:

| Register | Meaning |
| --- | --- |
| `r0` | service identifier on entry; status code on return |
| `r1` | first argument or argument pointer |
| `r2` | second argument |
| `r3` | third argument |
| `r12` | reserved and must be preserved by the gateway |
| `lr`, `pc`, `xPSR` | exception frame supplied by the processor |

The first service identifier is the bounded logging service. Its argument
pointer must refer only to application-readable memory, and the kernel must
validate its length and address before reading it. Unknown identifiers,
invalid pointers, invalid lengths, and malformed exception frames return a
typed failure without entering application memory or kernel state mutation.

The gateway must verify that the SVC originated in unprivileged Thread mode
with the expected PSP frame. Handler-mode calls and invalid EXC_RETURN values
are rejected. The handler returns through the original exception frame only
after the service has completed its bounded work.

### F405 application memory contract

The current 64 KiB application boundary is decomposed into two aligned MPU
regions for v3:

```text
0x20008000 - 0x2000FFFF   Application code, 32 KiB, read/execute
0x20010000 - 0x20017FFF   Application data and PSP, 32 KiB, read/write, XN
```

The v3 application linker contract must place code and read-only data in the
first region and writable data, zero-initialized data, and the PSP stack in the
second region. The loader must validate both bounds before copying. The v2
single-image linker contract remains unchanged for existing packages.

The initial eight-region budget is:

| Region | Planned use | Unprivileged permission |
| ---: | --- | --- |
| 0 | Kernel SRAM | no access |
| 1 | Kernel runtime and stack SRAM | no access |
| 2 | Application code and read-only data | read/execute |
| 3 | Application data and PSP | read/write, XN |
| 4 | Ordinary peripheral registers | no access |
| 5 | Kernel Flash policy boundary | no access unless explicitly revised |
| 6 | Future shared service memory | reserved |
| 7 | Future shared service memory | reserved |

The planned MPU encoding treats application SRAM as normal cacheable,
bufferable memory and the ordinary peripheral window as shareable device
memory. These attributes are encoded by the board descriptor; writing the MPU
registers and selecting the unprivileged context remain separate steps.

This budget is a design target, not an enabled configuration. The privileged
background map and default-memory attributes must be selected so that an
unprivileged access cannot bypass the no-access boundaries.

### ABI v3 package and linker contract

ABI v3 packages use AMRN format version `2`; the format revision is required
because the v1 fixed header cannot represent separate code/data segments and
runtime stack reservations. The v2 package contract is defined in
`docs/AMRN_FORMAT.md`. The `dali-amrn` crate provides host-side parsing and
construction. The CLI can build and inspect ABI v3 packages, and the kernel has
a feature-gated streaming validator/copy path. The default kernel remains
ABI v2-only until the unprivileged launch path is complete.

For the current F405 target, the linker must emit:

- code and read-only data at `0x20008000` within a 32 KiB region;
- initialized data at `0x20010000` within a 32 KiB region;
- zero-initialized data followed by the PSP stack within that same data
  region;
- a word-aligned entry offset relative to the code origin;
- metadata sufficient to validate code size, initialized-data size,
  zero-data size, and PSP stack size.

The package payload stores code followed by initialized data. Zero data and
the PSP stack are reservations, not file bytes. The loader must copy the two
file segments only after validating every region bound, then clear the
zero-data range and construct the PSP launch frame. This is a fixed-address
single-application contract; PIC, relocation, and multiple slots remain
deferred.

Format version 3 is the movable ABI v3 contract. Its host-side header,
relocation-entry validation, CLI extraction, package emission, and inspection
are defined in docs/AMRN_FORMAT.md and implemented in dali-amrn::v3 and the
CLI. The kernel accepts and applies it only with the explicit
`abi-relocation` feature; the default kernel path remains unchanged. The
current feature-gated loader still uses the target manifest's declared origins
and does not select multiple application slots. The relocation table is
intentionally a new format revision rather than an interpretation of v2
address fields.

The launch frame is kernel-generated. Its PC is the validated Thumb entry,
its PSP is within the declared stack bounds, its unused argument registers are
cleared, and its link value cannot return into kernel code. The package cannot
provide an exception-return value or a privileged function pointer. ABI v2
packages remain on the existing direct `ServiceTable` entry path and must not
be interpreted as ABI v3 packages.

Kernel SRAM, kernel runtime/stack SRAM, and ordinary peripheral registers are
not application-accessible. The v3 design does not claim DMA isolation,
confidentiality of readable Flash, package authenticity, or recovery from
arbitrary native faults.

### Fault boundary

MemManage, BusFault, UsageFault, and invalid exception-return paths must enter
a privileged kernel fault boundary. The boundary records a bounded fault
record, marks the current application terminated, and returns to a kernel-owned
control path. It must not unwind or reuse an application PSP as a kernel stack.
The recovery assembly uses a kernel-stack frame and does not reuse the
application PSP.

The feature-gated kernel now defines a bounded `FaultRecord` and diagnostic
MemManage, BusFault, and UsageFault handlers. These handlers report the SCB
status and return through a kernel-stack recovery frame to a bounded
kernel-owned recovery loop. They do not return to an application or scheduler,
and they do not change ABI v2 behavior.

The additional kernel feature `abi-mpu` programs the descriptor-backed MPU
map during bootstrap and provides the feature-gated PendSV transition into the
prepared PSP frame. MPU activation is two-phase: while the privileged loader
copies the validated code and data segments, both application regions are
kernel-only and non-executable; after copying and zero-initialization complete,
the kernel changes the code region to unprivileged read/execute and the data
region to unprivileged read/write, execute-never, before entering the PSP
context. This ordering prevents the protection map from blocking a valid
kernel-owned load. The default kernel does not enable this path. It must not be
treated as application isolation until fault recovery and F405 fault-injection
evidence are complete.

The default loader and SDK use ABI v2 packages with the direct `ServiceTable`
entry contract. The feature-gated ABI v3 loader validates and copies the
separate segments, prepares a kernel-owned launch frame, and materializes its
basic exception frame inside the validated application stack reservation. Only
the explicitly enabled `abi-mpu` path selects PSP, activates the descriptor
backed MPU map, and enters through PendSV; the default kernel does none of
these. ABI v3 host packages cannot be treated as isolated until fault recovery
and F405 fault-injection evidence are complete.
