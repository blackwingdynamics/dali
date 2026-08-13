# Dali OS Versioning

Dali OS has several versioned contracts. They must not be treated as one number because a kernel release, an AMRN format revision, and an application ABI change have different compatibility consequences.

## 1. Versioned components

| Component | Versioning scheme | Purpose |
| --- | --- | --- |
| Kernel | Semantic Versioning | Kernel runtime and service compatibility |
| `dali-sdk` | Semantic Versioning | Application developer API |
| `dali-cli` | Semantic Versioning | Package and device tooling |
| `.amrn` format | Integer format version | Binary package parsing rules |
| Application ABI | Integer ABI version | Kernel-to-application entry contract |
| Application package | Semantic Versioning | Application release identity |

## 2. Semantic Versioning

Kernel, SDK, CLI, and application packages follow:

```text
MAJOR.MINOR.PATCH
```

- `MAJOR` changes indicate incompatible public behavior or contracts;
- `MINOR` changes add backward-compatible functionality;
- `PATCH` changes fix behavior without changing the public contract.

Before `1.0.0`, the project is allowed to make breaking changes in minor releases, but every breaking change must be documented and reflected in the roadmap.

Pre-release versions use identifiers such as:

```text
0.1.0-alpha.1
0.1.0-beta.1
1.0.0-rc.1
```

## 3. AMRN format version

The `.amrn` header contains its own `format_version`. The loader must reject a format version it does not understand.

An AMRN format version must change when any of the following changes:

- header field meaning or byte order;
- header size or field offsets;
- payload interpretation;
- checksum interpretation;
- load or entry-point rules;
- required validation semantics.

The current MVP format is:

```text
AMRN format version: 1
Header size: 32 bytes
Integrity: CRC32
Load address: 0x20008000
Target ID: 0x01 (STM32F411CEU6)
ABI version: 1
```

Format version `1` must remain readable by every kernel that claims support for it.

## 4. Application ABI version

The application ABI is independent from the AMRN container format. A package may have a valid header and checksum while still being incompatible with a kernel ABI.

The MVP ABI is:

```text
ABI version: 1
Entry: unsafe extern "C" fn() -> !
Target: thumbv7em-none-eabihf
Interrupt ownership: kernel-controlled
```

The ABI version must change when entry semantics, calling convention, memory ownership, interrupt ownership, lifecycle behavior, or shared data structures change.

The package format carries the ABI version at header offset `0x18`.

## 5. Compatibility rules

The loader must validate at least:

- target architecture and MCU target;
- AMRN format version;
- application ABI version;
- load address and memory limits;
- application payload size;
- package integrity.

Compatibility is explicit. A package must not be loaded merely because its name or semantic version appears valid.

Future compatibility metadata may include:

```text
minimum_kernel_version
maximum_kernel_version
required_abi_version
required_services
memory_requirements
```

## 6. Release tags

Release tags use the component name and semantic version:

```text
kernel-v0.1.0
sdk-v0.1.0
cli-v0.1.0
amrn-format-v1
```

The current monorepo release workflow uses tags in the form `vX.Y.Z`. Component-specific tags may be introduced after the repository is split.

For a monorepo release, the release notes must clearly state which components changed and which contracts remain compatible.

## 7. Breaking changes

A breaking change requires:

1. an update to the affected specification;
2. an update to the compatibility rules;
3. migration notes;
4. tests for accepted and rejected versions;
5. a roadmap task or release note;
6. an explicit version increment.

Do not silently change the meaning of an existing AMRN field, ABI rule, memory address, or public SDK item.

## 8. Release checklist

Before publishing a release:

- [ ] The version is consistent in all relevant manifests.
- [ ] AMRN and ABI versions are recorded where applicable.
- [ ] Compatibility tests pass.
- [ ] Documentation reflects the release behavior.
- [ ] Breaking changes have migration notes.
- [ ] Release notes identify kernel, SDK, CLI, and format changes.
- [ ] The release tag follows the documented format.
- [ ] CI passes on the release commit.
