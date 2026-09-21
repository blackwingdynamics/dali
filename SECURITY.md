# Dali OS Security Policy

Dali OS is an early-stage embedded platform. Security-sensitive reports are
welcome. The baseline ABI v2 MVP is trusted native execution and does not
provide sandboxing, memory isolation, secure boot, cartridge authenticity,
encryption, anti-rollback, or application fault isolation. A feature-gated ABI
v3 single-application processor-side isolation path exists, but it is not a
complete sandbox and does not cover DMA or multi-application isolation.

## Reporting a vulnerability

Please do not report suspected vulnerabilities in public issues, pull requests, or discussions.

Use GitHub Private Vulnerability Reporting or a private maintainer channel when one is available. Include:

- affected commit, release, or component;
- hardware and firmware context;
- clear reproduction steps;
- expected and observed behavior;
- potential safety or security impact;
- a minimal proof of concept when it can be shared safely.

Do not include private keys, credentials, production device data, or other secrets in a report.

If private reporting is not available, contact the maintainer privately before making the issue public.

## Scope

Reports are especially important for:

- AMRN header parsing and bounds validation;
- CRC32 and cartridge loading;
- raw pointer or MMIO operations;
- memory layout and entry-point transfer;
- SD-card and filesystem handling;
- CI, release, and cartridge-generation workflows;
- any behavior that could compromise a connected device or actuator.

The project security model and current non-guarantees are documented in [docs/security/README.md](docs/security/README.md).

## Response

Reports will be triaged privately, reproduced where possible, and classified by impact. Fixes, tests, and release notes will be prepared before public disclosure when the issue affects users or devices.
