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
