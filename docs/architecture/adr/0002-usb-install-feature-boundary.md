# ADR 0002: Keep USB installation outside the normal CDC path

## Problem

The optional USB cartridge-ingress callback was added to the shared USB service
contract. As a result, the normal F405 development composition changed its
interrupt callback path even though it did not enable `usb-install`. The F405
baseline then regressed during boot and storage loading.

## Decision

Keep the pre-installer, log-only CDC callback contract when `usb-install` is
disabled. Compile the two-channel callback and installer receive path only
when `usb-install` is enabled. The installer remains an opt-in capability and
does not change the default F405 development firmware.

## Rejected alternative

Passing a no-op installer callback through the normal CDC path was rejected
because it changes the established runtime path and leaves optional behavior
inside the default composition.

## Compatibility and validation

The `f405-mvp` profile must preserve the accepted F405 boot, SDIO, AMRN
validation, and application-launch trace. The `f405-development` profile may
add experimental Flash-artifact capabilities and is not the MVP recovery
profile. The separate `usb-install` profile must continue to compile and retain
its dedicated USB interface. Hardware acceptance requires a fresh F405 flash
and a console trace through AMRN execution.

## Rollback

Revert this ADR and the associated callback-boundary change as one atomic
change, then restore the known-good F405 recovery image if hardware validation
fails.
