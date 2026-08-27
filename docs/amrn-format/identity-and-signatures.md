# AMRN format version 4: package identity and selection metadata

Format version `4` extends the relocatable ABI v3 package contract for
multi-package storage. It does not introduce an ABI v4: the package continues
to declare ABI version `3`, and the code/data/relocation semantics remain those
of format version `3`.

The v4 header is fixed at 128 bytes. The first 80 bytes retain the v3 segment,
relocation, target, and ABI fields at the same offsets. The additional fields
are:

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x50` | package_id | 16 bytes | Opaque stable application identity; not derived from a filename |
| `0x60` | package_version_major | 2 bytes | Little-endian semantic package version component |
| `0x62` | package_version_minor | 2 bytes | Little-endian semantic package version component |
| `0x64` | package_version_patch | 2 bytes | Little-endian semantic package version component |
| `0x66` | minimum_kernel_major | 2 bytes | Minimum compatible kernel API version component |
| `0x68` | minimum_kernel_minor | 2 bytes | Minimum compatible kernel API version component |
| `0x6A` | minimum_kernel_patch | 2 bytes | Minimum compatible kernel API version component |
| `0x6C` | required_services | 4 bytes | Manifest-defined service capability bitset |
| `0x70` | slot_id | 1 byte | Explicit target-manifest slot identifier |
| `0x71` | flags | 1 byte | Must be zero until a flag is specified by this document |
| `0x72` | reserved | 2 bytes | Must be zero |
| `0x74` | crc32 | 4 bytes | CRC32 of header bytes after this field, payload, and relocation table |
| `0x78` | reserved | 8 bytes | Must be zero |

The v4 header retains the v3 fields through offset `0x4F`, including the
relocation metadata and ABI version. The v4 `crc32` field is moved to the
extension area so the integrity check covers the complete selection metadata;
the old v3 payload-only checksum rule is unchanged for format version `3`.
The exact CRC input order is the bytes from `0x78` through the end of the
package followed by the bytes from `0x00` through `0x73`, excluding the v4
`crc32` field itself. A builder and loader must use this order exactly.

Selection rules are bounded and explicit:

- `package_id` must be non-zero and must be unique among installed packages;
- package version and minimum kernel version use three unsigned, little-endian
  semantic components and must not overflow their field widths;
- `required_services` must be a subset of services declared by the kernel;
- `slot_id` must name a slot declared by the target manifest;
- target ID, ABI version, relocation contract, and memory ranges must still
  pass the format v3 validation rules;
- a missing identity, incompatible package, duplicate identity, or occupied
  slot is a rejection, never an implicit filename- or directory-order choice;
- format v3 packages remain valid under their existing single-package rules.

The v4 codec, CLI builder/inspection, feature-gated kernel selection/streaming
loader, and host streaming-loader fixtures are implemented without changing the
ABI v2/v3 package paths. A manifest-backed v4 relocation fixture is available
for hardware testing, and the F405 hardware path has accepted that fixture for
selection, relocation, and successful execution. v4-specific rejection and
recovery hardware tests remain open. Format v4 must not be advertised as
multi-application support yet.

Future revisions may add manifest data, kernel compatibility, required services,
memory declarations, encryption metadata, and rollback information. These
require a new format revision or an explicitly versioned extension area.

## Signature envelope contract

The hardware-neutral `dali-amrn::signature` module defines a bounded `DSIG`
envelope for a future versioned AMRN signature extension. The fixed envelope is
88 bytes:

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x00` | magic | 4 bytes | ASCII `DSIG` |
| `0x04` | envelope_version | 1 byte | Value is `1` |
| `0x05` | algorithm | 1 byte | Value `1` means Ed25519 |
| `0x06` | key_id_length | 1 byte | Value is `16` |
| `0x07` | signature_length | 1 byte | Value is `64` |
| `0x08` | key_id | 16 bytes | Opaque trust-anchor identifier |
| `0x18` | signature | 64 bytes | Algorithm-specific signature bytes |

The current parser validates only structure and bounded lengths; it does not
make unsigned v4 packages secure. The v5 loader adds feature-gated signature
verification and static target trust-anchor selection. The complete
multi-developer trust chain, metadata roles, and update policy are defined by
`docs/package-distribution/README.md` and are not implemented by this envelope alone.
The `SignatureVerifier` trait is the hardware-neutral boundary for an audited
host signer and a target trust-store verifier; it does not provide a default
or bypass implementation. The `dali-crypto` facade now provides a bounded
incremental verifier backed by `ed25519-dalek 3.0.0`; its standard-Ed25519
chunked verification is host-tested and thumb-target checked. This backend
selection does not, by itself, enable kernel-side verification or Secure Boot.

### Signed package container (format version 5, feature-gated kernel path)

Format version `5` is the signed successor to the identity-aware relocatable
contract. It does not modify the v4 byte layout. A v5 package uses the v3/v4
image and identity fields in the first 128 bytes, extends the fixed header to
160 bytes, stores the image payload after that header, and appends one fixed
88-byte `DSIG` envelope:

```text
┌──────────────────────────────┐
│ AMRN v5 fixed header (160 B)  │
├──────────────────────────────┤
│ code + data + relocation data │ signed bytes
├──────────────────────────────┤
│ DSIG envelope (88 B)          │ not signed
└──────────────────────────────┘
```

The first 128 bytes retain the v4 field offsets. Their format-version field is
`5`, their header-size field is `160`, and the relocation offset is adjusted
from the v3 linked image to the v5 payload origin. The extension fields are:

| Offset | Field | Size | Description |
| --- | --- | ---: | --- |
| `0x80` | signature_offset | 4 bytes | Little-endian; exact start of the DSIG trailer |
| `0x84` | signature_size | 2 bytes | Little-endian; value is `88` |
| `0x86` | reserved | 2 bytes | Must be zero |
| `0x88` | signed_size | 4 bytes | Little-endian; equal to `signature_offset` |
| `0x8C` | reserved | 4 bytes | Must be zero |
| `0x90` | reserved | 16 bytes | Must be zero |

The signed byte range is exactly `package[0..signature_offset]`. It includes
the final header and package CRC32, but excludes the DSIG trailer. The package
CRC32 is calculated over the signed range with its own four bytes treated as
zero, so CRC construction is not circular with signature construction. The
signature envelope is structurally validated by `dali-amrn`; cryptographic
verification and trust-anchor lookup remain separate responsibilities.

The host codec currently encodes and parses this contract with boundary tests;
its v5 API can also parse the fixed header and DSIG trailer separately for a
bounded streaming loader.
The CLI can now produce format 5 when the application manifest declares
`signing_key_id` and the private seed is supplied through the external
`DALI_SIGNING_KEY_HEX` environment variable. The feature-gated kernel loader
now has a bounded v5 path that authenticates the signed range before copying
or relocating the image, using static trust anchors from the selected target
profile. F405 hardware has verified the configured release-anchor path. This
is not Secure Boot and is not the completed multi-developer distribution
contract.
