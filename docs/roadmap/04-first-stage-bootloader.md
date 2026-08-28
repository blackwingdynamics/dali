# First-Stage Bootloader

Status: **Future**.

The current F405 startup image begins at the application flash boundary. The
existing `dali.secure-boot.v1` contract is a hardware-neutral policy and codec;
it is not yet enforced before the kernel reset vector executes.

## Backlog

- Define the ROM/FSBL handoff ABI and immutable verification boundary.
- Verify the kernel image target, version, length, SHA-256, and root-role
  signatures before transferring control to its reset vector.
- Bind image-version admission to a durable anti-rollback counter.
- Define recovery behavior for invalid, expired, revoked, or interrupted
  images without entering an unbounded reset loop.
- Specify production root-key custody, rotation, revocation, debug lock, and
  provisioning procedures without placing private keys on the target.
- Add F405 hardware evidence for valid, rejected, and rollback image flows.
- Define atomic update and power-loss recovery semantics.

## Explicit non-claims

Repository/package signature verification after reset is not pre-reset Secure
Boot. The current F405 release path therefore remains outside the FSBL/ROM
acceptance claim until this document's evidence gate is met.
