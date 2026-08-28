# Documentation Index

The documentation is written in English so that the project can be understood and adopted internationally. Georgian cultural references are preserved in the project identity and explained where relevant.

The contribution workflow is documented in the repository root [CONTRIBUTING.md](../CONTRIBUTING.md).

The repository coding-agent contract is documented in the root [AGENTS.md](../AGENTS.md).

Read the documents in this order:

1. [Architecture](architecture/README.md) — categorized project vision, MVP boundary, runtime layers, and core contracts.
2. [File Structure](file-structure/README.md) — repository layout and prioritized file sequence.
3. [Roadmap](roadmap/README.md) — phase gateway, status, and acceptance boundaries.
4. [Coding Standards](coding-standards/README.md) — mandatory code, comment, safety, and review rules.
5. [AMRN Format](amrn-format/README.md) — binary cartridge layout and validation rules.
6. [ABI](abi/README.md) — kernel-to-application execution contract.
7. [Hardware](hardware/README.md) — STM32F405 MVP board, pins, clock, SD wiring, and electrical assumptions.
8. [Development](development/README.md) — build, flash, logging, and debugging workflow.
9. [Application Workflow](application-workflow/README.md) — build, package, install, and validate an application.
10. [Testing](testing/README.md) — categorized host, target, and hardware testing documentation.
11. [MVP Acceptance](mvp-acceptance/README.md) — the physical STM32F405 end-to-end acceptance procedure.
12. [Versioning](versioning/README.md) — component versions, ABI compatibility, and release tags.
13. [Security](security/README.md) — current guarantees, non-guarantees, and post-MVP security work.
14. [Application Relocation](relocation/README.md) — movable ABI v3 application design and evidence boundaries.
15. [Package Distribution](package-distribution/README.md) — normative multi-developer trust, metadata, update, and acceptance contract.
16. [Ustari Protocol](ustari-protocol/README.md) — the planned bounded command, telemetry, session-security, and package-delivery protocol.
17. [CLI documentation](cli/README.md) — installation, commands, workflows, output, errors, and testing.
18. [Target Profiles](target-profiles/README.md) — declarative board metadata and kernel mapping ownership.
19. [Target Manifest Reference](target-manifest/README.md) — complete TOML schema, field rules, and F405 example.
20. [Platform Backend Contract](platform-backends/README.md) — ownership boundaries and the workflow for adding a target backend.
21. [Driver Architecture](drivers/README.md) — no-heap, bounded, hardware-neutral driver principles.
    - [GPIO and EXTI](drivers/gpio.md) — pin modes, level access, and interrupt polling.
    - [Timers](drivers/timer.md) — countdown, timeout, and SysTick policy.
    - [UART and SPI](drivers/serial.md) — bounded serial contracts and buffer rules.

The categorized documentation indexes and their topic files are the source of truth for their respective areas.

## Documentation ownership

Each domain has one canonical index and one primary source of truth. Topic
documents inherit the owner and scope of their domain unless they explicitly
reference a more authoritative contract.

| Domain | Owner | Canonical source |
| --- | --- | --- |
| Architecture | Kernel architecture | [Architecture index](architecture/README.md) |
| ABI | Kernel/application ABI | [ABI index](abi/README.md) |
| AMRN format | AMRN format contract | [AMRN format index](amrn-format/README.md) |
| Application workflow | Application delivery workflow | [Application workflow index](application-workflow/README.md) |
| CLI | Host CLI behavior | [CLI index](cli/README.md) |
| Coding standards | Repository engineering policy | [Coding standards index](coding-standards/README.md) |
| Development | Developer setup and operations | [Development index](development/README.md) |
| Drivers | Hardware-neutral driver contracts | [Driver index](drivers/README.md) |
| File structure | Repository organization | [File structure index](file-structure/README.md) |
| Hardware | Board and electrical constraints | [Hardware index](hardware/README.md) |
| Metadata binary v2 | Repository metadata wire format | [Metadata index](metadata-binary-v2/README.md) |
| MVP acceptance | End-to-end acceptance evidence | [MVP acceptance index](mvp-acceptance/README.md) |
| Package distribution | Trust, update, and distribution policy | [Package distribution index](package-distribution/README.md) |
| Platform backends | Target-specific hardware boundary | [Platform backend index](platform-backends/README.md) |
| Relocation | Movable application design | [Relocation index](relocation/README.md) |
| Roadmap | Project phases and execution order | [Roadmap index](roadmap/README.md) |
| Security | Security guarantees and limitations | [Security index](security/README.md) |
| Target manifest | TOML schema and field rules | [Target manifest index](target-manifest/README.md) |
| Target profiles | Generated profile ownership | [Target profile index](target-profiles/README.md) |
| Testing | Validation strategy and evidence | [Testing index](testing/README.md) |
| Ustari protocol | Future protocol contract | [Ustari index](ustari-protocol/README.md) |
| Versioning | Compatibility and release rules | [Versioning index](versioning/README.md) |

The implementation source, target manifests, generated profiles, and recorded
Silicon Trace remain authoritative for their respective technical facts. This
matrix defines documentation ownership; it does not replace those sources.
