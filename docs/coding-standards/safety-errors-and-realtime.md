# 7. `unsafe` policy

`unsafe` is forbidden by default and may be used only where a safe abstraction cannot express the hardware or execution contract.

Allowed MVP use cases are:

- memory-mapped register access;
- copying validated bytes into the reserved application SRAM region;
- transferring control through the validated application entry point;
- interrupt or vector-table operations when explicitly specified;
- unavoidable low-level HAL operations.

Every unsafe operation must:

- keep the unsafe block as small as possible;
- validate inputs before entering the unsafe block;
- add an immediate `SAFETY` comment;
- document the invariant that makes the operation valid;
- wrap repeated unsafe logic in a small safe abstraction;
- avoid exposing raw pointers across module boundaries.

Example:

```rust
// SAFETY: The loader validated that `destination` is inside the reserved
// application SRAM region and that `length` does not exceed its capacity.
unsafe {
    core::ptr::copy_nonoverlapping(source, destination, length);
}
```

Native application code is trusted and is not sandboxed or fault-isolated in the MVP.

## 8. Error handling

Runtime code must not silently discard meaningful failures.

The following are forbidden in kernel runtime paths unless a boot-time invariant is explicitly documented and reviewed:

- `unwrap()`;
- `expect()`;
- `panic!()`;
- `unreachable!()`;
- ignored `Result` values;
- converting meaningful errors into an unexplained `bool` or `None`.

Use typed error enums with enough information to identify the failed operation. Preserve the original cause where practical. Malformed SD data and AMRN cartridges must be rejected gracefully.

## 9. Embedded and real-time rules

- The kernel remains `#![no_std]`.
- Heap allocation is forbidden in safety-critical or real-time paths.
- Use fixed-size buffers, queues, and collections where bounded behavior is required.
- Do not place blocking SD operations in a future safety-critical control task.
- Every buffer has an explicit maximum size.
- DMA buffers have documented alignment, lifetime, and ownership rules.
- Integer overflow behavior must be intentional.
- Memory addresses, pin assignments, and peripheral constants must be named and centralized.
- Magic numbers are forbidden in implementation code.
- Timing assumptions must be documented and testable.
- Logging must not make a critical path unbounded or block indefinitely.

The MVP application has no scheduler or application-owned interrupts. Its only
service is the bounded logging ABI defined in `docs/abi/README.md`.
