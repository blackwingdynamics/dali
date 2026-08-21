# Dali Metadata Binary v2

Status: Design Frozen. The codec and parser implementation follows this
contract; changes require an explicit wire-format version or contract review.

## 1. Decision

Dali metadata v2 uses a bounded custom binary codec. Postcard and CBOR were
not selected because the repository needs stable role identifiers, canonical
signature bytes, explicit length bounds, selective target-record streaming,
and a format that can be audited without a general-purpose serializer.

JSON metadata v1 remains a host-side compatibility format. A v2 repository
MUST contain only binary v2 metadata documents. A bundle MUST NOT mix JSON v1
and binary v2 documents.

Binary encoding is little-endian. Integers are unsigned unless a field says
otherwise. Variable-length values are length-prefixed and are never NUL
terminated.

## 2. Signed envelope

Every binary metadata file has this envelope:

| Offset | Size | Field | Rule |
| ---: | ---: | --- | --- |
| `0x00` | 4 | Magic | ASCII `DMB2` |
| `0x04` | 1 | Format | `2` |
| `0x05` | 1 | Role | Frozen `MetadataRole` numeric value |
| `0x06` | 1 | Flags | Must be zero in v2 |
| `0x07` | 1 | Reserved | Must be zero |
| `0x08` | 4 | Body length | Exact body length in bytes |
| `0x0C` | 1 | Signature count | `1..=8` |
| `0x0D` | 3 | Reserved | Must be zero |
| `0x10` | N | Body | Canonical role body |
| `0x10 + N` | `80 * count` | Signatures | Sorted by key ID |

One signature record is 16 bytes of key ID followed by 64 bytes of Ed25519
signature. Signatures authenticate exactly the body bytes. The bounded
decoder validates the envelope, role, lengths, signature-record ordering, and
signature-set shape before policy decisions. Cryptographic signature
verification is a separate chain step.

## 3. Common body prefix

Every role body starts with:

| Offset | Size | Field |
| ---: | ---: | --- |
| `0x00` | 8 | Role version, little-endian |
| `0x08` | 8 | Expiry Unix time, zero means no trusted clock |

The role-specific body follows immediately. Canonical encoders emit fields in
the order defined below and reject trailing bytes.

## 4. Role bodies

Fixed binary values use their natural width. Text uses `u16 length` followed
by UTF-8 bytes. The decoder validates UTF-8, the declared bound, and the role
contract before exposing a borrowed slice.

Root body:

```text
common prefix
u8 key_count
key_count * (key_id[16], role:u8, public_key[32])
u8 role_count
role_count * (role:u8, threshold:u8, key_count:u8, key_ids[key_count * 16])
```

Timestamp body:

```text
common prefix
snapshot_version:u64
snapshot_length:u32
snapshot_sha256:[u8;32]
```

Snapshot body:

```text
common prefix
targets_reference(version:u64, length:u32, sha256:[u8;32])
revocations_reference(version:u64, length:u32, sha256:[u8;32])
u8 delegation_count
delegation_count * (id:text, version:u64, length:u32, sha256:[u8;32])
```

Targets body:

```text
common prefix
u16 delegation_count
delegation_count * delegation_id:text
u32 package_count
package_count * (record_length:u16, target_record)
```

Each target record retains the existing semantic fields, encoded in fixed
order. A record length lets the kernel skip non-selected records without
retaining them. The F405 loader parses and authorizes one selected record at a
time.

Delegation body:

```text
common prefix
developer_id:text
key_id:[u8;16]
public_key:[u8;32]
u8 namespace_count, namespace_count * namespace:text
u8 target_count, target_count * target_profile:text
u8 abi_count, abi_count * abi:u16
not_before:u64
not_after:u64
```

Revocation body:

```text
common prefix
u16 record_count
record_count * (key_id:[u8;16], revoked_at:u64, reason:text)
```

## 5. F405 streaming profile

The generic metadata contract retains larger bounds for host and future
targets. The F405 profile uses a 512-byte I/O chunk and bounded verifier state:

- no complete metadata envelope is retained after its signature/hash pass;
- targets records are length-prefixed and processed one at a time;
- only the selected target, delegation, references, and revocation decision
  are retained;
- the AMRN package is verified through a streamed header/payload pass;
- no heap allocation is permitted.

The shared `BinaryEnvelopeStreamParser` in `dali-metadata` implements the
envelope portion of this profile for root, timestamp, snapshot, targets,
delegation, and revocation. It accepts fragmented input, hashes the exact
body bytes, forwards body chunks to a role parser, and retains only the fixed
envelope header plus bounded signature records. `StreamingRoleVerifier`
consumes those chunks through a real SHA-256 accumulator and one Ed25519
stream verifier per signature; it does not retain the body or use a fake
verifier.

The cryptographic verifier is a second-pass primitive: Binary Metadata v2
places signatures after the body, so a storage adapter must first parse the
envelope and obtain its signature records, then replay the same body stream
into `StreamingRoleVerifier`. Root remains a trust-anchor bootstrap case and
requires a provisioned root anchor or an explicitly documented two-pass
policy; it must not be silently treated as self-trusted.

`verify_binary_role_envelope` now packages that parse-and-replay operation for
the typed Root, Timestamp, Snapshot, Delegation, and Revocation body parsers.
It compares the first-pass and replay SHA-256 digests and rejects malformed or
tampered bodies before returning typed metadata. This is a chain primitive;
storage orchestration, Targets selection, Root trust-anchor provisioning, and
AMRN package streaming remain loader integration work.

Typed role-body streaming parsers now cover Root, Timestamp, Snapshot,
Delegation, and Revocation. Each parser uses a bounded queue and emits typed
fixed-capacity metadata after fragmented-input validation. Targets continues
to use the kernel's selective record parser because its `record_length` field
allows non-selected package records to be skipped without retaining them.
The complete repository chain and package discovery wiring remain in progress.

The `<4 KiB` figure is a measured F405 verifier-state budget, not a property
of binary encoding alone. It must be reported from the linked target image
and include scratch storage, parser state, and callback state.

## 6. Migration and compatibility

1. CLI emits a Binary v2 bundle manifest and resolves referenced metadata
   files with the `.dmb` extension when `--metadata-format binary-v2` is
   selected. The role-file generators are still a separate CLI milestone.
2. JSON v1 remains available for host inspection and migration tooling.
3. The current `bundle.manifest` body has no standalone format field; the
   selected CLI format controls the referenced file suffix and parser. Adding
   an explicit manifest format field requires a versioned contract change.
4. The kernel currently has a bounded selective targets parser, but the full
   Binary v2 verification chain is not yet wired into `load_repository()` or
   hardware-tested. JSON v1 remains the active repository-loader path.
5. Rollback protection compares the binary bundle version and digest through
   the existing durable coordinator.
6. JSON generation may be removed only after host workflows have migrated and
   the compatibility policy is updated.

No signature is reused across JSON and binary representations. Re-encoding a
role requires signing the canonical binary body again.
