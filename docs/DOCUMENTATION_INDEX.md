# Documentation Index

The documentation is written in English so that the project can be understood and adopted internationally. Georgian cultural references are preserved in the project identity and explained where relevant.

The contribution workflow is documented in the repository root [CONTRIBUTING.md](../CONTRIBUTING.md).

The repository coding-agent contract is documented in the root [AGENTS.md](../AGENTS.md).

Read the documents in this order:

1. [Architecture](ARCHITECTURE.md) — index for categorized project vision, MVP boundary, runtime layers, and core contracts.
2. [File Structure](FILE_STRUCTURE.md) — repository layout and prioritized file sequence.
3. [Roadmap](ROADMAP.md) — phase gateway, status, and acceptance boundaries;
   detailed phase plans are in [docs/roadmap](roadmap/).
4. [Coding Standards](CODING_STANDARDS.md) — mandatory code, comment, safety, and review rules.
5. [AMRN Format](AMRN_FORMAT.md) — binary package layout and validation rules.
6. [ABI](ABI.md) — kernel-to-application execution contract.
7. [Hardware](HARDWARE.md) — STM32F405 MVP board, pins, clock, SD wiring, and electrical assumptions.
8. [Development](DEVELOPMENT.md) — build, flash, logging, and debugging workflow.
9. [Application Workflow](APPLICATION_WORKFLOW.md) — build, package, install, and validate an application.
10. [Testing](TESTING.md) — navigation index for categorized host, target, and hardware testing documentation.
11. [MVP Acceptance](MVP_ACCEPTANCE.md) — the physical STM32F405 end-to-end acceptance procedure.
12. [Versioning](VERSIONING.md) — component versions, ABI compatibility, and release tags.
13. [Security](SECURITY.md) — current guarantees, non-guarantees, and post-MVP security work.
14. [Package Distribution](PACKAGE_DISTRIBUTION.md) — the normative multi-developer trust, metadata, update, and acceptance contract.
15. [Ustari Protocol](USTARI_PROTOCOL.md) — the planned bounded command, telemetry, session-security, and package-delivery protocol.
16. [CLI documentation](cli/README.md) — installation, commands, workflows, output, errors, and testing.
17. [Target Profiles](TARGET_PROFILES.md) — declarative board metadata and kernel mapping ownership.
18. [Target Manifest Reference](TARGET_MANIFEST.md) — complete TOML schema, field rules, and F405 example.
19. [Platform Backend Contract](PLATFORM_BACKENDS.md) — ownership boundaries and the workflow for adding a target backend.
20. [Driver Architecture](drivers/README.md) — no-heap, bounded, hardware-neutral driver principles.
    - [GPIO and EXTI](drivers/gpio.md) — pin modes, level access, and interrupt polling.
    - [Timers](drivers/timer.md) — countdown, timeout, and SysTick policy.
    - [UART and SPI](drivers/serial.md) — bounded serial contracts and buffer rules.

The architecture and MVP documents are the source of truth until the more detailed specifications are created.
