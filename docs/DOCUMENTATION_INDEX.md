# Documentation Index

The documentation is written in English so that the project can be understood and adopted internationally. Georgian cultural references are preserved in the project identity and explained where relevant.

The contribution workflow is documented in the repository root [CONTRIBUTING.md](../CONTRIBUTING.md).

The repository coding-agent contract is documented in the root [AGENTS.md](../AGENTS.md).

Read the documents in this order:

1. [Architecture](architecture/README.md) — categorized project vision, MVP boundary, runtime layers, and core contracts.
2. [File Structure](file-structure/README.md) — repository layout and prioritized file sequence.
3. [Roadmap](ROADMAP.md) — phase gateway, status, and acceptance boundaries;
   detailed phase plans are in [docs/roadmap](roadmap/).
4. [Coding Standards](coding-standards/README.md) — mandatory code, comment, safety, and review rules.
5. [AMRN Format](amrn-format/README.md) — binary package layout and validation rules.
6. [ABI](abi/README.md) — kernel-to-application execution contract.
7. [Hardware](HARDWARE.md) — STM32F405 MVP board, pins, clock, SD wiring, and electrical assumptions.
8. [Development](development/README.md) — build, flash, logging, and debugging workflow.
9. [Application Workflow](application-workflow/README.md) — build, package, install, and validate an application.
10. [Testing](testing/README.md) — categorized host, target, and hardware testing documentation.
11. [MVP Acceptance](mvp-acceptance/README.md) — the physical STM32F405 end-to-end acceptance procedure.
12. [Versioning](versioning/README.md) — component versions, ABI compatibility, and release tags.
13. [Security](SECURITY.md) — current guarantees, non-guarantees, and post-MVP security work.
14. [Application Relocation](relocation/README.md) — movable ABI v3 application design and evidence boundaries.
15. [Package Distribution](package-distribution/README.md) — normative multi-developer trust, metadata, update, and acceptance contract.
16. [Ustari Protocol](ustari-protocol/README.md) — the planned bounded command, telemetry, session-security, and package-delivery protocol.
17. [CLI documentation](cli/README.md) — installation, commands, workflows, output, errors, and testing.
18. [Target Profiles](TARGET_PROFILES.md) — declarative board metadata and kernel mapping ownership.
19. [Target Manifest Reference](target-manifest/README.md) — complete TOML schema, field rules, and F405 example.
20. [Platform Backend Contract](PLATFORM_BACKENDS.md) — ownership boundaries and the workflow for adding a target backend.
21. [Driver Architecture](drivers/README.md) — no-heap, bounded, hardware-neutral driver principles.
    - [GPIO and EXTI](drivers/gpio.md) — pin modes, level access, and interrupt polling.
    - [Timers](drivers/timer.md) — countdown, timeout, and SysTick policy.
    - [UART and SPI](drivers/serial.md) — bounded serial contracts and buffer rules.

The categorized documentation indexes and their topic files are the source of truth for their respective areas.
