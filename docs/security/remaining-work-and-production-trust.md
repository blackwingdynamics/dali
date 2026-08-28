# Remaining security work

- pre-reset ROM/first-stage bootloader verification for the kernel image;
- production key custody and debug-lock enforcement;
- broader MPU-backed isolation and multi-application ownership where supported;
- arbitrary DMA-controller/peripheral isolation;
- cross-target watchdog failure semantics and timeout evidence;
- power-loss-safe update and rollback acceptance;
- security review of package parsing and storage access.

Security claims must be added only after the corresponding mechanism and test evidence exist.

The future multi-developer package ecosystem is defined in
`docs/package-distribution/README.md`. That document is the source of truth for the
Dali root, repository metadata roles, developer delegations, signed trust-store
updates, revocation, rotation, rollback, and offline installation. The current
F405 static release trust-anchor path is a precursor to that design and must
not be described as a completed multi-developer registry.

## Production trust contract slice

The hardware-neutral `dali-metadata` policy now defines production root-key
custody requirements: at least three distinct root public keys, a two-of-three
root threshold, and no private key material on the target. It also defines the
`dali.secure-boot.v1` kernel-image descriptor and admission checks for target,
version, length, SHA-256, and root-role signatures.

This is a policy and codec boundary only for the kernel image. The current F405
startup path does not yet verify a signed kernel image before reset-vector
execution, so pre-reset Secure Boot and production root custody remain
unaccepted hardware claims. Separately, the F405 repository boot path verifies
the signed `bundle.manifest` through the Root-declared Bundle role before
applying its generation admission policy. Existing trust-store verification
continues to apply rotation overlap, signed revocation state, metadata expiry
when trusted time is available, and durable generation rollback checks.
