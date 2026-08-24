# Dali OS Versioning

Dali OS has several versioned contracts. They must not be treated as one number because a kernel release, an AMRN format revision, and an application ABI change have different compatibility consequences.

## 1. Versioned components

| Component | Versioning scheme | Purpose |
| --- | --- | --- |
| Kernel | Semantic Versioning | Kernel runtime and service compatibility |
| `dali` | Semantic Versioning | Application developer API |
| `dali-cli` | Semantic Versioning | Package and device tooling |
| `.amrn` format | Integer format version | Binary package parsing rules |
| Metadata wire format | Integer format version | Canonical repository metadata encoding |
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

The first Dali OS MVP release uses the pre-release version
`0.1.0-alpha.1` because the F405 MVP path is accepted, while production
hardening and post-MVP platform capabilities are not complete.
GitHub Releases for `alpha`, `beta`, and `rc` versions must be marked as
pre-releases automatically by the release workflow.

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
Target ID: 0x02 (STM32F405RGT6)
ABI version: 2
```

The current AMRN v1 target profile is STM32F405RGT6 (`0x02`). The STM32F411
BlackPill manifest is generator-only and does not authorize native application
execution.

Format version `1` must remain readable by every kernel that claims support for it.

## 4. Application ABI version

The application ABI is independent from the AMRN container format. A package may have a valid header and checksum while still being incompatible with a kernel ABI.

The MVP ABI is:

```text
ABI version: 2
Entry: unsafe extern "C" fn(*const ServiceTable) -> !
Target: thumbv7em-none-eabihf
Interrupt ownership: kernel-controlled
```

The ABI version must change when entry semantics, calling convention, memory ownership, interrupt ownership, lifecycle behavior, service-table layout, or shared data structures change.

The MPU and unprivileged execution boundary change entry semantics, service
dispatch, memory ownership, and fault handling. It therefore requires a new
ABI version and an AMRN compatibility decision; it must not be shipped as an
ABI v2-compatible implementation.

The package format carries the ABI version at header offset `0x18`.

The feature-gated ABI v3 uses an SVC service gateway, unprivileged Thread mode,
a PSP-backed application stack, and separate application code/data regions. It
is not compatible with ABI v2's direct service-table function pointer or its
single-region linker contract. ABI v3 packages must therefore be rejected by
ABI v2 kernels. The F405 implementation and hardware evidence are scoped to
the experimental feature-gated path; ABI v2 remains the default.

The ABI v3 package contract uses AMRN format version `2`. Format version `2`
has a fixed 64-byte header, separate code and initialized-data file segments,
explicit zero-data and PSP stack reservations, target-defined code/data
origins, and a CRC32 over both file segments. The current F405 linker contract
uses 32 KiB code and 32 KiB data regions. This contract is documented in
`docs/AMRN_FORMAT.md`; it does not authorize the current v1 parser or builder
to accept or emit ABI v3 packages.

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

Target profiles permanently declare package authentication policy for both
development and release builds. The current F405 policy permits unsigned
development packages for local bring-up and requires Ed25519 for release
packages. Release manifests must select AMRN format `5`, declare a
`signing_key_id`, and provide the private seed through the external
`DALI_SIGNING_KEY_HEX` environment variable. Target manifests provision
verification public keys through `authentication.development_trust_anchors`
and `authentication.release_trust_anchors`; private key material must never be
placed in a target manifest or firmware source. The reference F405 development
anchor is the RFC8032 test vector and is enabled only by `abi-test-fixtures`;
the release manifest contains a generated public anchor and has hardware
verification evidence. This static target-profile mechanism is the precursor
to the multi-developer repository trust contract in
`docs/PACKAGE_DISTRIBUTION.md`; it is not that contract's dynamic trust store.

AMRN format version `4` defines package identity and selection metadata. It
remains ABI v3-compatible: format v4 changes the container header and
compatibility checks, not the application calling convention, service gateway,
or MPU contract. The current feature-gated F405 loader supports format v4 for
one selected package; it does not yet provide multi-package execution.

AMRN format version `5` is the signed successor to format v4. It leaves the
v4 bytes unchanged, signs the fixed header and payload, and appends a DSIG
trailer. Format v5 is a host/CLI contract only until target-side verification
is implemented. The crypto crate now rejects unknown key identifiers before
verification; this still must not be described as Secure Boot until the target
trust store is provisioned and the kernel loader enforces it.

## 6. Release tags

Release tags use the component name and semantic version:

```text
kernel-v0.1.0
sdk-v0.1.0
cli-v0.1.0
amrn-format-v1
```

The current monorepo release workflow uses tags in the form `vX.Y.Z` or
`vX.Y.Z-alpha.N`. Component-specific tags may be introduced after the
repository is split.

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

- [x] The version is consistent in all relevant manifests.
- [x] AMRN and ABI versions are recorded where applicable.
- [x] Compatibility tests pass.
- [x] Documentation reflects the release behavior.
- [x] Breaking changes have migration notes.
- [x] Release notes identify kernel, SDK, CLI, and format changes.
- [x] The release tag follows the documented format.
- [x] CI passes on the release commit.
