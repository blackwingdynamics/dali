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

## Feature-gated isolation ABI

The F405 isolation milestone introduces a feature-gated ABI v3 rather than
silently changing ABI v2. The implemented boundary is:

- application code runs in unprivileged Thread mode using a PSP;
- the kernel retains privileged Handler mode and MSP ownership;
- application services cross an SVC gateway with versioned service identifiers;
- applications cannot call privileged kernel functions through ordinary
  function pointers;
- MemManage, BusFault, UsageFault, and invalid exception returns are handled by
  a kernel-owned fault boundary;
- shared memory and application scheduling remain unsupported until their
  layouts and ownership rules are separately specified.

The SVC frame, service identifier encoding, PSP layout, fault recovery state,
and application memory regions are specified in this document and covered by
host tests plus the recorded F405 hardware evidence below. No ABI v4 or
multi-application contract is enabled.

## Feature-gated ABI v3 boundary

ABI v3 is feature-gated and is not the default kernel execution path. ABI v2
remains the default. ABI v3 requires explicit feature selection and is still
an experimental single-application isolation path. The processor-side fault
cases and repeated invalid-PSP recovery are hardware-tested; watchdog and
lifecycle policy, DMA isolation, and multi-application isolation remain open.

The source code uses the central `abi-current` selector and the version-neutral
`abi-mpu` and `abi-relocation` capabilities. These are the only ABI-related
Cargo features; version names are not repeated in feature names. A future ABI
version changes the selector and adds only the implementation-specific contract
code; ordinary kernel modules do not need a version-name replacement.
The `dali-amrn::compatibility` module is the shared ABI-family and
AMRN-format compatibility table used by the kernel-facing build contracts and
the CLI. It rejects unknown ABI versions and incompatible explicit format
requests before package construction.
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

### Application lifecycle foundation

The kernel runtime defines a bounded lifecycle record for one discovered
package and its manifest-owned slot:

```text
Discovered -> Loaded -> Ready -> Running -> Faulted -> Recovering -> Terminated
```

Each transition is explicit and invalid transitions are rejected. A declared
slot must match the slot allocation recorded at load time. `Terminated` is a
terminal state: the current runtime does not implicitly restart an application,
schedule another application, or switch MPU contexts. Those behaviors require
separate lifecycle, scheduler, and protection contracts.

### Active context ownership foundation

The runtime currently permits at most one active application context. A ready
lifecycle must be explicitly activated before it can enter `Running`, and a
second activation is rejected. The active context retains the package identity
and manifest-owned slot allocation. Retirement is accepted only after the
lifecycle reaches `Terminated`; no implicit restart or context switch exists.
The v4 loader creates the lifecycle after validated copy and relocation, then
the launch path performs the `Loaded -> Ready -> Running` transition before
MPU activation. Legacy v2/v3 package paths retain their existing launch
behavior until they receive an identity-aware lifecycle contract.
F405 hardware has confirmed these transitions together with two-package
loading, manifest slot boundaries, slot0 execution, and the complete fault
transition. The current policy requires a manual reset after termination,
provides no rollback on the read-only package boundary, and does not arm a
watchdog without a bounded feed owner.

### Context-switch foundation

The runtime defines a hardware-neutral saved-context record containing the
application PSP, `r4..r11`, `CONTROL`, and `EXC_RETURN`. Each scheduler record
also retains the manifest-owned application slot that must accompany the CPU
state during a future protected switch. A fixed-capacity context table enforces
one running owner, bounded insertion, terminal exclusion, and
round-robin-style selection of ready contexts. This is a scheduler contract
only: SysTick does not yet request a switch, PendSV does not yet save or restore
multiple contexts, and MPU regions are not switched between applications.

The scheduling contract also defines a board-independent tick budget. Each
target profile declares the platform timer frequency and the quantum in timer
ticks. The budget emits a one-shot PendSV request when its quantum expires, and
the deferred handler remains responsible for the actual register save/restore.

The optional `abi-context-switch` feature now provides target-compiled ARM
save/restore primitives for the kernel-owned context record. The primitives do
not select a context, validate a PSP, program the MPU, or enable an interrupt;
they remain separate from the default one-context launch path until scheduler
ownership and validation are integrated.

The scheduler facade now owns the bounded sequence around those primitives:
SysTick records a preemption request, PendSV saves the active record, and the
facade selects the next ready context. A feature-gated target SysTick hook now
feeds the scheduler and only pends PendSV when both active and ready contexts
exist. The selected platform enables SysTick only after the first application
context is active. Fault recovery permanently retires the active scheduler
context and can select the next ready record; the ARM recovery transfer and
MPU reprogramming still require target evidence. F405 hardware has verified
repeated CPU context switching with independent slot progress markers, but
has now also verified faulted-context exclusion and continued slot0 execution
after slot1 recovery; F405 GDB evidence also verified the slot-specific MPU
code/data bases at `restore_selected`. Full application-to-application
isolation remains a separate claim.

