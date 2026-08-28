# 6. Metadata format contract

The repository metadata wire format MUST be canonical and deterministic. The
first implementation MUST select one canonical encoding and document it before
writing a parser. Equivalent values with multiple byte encodings MUST NOT be
accepted if that creates signature ambiguity.

The initial Dali profile MUST use canonical JSON compatible with the relevant
TUF canonicalization rules for repository metadata. The embedded trust-store
update MAY use a bounded binary envelope containing the canonical metadata
bytes, but the signed bytes and encoding MUST remain identical across host and
target implementations.

Every metadata document MUST include:

- a format identifier and metadata version;
- a role identifier;
- a key identifier for the signer;
- a signed expiration or freshness field where applicable;
- a monotonic version number;
- a canonical signed body; and
- an explicit signature list with algorithm and key identifier.

The parser MUST enforce maximum sizes for the document, signature list, key
list, cartridge list, namespace length, and string fields before parsing nested
content. Unknown fields MUST be rejected in security-critical metadata unless
the metadata version explicitly permits extension fields.

## 6.1 Hash and length rules

Repository metadata MUST record both the exact cartridge length and a
cryptographic hash of the complete AMRN file. The hash MUST cover the bytes as
stored and transported, including the AMRN header and signature trailer.

CRC32 remains the bounded corruption detector inside AMRN. It is not a trust
anchor, certificate, or authenticity mechanism.

### 6.2 Signature rules

The initial profile uses Ed25519 for developer and metadata signatures. The
algorithm identifier, key length, signature length, and signed range MUST be
named constants in implementation code and versioned in the contract.

The same key MUST NOT be reused across root, repository, developer, and
timestamp roles. Role separation limits the effect of one compromised key.

### 6.3 Initial metadata schemas

The initial profile uses one signed envelope for every metadata role. The
envelope contains a canonical `signed` object and a `signatures` array:

```json
{
  "signed": { "...role-specific fields...": "..." },
  "signatures": [
    { "key_id": "32 lowercase hex characters", "signature": "128 lowercase hex characters" }
  ]
}
```

The signature input is the canonical UTF-8 JSON serialization of the `signed`
object only. The envelope, whitespace, and signature array are not part of
that input. Profile version `1` fixes Ed25519 as the only signature algorithm;
the algorithm is selected by the profile and is intentionally not repeated as
a per-signature wire field. A verifier MUST reject an envelope with duplicate
signature key identifiers, an unknown role, or a signature over a different
serialization.

The common signed fields are:

```text
schema        string, exactly "dali.metadata.v1"
role          string, one of root | timestamp | snapshot | targets | delegation | revocation | recovery | bundle
version       unsigned 64-bit integer, strictly increasing for the role
expires       unsigned 64-bit Unix seconds, zero only when the target has no clock
```

The initial profile encodes all key identifiers, public keys, hashes, and
signatures as lowercase hexadecimal strings. It encodes versions, lengths,
limits, slot IDs, and timestamps as JSON integers. Floating-point values,
negative integers, `null`, and duplicate object keys are invalid.

### 6.2 Durable trust-store payload

The bytes stored in `DALI-ACT.BIN` and `DALI-CAN.BIN` are a signed Binary
Metadata v2 envelope with role `recovery`. Its body is a bounded
`TrustStorePayload` with this canonical field order:

```text
metadata header: role=recovery, version, expires
target profile: u16 length + UTF-8 bytes
file count: u16
file references, sorted by kind then identifier:
  kind: u8 (root, timestamp, snapshot, targets, revocation, delegation)
  identifier: u16 length + UTF-8 bytes
  exact file length: u32
  complete-file SHA-256: 32 bytes
```

The payload has a maximum of `MAX_TRUST_STORE_FILES` references and
`MAX_TRUST_STORE_BYTES` encoded body bytes. It MUST contain the five singleton
metadata roles and at least one delegation reference. Cartridge references,
duplicate references, zero lengths, zero digests, invalid UTF-8, and
non-canonical ordering are rejected before activation. The signed envelope
authenticates the payload; the payload itself is only a bounded state
description.

This schema stores references rather than private keys or mutable verifier
state. Recovery can therefore reconstruct the complete signed root,
delegation, revocation, freshness, and rotation inputs without coupling the
payload to FAT, SDIO, or an MCU.

The root signed body is:

