# 1. Decision

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
