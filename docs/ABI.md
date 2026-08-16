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

## Planned ABI v3 boundary

ABI v3 is a design contract only. ABI v2 remains the active MVP ABI until the
implementation and F405 hardware evidence are complete.

The repository now contains a feature-gated SVC frame validator and bounded log
dispatcher behind the kernel `abi-v3` feature. This is an implementation
scaffold for testing only; it does not enable MPU protection, privilege
transition, or ABI v3 package execution by default.

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

This budget is a design target, not an enabled configuration. The privileged
background map and default-memory attributes must be selected so that an
unprivileged access cannot bypass the no-access boundaries.

Kernel SRAM, kernel runtime/stack SRAM, and ordinary peripheral registers are
not application-accessible. The v3 design does not claim DMA isolation,
confidentiality of readable Flash, package authenticity, or recovery from
arbitrary native faults.

### Fault boundary

MemManage, BusFault, UsageFault, and invalid exception-return paths must enter
a privileged kernel fault boundary. The boundary records a bounded fault
record, marks the current application terminated, and returns to a kernel-owned
control path. It must not unwind or reuse an application PSP as a kernel stack.
The exact recovery assembly and fault record layout remain implementation work.

The feature-gated kernel now defines a bounded `FaultRecord` and diagnostic
MemManage, BusFault, and UsageFault handlers. These handlers report the SCB
status and halt the diagnostic path; they do not yet recover an application or
return to a scheduler. They do not change ABI v2 behavior.
