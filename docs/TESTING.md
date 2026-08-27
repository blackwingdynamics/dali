# Testing Strategy

This file is the stable entry point for Dali OS testing documentation. The
full testing procedures and evidence records are organized by category under
[`docs/testing/`](testing/).

## Navigation

- [Host tests](testing/host.md) — hardware-neutral contracts, parser tests,
  and host validation commands.
- [Target tests](testing/target.md) — embedded target checks and limitations.
- [F405 Silicon evidence](testing/f405-silicon.md) — board traces and
  target-specific acceptance records.
- [Evidence boundary](testing/evidence-boundary.md) — what is and is not
  hardware evidence.
- [Signed package acceptance](testing/signed-packages.md) — repository,
  trust-store, and signed-package validation records.
- [MVP acceptance](testing/mvp-acceptance.md) — baseline end-to-end acceptance
  procedure.

## Reading order

1. Read [host tests](testing/host.md) for contract and parser validation.
2. Read [target tests](testing/target.md) for embedded build validation.
3. Read the [evidence boundary](testing/evidence-boundary.md) before
   interpreting any result as hardware proof.
4. Read [F405 Silicon evidence](testing/f405-silicon.md) for the actual board
   traces and acceptance records.
5. Use [MVP acceptance](testing/mvp-acceptance.md) for the complete baseline
   procedure.

The extracted documents preserve the testing material previously contained in
this file. Hardware-dependent claims remain limited to the recorded evidence;
host tests and compilation do not substitute for Silicon Trace validation.
