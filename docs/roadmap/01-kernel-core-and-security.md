# Kernel Core and Security

Status: **Completed for the feature-gated STM32F405 scope**.

This document records what is implemented and evidenced. It does not expand
the baseline ABI v2 MVP into a general-purpose secure kernel.

## Completed contracts

- `no_std` F405 bootstrap, clock, LED, USB CDC, and SDIO block access.
- Bounded AMRN parsing, payload validation, CRC32 checking, and native entry.
- Feature-gated ABI v3 F405 context switching with PSP ownership,
  SysTick/PendSV handoff, MPU region switching, SVC dispatch, and kernel-owned
  fault recovery for declared application contexts.
- Faulted-context retirement and resumption of a ready context on F405.
- Bounded kernel-owned F405 SDIO DMA policy, validated buffer ownership, and
  application DMA request rejection.
- AMRN v5 signed package/repository verification, developer authorization,
  trust-store rotation/revocation policy, expiry handling, and durable
  generation anti-rollback admission in the configured repository path. F405
  hardware evidence covers signed package verification and anti-rollback;
  target-side revoked-developer-key acceptance remains a separate test.
- Watchdog reset-cause detection, one-shot feed-failure verification, and
  stable Safe Mode heartbeat recovery on F405.
- Storage lifecycle recovery for unavailable, removed, faulted, and
  reinitialized cards with bounded retry behavior.
- Release CI matrix and measured F405 memory/linker-map reporting.

## Evidence boundary

The F405 evidence and commands are maintained in
[`docs/testing/README.md`](../testing/README.md). Host tests validate hardware-neutral
contracts; they are not hardware evidence.

The completed claims are intentionally bounded. They do not claim arbitrary
DMA-controller isolation, confidentiality, production multi-application
lifecycle isolation, cross-target portability, or pre-reset kernel-image
Secure Boot. The `dali.secure-boot.v1` image policy and codec are implemented
and host-tested, but FSBL/ROM enforcement is backlog.

The baseline ABI v2 remains trusted native code. The isolation and repository
security paths require their documented feature combinations and target
configuration.

## Future Microkernel IPC backlog

- [ ] Approve and implement the staged Ustari message boundary between AMRN
  applications and the kernel SVC gateway.
- [ ] Define MPU-protected, fixed-capacity request/response rings with explicit
  ownership, publication ordering, stale-slot invalidation, and reset recovery.
- [ ] Add hardware-neutral host tests before introducing a target IPC runtime.

See [`RFC: Ustari-Based AMRN IPC Messaging`](../architecture/rfc-ustari-amrn-ipc.md)
for the proposed message, zero-copy, security, and acceptance model.

## Validation gate

- Host workspace checks and strict host Clippy pass when the no-std kernel
  binary is excluded from the host test harness.
- Kernel library tests pass separately.
- F405 release build and strict target Clippy pass.
- Formatting and `git diff --check` pass.
