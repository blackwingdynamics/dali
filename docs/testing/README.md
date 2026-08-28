# Testing Documentation

This directory contains the testing strategy, validation procedures, and
recorded evidence previously maintained in the former top-level testing guide.

This file is the testing documentation entry point and navigation index. The
documents below preserve the original testing material by
category:

Board-specific wiring and evidence context: [STM32F405 board documentation](../boards/stm32f405/README.md).

1. [Host tests](host.md) — hardware-neutral contracts, parser tests, and host
   validation commands.
2. [Target tests](target.md) — embedded target checks and their limits.
3. [F405 Silicon evidence](f405-silicon.md) — recorded board traces and
   target-specific acceptance records.
4. [Evidence boundary](evidence-boundary.md) — evidenced and unevidenced
   behavior, including the distinction between host tests and hardware proof.
5. [Signed package acceptance](signed-packages.md) — repository, trust-store,
   and signed-package validation records.
6. [MVP acceptance](mvp-acceptance.md) — the baseline end-to-end acceptance
   procedure.
7. [Requirements and evidence traceability](requirements-and-evidence.md) —
   implementation, validation-layer, and Silicon Trace mapping.

The extracted files are documentation reorganizations only. They must not
change the meaning of a test, acceptance claim, or hardware evidence record.
