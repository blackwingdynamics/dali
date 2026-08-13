# Kernel–Application ABI

## MVP status

The MVP uses a deliberately small native entry contract. It is not a stable public SDK ABI yet.

```rust
pub type EntryPoint = unsafe extern "C" fn() -> !;
```

The exact ABI must be identical in the kernel and demo application. The application is linked for `0x20008000` and its complete image is copied to the reserved SRAM region before the jump.

## Rules

- target: `thumbv7em-none-eabihf`;
- architecture: ARM Cortex-M4F;
- entry address: `load_address + execution_offset`, with the Thumb bit set;
- application return: forbidden in the MVP;
- panic behavior: the application must not rely on a kernel panic handler;
- direct register access: allowed for the demo application only;
- kernel-private symbols: not available to applications;
- interrupts: disabled and not owned by applications in the MVP;
- shared memory: not available until an explicit layout is documented.

The first application proves execution with a deterministic LED pattern. Shared RTT logging is intentionally deferred until a logging ABI exists.

## Safety boundary

Calling the entry point is `unsafe` because the kernel cannot prove that the loaded native code obeys the ABI. A malformed or incompatible application may corrupt kernel state or stop execution. The MVP therefore provides package validation and integrity checking, but not sandboxing or fault isolation.

## Future ABI work

Before `dali-sdk` is published, specify application context, logging, task creation, service calls, shutdown, health reporting, capability handles, version compatibility, and error representation.