The scheduler capacity is generated from the selected target's declared
isolation slots. It is not derived from filesystem enumeration limits and is
not a board-independent hardcoded slot count.

The preemption quantum and timer frequency are declared by the target profile.
The kernel does not embed a deployment-specific timing literal in scheduler
code.

Kernel-owned scheduler storage is initialized once during bootstrap. Mutable
access requires an explicit interrupt-exclusivity guarantee; uninitialized or
repeated initialization is rejected before PendSV integration.

With `abi-context-switch`, bootstrap constructs this storage from the selected
target profile. The loader registers validated application launch records,
activates the first scheduler context, and then enables the target SysTick.
The scheduler contract includes permanent retirement of a faulted context;
F405 hardware has verified that recovery resumes a ready context without
re-entering the terminated slot. F405 GDB evidence also verified that
`restore_selected` observes the slot-specific MPU code/data regions.

`prepare_pendsv` makes that sequence explicit and bounded: without a pending
request it leaves the active context unchanged; with a request it records the
outgoing state before selecting the next ready context. Register restoration and
exception return remain the responsibility of the low-level adapter.

The kernel fault boundary updates the shared runtime state atomically through
`Faulted -> Recovering -> Terminated` before entering the recovery loop. This
state channel records ownership without exposing a mutable application pointer
to exception handlers. F405 hardware has confirmed the complete transition
with the v4 invalid-PSP application fixture; this proves lifecycle accounting
and recovery entry, not restart, scheduling, or complete application isolation.

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

### Target application memory contract

The target manifest owns an ordered slot table. The current F405 manifest
decomposes its 64 KiB application pool into two aligned slots for v3:

```text
0x20008000 - 0x2000BFFF   Slot 0 code, 16 KiB, read/execute
0x2000C000 - 0x2000FFFF   Slot 0 data/PSP, 16 KiB, read/write, XN
0x20010000 - 0x20013FFF   Slot 1 code, 16 KiB, read/execute
0x20014000 - 0x20017FFF   Slot 1 data/PSP, 16 KiB, read/write, XN
```

The v3 application linker contract places the current application in the
manifest's active slot (slot 0). The loader and MPU validate that slot before
copying or launching. Slot 1 is declared and validated by the target contract,
but concurrent execution and context switching remain future work. The v2
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

This map is enabled only by the explicit `abi-mpu` feature and has been
hardware-tested for the documented single-application fault cases. It is not
the default kernel configuration, and the privileged background map and
default-memory attributes must continue to prevent bypass of the no-access
boundaries.

### ABI v3 package and linker contract

ABI v3 packages use AMRN format version `2`; the format revision is required
because the v1 fixed header cannot represent separate code/data segments and
runtime stack reservations. The v2 package contract is defined in
`docs/AMRN_FORMAT.md`. The `dali-amrn` crate provides host-side parsing and
construction. The CLI can build and inspect ABI v3 packages, and the kernel has
a feature-gated streaming validator/copy path. The default kernel remains
ABI v2-only; the feature-gated path is experimental and its hardware evidence
is complete for the documented single-application F405 cases. It does not
enable concurrent applications or claim the remaining security guarantees.

For a target selected through the manifest, the linker must emit:

- code and read-only data within the active slot's declared code region;
- initialized data within the active slot's declared data region;
- zero-initialized data followed by the PSP stack within that same data
  region;
- a word-aligned entry offset relative to the code origin;
- metadata sufficient to validate code size, initialized-data size,
  zero-data size, and PSP stack size.

The package payload stores code followed by initialized data. Zero data and
the PSP stack are reservations, not file bytes. The loader must copy the two
file segments only after validating every region bound, then clear the
zero-data range and construct the PSP launch frame. This is a fixed-address
single-application contract. Explicit relocation is defined by AMRN format
version 3 and hardware-tested separately; compiler PIC or RWPI alone is not
treated as an AMRN relocation contract. Multiple-package execution remains
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
kernel-owned load. The default kernel does not enable this path. It provides
single-application processor-side isolation evidence, but it must not be
treated as complete application isolation because DMA isolation, lifecycle
policy, and multi-application isolation remain open.

The default loader and SDK use ABI v2 packages with the direct `ServiceTable`
entry contract. The feature-gated ABI v3 loader validates and copies the
separate segments, prepares a kernel-owned launch frame, and materializes its
basic exception frame inside the validated application stack reservation. Only
the explicitly enabled `abi-mpu` path selects PSP, activates the descriptor
backed MPU map, and enters through PendSV; the default kernel does none of
these. ABI v3 host packages cannot be treated as isolated until fault recovery
and F405 fault-injection evidence are complete.
