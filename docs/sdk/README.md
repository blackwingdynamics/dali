# Dali SDK Documentation

The Dali SDK is the hardware-neutral application-facing crate at
[`crates/dali-sdk`](../../crates/dali-sdk/). It exposes the application entry
service boundary and bounded kernel logging wrapper. Applications do not
access board registers, storage internals, or kernel-private symbols through
the SDK.

## Contract

The current default application contract provides:

- `ServiceTable`, passed to the native application entry point;
- the kernel-owned `log` service;
- bounded UTF-8 log messages;
- boolean success or rejection reporting from the SDK wrapper.

The ABI entry shape and lifecycle rules are defined by the [ABI
documentation](../abi/README.md). The SDK does not redefine the ABI, AMRN
cartridge format, target metadata, or memory layout.

## Service boundary

The SDK forwards application requests through the declared kernel service
boundary. The kernel owns validation, logging transport, memory policy, and
hardware access. Applications must not assume that USB CDC, RTT, SDIO, or an
OLED display is available merely because the logging service accepts a
message.

The current SDK surface is intentionally small. Additional services require a
versioned ABI decision, documentation, host coverage, target validation, and
recorded hardware evidence where physical behavior is involved.

## Compatibility

SDK compatibility is jointly constrained by the SDK version, application ABI
version, AMRN cartridge format, and selected target capabilities. Use the
[versioning documentation](../versioning/README.md) for breaking-change
policy and the [application project contract](../cli/APPLICATION_PROJECT.md)
for generated project configuration.

## Validation

Run the SDK host tests from the repository root:

```text
cargo test -p dali-sdk
```

These tests validate the hardware-neutral service wrapper. They are not
evidence that a cartridge executed on an F405 target or that a physical
transport delivered its logs. Physical claims require the procedures and
records in [MVP acceptance](../mvp-acceptance/README.md) and [testing
documentation](../testing/README.md).

## Source map

- [`crates/dali-sdk/src/lib.rs`](../../crates/dali-sdk/src/lib.rs) — public SDK facade and default service table.
- [`crates/dali-sdk/src/svc.rs`](../../crates/dali-sdk/src/svc.rs) — versioned service gateway types.
- [`crates/dali-sdk/src/svc_log.rs`](../../crates/dali-sdk/src/svc_log.rs) — feature-gated SVC logging wrapper.
- [Application Workflow](../application-workflow/README.md) — build and package lifecycle.
- [CLI documentation](../cli/README.md) — project generation and cartridge tooling.
