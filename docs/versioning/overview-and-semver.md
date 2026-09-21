# Versioning Overview and SemVer

Dali OS has several versioned contracts. They must not be treated as one number because a kernel release, an AMRN format revision, and an application ABI change have different compatibility consequences.

## 1. Versioned components

| Component | Versioning scheme | Purpose |
| --- | --- | --- |
| Kernel | Semantic Versioning | Kernel runtime and service compatibility |
| `dali` | Semantic Versioning | Application developer API |
| `dali-cli` | Semantic Versioning | Cartridge and device tooling |
| `.amrn` format | Integer format version | Binary cartridge parsing rules |
| Metadata wire format | Integer format version | Canonical repository metadata encoding |
| Application ABI | Integer ABI version | Kernel-to-application entry contract |
| Application cartridge | Semantic Versioning | Application release identity |

## 2. Semantic Versioning

Kernel, SDK, CLI, and application cartridges follow:

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

The first Dali OS MVP release used the pre-release version `0.1.0-alpha.1`.
The current workspace release is `0.1.0-alpha.4`. It records the accepted
F405 MVP foundation; production hardening and post-MVP platform capabilities
remain incomplete.
GitHub Releases for `alpha`, `beta`, and `rc` versions must be marked as
pre-releases automatically by the release workflow.