```json
{
  "schema": "dali.metadata.v1",
  "role": "root",
  "version": 1,
  "expires": 0,
  "keys": [
    { "key_id": "...", "public_key": "...", "role": "root" }
  ],
  "roles": [
    { "name": "bundle", "key_ids": ["..."], "threshold": 1 },
    { "name": "delegation", "key_ids": ["..."], "threshold": 1 },
    { "name": "recovery", "key_ids": ["..."], "threshold": 2 },
    { "name": "revocation", "key_ids": ["..."], "threshold": 1 },
    { "name": "snapshot", "key_ids": ["..."], "threshold": 1 },
    { "name": "targets", "key_ids": ["..."], "threshold": 1 },
    { "name": "timestamp", "key_ids": ["..."], "threshold": 1 }
  ]
}
```

The targets signed body contains bounded cartridge records and developer
delegations:

```json
{
  "schema": "dali.metadata.v1",
  "role": "targets",
  "version": 1,
  "expires": 0,
  "delegations": ["developer-delegation-id"],
  "cartridges": [
    {
      "cartridge_id": "32 lowercase hex characters",
      "namespace": "developer/application",
      "developer_id": "developer-id",
      "developer_key_id": "...",
      "target_profile": "f405",
      "amrn_format": 5,
      "abi_version": 3,
      "cartridge_version": "0.1.0",
      "minimum_kernel_version": "0.1.0",
      "length": 1234,
      "sha256": "64 lowercase hex characters",
      "required_services": 1,
      "slot_id": 1
    }
  ]
}
```

The snapshot signed body contains exactly one hash-and-length reference for
each metadata file it binds:

```json
{
  "schema": "dali.metadata.v1",
  "role": "snapshot",
  "version": 1,
  "expires": 0,
  "metadata": [
    { "role": "targets", "version": 1, "length": 1234, "sha256": "..." },
    { "role": "revocation", "version": 1, "length": 128, "sha256": "..." },
    { "role": "delegation", "id": "developer-id", "version": 1, "length": 456, "sha256": "..." }
  ]
}
```

The revocation signed body contains explicit developer-key revocations:

```json
{
  "expires": 0,
  "revocations": [
    {
      "developer_id": "developer-id",
      "effective_version": 2,
      "issuer_key_id": "...",
      "key_id": "...",
      "reason": "compromised"
    }
  ],
  "role": "revocation",
  "schema": "dali.metadata.v1",
  "version": 1
}
```

The timestamp signed body contains one snapshot reference:

```json
{
  "schema": "dali.metadata.v1",
  "role": "timestamp",
  "version": 1,
  "expires": 0,
  "snapshot": { "version": 1, "length": 1234, "sha256": "..." }
}
```

A delegation signed body contains one developer key and its bounded scope:

```json
{
  "schema": "dali.metadata.v1",
  "role": "delegation",
  "version": 1,
  "expires": 0,
  "developer_id": "developer-id",
  "key_id": "...",
  "public_key": "...",
  "allowed_namespaces": ["developer/application"],
  "allowed_targets": ["f405"],
  "allowed_abis": [3],
  "not_before": 0,
  "not_after": 0
}
```

The `targets` record MUST reference the exact delegation and developer key
that authorize the cartridge. A cartridge is not authorized when its AMRN key ID,
namespace, target, ABI, or version disagrees with metadata.

### 6.4 Offline bundle manifest

An offline bundle manifest is a signed `bundle` role document. Its canonical
signed body contains exactly these top-level fields in lexicographic order:

```json
{
  "files": [
    { "id": "developer-id", "kind": "delegation", "length": 456, "sha256": "..." },
    { "id": "cartridge-sha256", "kind": "cartridge", "length": 1234, "sha256": "..." }
  ],
  "role": "bundle",
  "schema": "dali.metadata.v1",
  "target_profile": "f405",
  "version": 1
}
```

`kind` MUST be one of `delegation`, `cartridge`, `root`, `snapshot`,
`targets`, `revocation`, or `timestamp`. Fixed metadata files use their kind as the logical
identity; delegation files use the delegation identifier; cartridge files use
the lowercase SHA-256 filename stem. Each kind/id pair MUST be unique, every
length MUST be non-zero, and every digest MUST be non-zero. The canonical file
order is `root`, `timestamp`, `snapshot`, `targets`, `revocation`,
`delegation`, then `cartridge`; each repeated kind is ordered by its identifier.
A complete ordinary bundle MUST contain all seven kinds, and the manifest MUST
be verified before any file is installed. Recovery metadata is distributed by a
separate recovery procedure and is not required in an ordinary bundle.
