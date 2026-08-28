# Security Scope and Evidence

This document is the navigation point for Dali OS security scope. It states
which claims belong to the baseline, which claims are feature-gated, and what
evidence is required before a mechanism is described as accepted.

## Threat-model scope

Dali OS currently addresses malformed cartridge input, bounded payload
validation, selected processor-access violations on the F405 feature-gated
path, and kernel-owned recovery at the documented boundaries. The threat model
does not assume that native application code is trustworthy merely because its
cartridge was loaded, and it does not treat a host test as hardware evidence.

The baseline ABI v2 path runs native application code in the kernel address
space. It is therefore a trusted-code path, not a sandbox. The feature-gated
ABI v3 path narrows the processor-access boundary for declared F405 contexts,
but it is not a complete production isolation model.

## Guarantees and non-guarantees

The [baseline security boundary](baseline-and-current-boundary.md) defines
the current ABI v2 guarantees and non-guarantees. The [F405 isolation
foundation](f405-isolation-foundation.md) defines the additional feature-gated
processor boundary and its limitations. The [remaining security work](remaining-work-and-production-trust.md)
lists mechanisms that are not yet accepted claims.

In particular, the current documentation does not claim complete sandboxing,
secure boot, production key custody, arbitrary DMA isolation, confidentiality,
complete multi-application policy, or power-loss-safe recovery.

## Evidence rule

A security claim requires all applicable layers to be identified separately:

- host validation for hardware-neutral contracts and codecs;
- embedded validation for target compilation and image inspection;
- Silicon Trace for behavior observed on the real target;
- explicit limitations for scenarios that were not tested.

Compilation, flashing, device enumeration, and a passing host test are not
Silicon Trace. Recorded physical evidence is maintained in the [F405
hardware records](../testing/f405-silicon.md) and the [requirements and
evidence matrix](../testing/requirements-and-evidence.md).

## Review gates

Before adding or strengthening a security claim, update the owning security
document, the relevant contract and tests, the evidence record, and the
roadmap status. Do not infer a guarantee from an adjacent success path, and do
not mark an incomplete hardware scenario as accepted.
