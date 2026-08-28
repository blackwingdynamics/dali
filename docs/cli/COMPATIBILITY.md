# Dali CLI Compatibility

## Versioned contracts

CLI compatibility depends on more than the executable version:

- CLI semantic version;
- AMRN format version;
- target identifier;
- application ABI version;
- kernel and application compatibility rules.

The AMRN format and ABI specifications are the source of truth:

- AMRN format: ../amrn-format/README.md
- ABI: ../ABI.md
- Versioning: ../versioning/README.md

## Current MVP profile

The current package profile is AMRN format version 1, STM32F405 target
identifier 0x02, and ABI version 2. These values come from the shared
dali-amrn contract and must not be independently redefined by the CLI.

`dali inspect` can also validate the host-side AMRN format version 2 contract
for a target that declares isolation memory metadata. This is inspection-only:
the CLI does not build v3 packages, and the kernel does not execute them yet.

Build and package commands enforce the target manifest capabilities before
constructing an application package. Isolation ABI builds require declared MPU
support, while AMRN relocation format builds require declared relocation
support.

## Breaking changes

Changes to package bytes, validation rules, output semantics, or exit codes
must update the relevant contract documentation, tests, roadmap evidence, and
versioning decision together.
