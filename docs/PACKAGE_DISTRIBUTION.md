# Dali Package Distribution and Trust Contract

Status: frozen initial profile and source of truth for the multi-developer
package ecosystem.

Contract identifier: `dali.package-distribution.v1`

This document defines how a prebuilt Dali kernel can accept applications from
multiple independent developers without rebuilding the kernel for every new
developer. It is normative for the repository, CLI, package registry, trust
store, update agent, and target loader. Implementation must not begin by
guessing fields or behavior that are not defined here.

## 1. Goals

The distribution system must:

1. allow a user to install a prebuilt kernel and use it without a Rust toolchain;
2. allow each developer to create and retain a private signing key locally;
3. prevent a developer from receiving or sharing Dali's root private key;
4. authorize developers without rebuilding or reflashing the kernel for every
   new developer;
5. support offline devices that receive a bounded update bundle over storage;
6. detect modified, substituted, truncated, stale, and replayed metadata;
7. support developer-key rotation, expiry, revocation, and incident response;
8. preserve the existing AMRN package validation and memory-safety boundaries;
9. make repository and target metadata reproducible and reviewable; and
10. fail closed when trust, compatibility, or freshness cannot be established.

The design is TUF-like rather than a claim that Dali implements the complete
upstream TUF specification. Any deviation from this contract requires a
versioned contract change and new acceptance evidence.

## 2. Non-goals

This contract does not by itself provide:

- confidentiality or encryption of applications;
- a secure boot chain for the kernel image;
- protection from a compromised kernel or compromised root authority;
- protection from a physically invasive attacker with unrestricted debug access;
- automatic network access on a microcontroller;
- arbitrary native code safety without the separately documented ABI/MPU path;
- application scheduling, DMA isolation, or storage hot-plug behavior; or
- permission for an application to install another application.

Package authenticity and package authorization are separate from application
memory isolation. A correctly signed malicious application remains malicious
code unless the target's isolation contract contains it.

## 3. Normative language and invariants

The words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, and MAY are normative.

The following invariants are mandatory:

- Private keys MUST never be shipped in firmware, target manifests, AMRN
  packages, CI artifacts, issue reports, or logs.
- A public key MUST NOT become trusted merely because it appears in a package.
- Every accepted package MUST be authorized by the active target policy and a
  valid trust chain rooted in a kernel-provisioned Dali root key.
- Every signed byte range MUST be specified before a signer or verifier is
  implemented.
- Metadata MUST be bounded before parsing, allocation, copying, or execution.
- A failed signature, hash, version, expiry, role, or rollback check MUST
  reject the update without partially activating it.
- Trust-store replacement MUST be atomic: power loss may leave the previous
  valid store active, but never a partially written store.
- A package MUST be verified before relocation, MPU locking, application entry,
  or any application-owned state transition.
- A package hash and an AMRN signature are complementary controls; neither may
  be silently treated as a substitute for the other.

## 4. Threat model

### 4.1 Protected assets

The system protects:

- the Dali root of trust;
- developer private signing keys;
- target trust policy and revocation state;
- package identity, version, size, and hash metadata;
- the integrity of the installed application image; and
- the monotonic state used to prevent rollback where hardware supports it.

### 4.2 Threats in scope

The design MUST address:

- a modified package or metadata file;
- a package replaced by another valid package;
- an unknown developer key;
- a revoked or expired developer certificate;
- replay of an old metadata bundle;
- rollback to an older application version;
- truncation, duplication, and inconsistent repository metadata;
- compromise of one developer key;
- loss or planned rotation of one signing key;
- power loss during trust-store installation; and
- a registry mirror serving stale or incomplete data.

### 4.3 Threats outside this contract

The design does not solve compromise of the offline Dali root keys, a
compromised kernel binary, or an attacker able to bypass the MCU's debug and
flash protections. Production hardware must define debug-lock, root-key
custody, and recovery policy separately.

## 5. Trust hierarchy

The trust chain is:

```text
Dali root keys
    |
    +-- sign repository roles, developer delegations, and bundle manifests
            |
            +-- developer public key / certificate
                    |
                    +-- sign AMRN application package
```

The installed kernel contains only the Dali root public-key set and a target
policy identifier. It does not contain every developer key and it does not
contain any private key.

### 5.1 Root role

The root role is the highest authority. It MUST be kept offline or in an
approved hardware-backed signing system. Root metadata defines:

- the root key identifiers and public keys;
- the threshold required for root metadata updates;
- the allowed roles and their key identifiers;
- metadata format and version rules;
- supported target-policy identifiers; and
- root key expiry and rotation information.

The initial production configuration SHOULD use at least three independently
stored root keys with a two-of-three signing threshold. A single root key MAY
be used only for development and must be labelled non-production.

Root keys MUST NOT sign application packages directly. Their compromise is an
ecosystem incident and requires an emergency recovery procedure.

### 5.2 Targets role

The targets role authorizes package names, target profiles, developer
delegations, package hashes, package lengths, package versions, and package
compatibility metadata. It MUST be signed by a key authorized by root
metadata.

Targets metadata MUST contain enough information to reject a package before
loading it:

- package identity;
- target profile and AMRN format;
- ABI and minimum kernel version;
- package version;
- exact byte length;
- cryptographic hash of the complete AMRN artifact;
- developer key identifier or delegation identifier;
- expiry or repository freshness policy; and
- optional required-service and slot policy.

### 5.3 Snapshot role

The snapshot role binds an internally consistent version of the targets
metadata and delegated metadata. It prevents a client from combining a new
targets file with an old delegated file or vice versa.

Snapshot metadata MUST identify the version and hash of every metadata file it
covers. A client MUST reject a bundle whose referenced files do not match.

### 5.4 Timestamp role

The timestamp role gives clients a freshness boundary for snapshot metadata.
It SHOULD be short-lived and signed by a dedicated online or controlled
service key, not by a root key. Devices without a real-time clock MUST use the
target's declared freshness policy and monotonic update state; they MUST NOT
pretend that an unavailable wall clock proves freshness.

Timestamp metadata MUST NOT be the only anti-rollback mechanism on a device
without a trustworthy clock. Hardware monotonic counters or an equivalent
durable version policy are required for a production anti-rollback claim.

### 5.5 Developer delegation

Each developer receives a delegated identity containing:

- developer identifier;
- developer public key and key identifier;
- allowed package namespace or application ownership scope;
- allowed target profiles and ABI families;
- certificate validity interval;
- delegation version; and
- revocation status or revocation reference.

A developer delegation MUST be signed by an authorized targets/delegation key.
The developer private key signs application artifacts, never delegation
metadata. A developer MUST NOT be able to authorize another developer.

### 5.6 Revocation role

Revocation is an explicit signed metadata document. It is not inferred from a
missing delegation, a changed targets file, or a local deny-list. Each record
identifies a developer key, the developer identity, the repository version from
which the revocation applies, the issuing authority key, and a bounded reason.
The revocation document is signed by the root-authorized `revocation` role and
is referenced by snapshot metadata. A target MUST reject a package signed by a
revoked developer key when the package is evaluated at or after the record's
effective version.

The `recovery` role is also root-authorized, but it is reserved for separately
authorized recovery metadata and is not an ordinary repository bundle file.

### 5.7 Secure Boot admission contract

The Secure Boot contract identifier is `dali.secure-boot.v1`. It defines the
minimum admission checks for a kernel image; it does not select a bootloader,
flash controller, debug transport, or board-specific startup path.

The signed kernel-image descriptor is a fixed Binary v1 record:

```text
magic:           4 bytes, ASCII `DLKB`
descriptor_ver:  1 byte, value 1
target_length:   u16, little-endian
target_profile:  32 bytes, zero-padded UTF-8
image_version:   u64, little-endian, non-zero
image_length:    u32, little-endian, non-zero
image_sha256:    32 bytes, SHA-256 of the complete image
```

The descriptor is exactly 83 bytes and is signed by the root role. A target
MUST verify the root custody policy, root-role threshold, descriptor target,
exact image length, complete-image SHA-256, and root signature set before
entering the image. The image version MUST be strictly newer than the durable
trust-store version when rollback protection is enabled. Any failure MUST
halt admission before kernel execution.

Production custody requires at least three distinct root public keys and a
two-of-three root threshold. Their private counterparts MUST remain offline or
hardware-backed, independently held, and MUST never be provisioned to the
target. The target stores public verification material only.

`dali-metadata` now implements this bounded policy and descriptor verification
against caller-owned bytes. A target bootloader/ROM handoff and a production
kernel-image release pipeline remain separate integration and hardware-
acceptance work; this implementation must not be described as completed Secure
Boot evidence by itself.

## 6. Metadata format contract

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
list, package list, namespace length, and string fields before parsing nested
content. Unknown fields MUST be rejected in security-critical metadata unless
the metadata version explicitly permits extension fields.

### 6.1 Hash and length rules

Repository metadata MUST record both the exact package length and a
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
metadata roles and at least one delegation reference. Package references,
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

The targets signed body contains bounded package records and developer
delegations:

```json
{
  "schema": "dali.metadata.v1",
  "role": "targets",
  "version": 1,
  "expires": 0,
  "delegations": ["developer-delegation-id"],
  "packages": [
    {
      "package_id": "32 lowercase hex characters",
      "namespace": "developer/application",
      "developer_id": "developer-id",
      "developer_key_id": "...",
      "target_profile": "f405",
      "amrn_format": 5,
      "abi_version": 3,
      "package_version": "0.1.0",
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
that authorize the package. A package is not authorized when its AMRN key ID,
namespace, target, ABI, or version disagrees with metadata.

### 6.4 Offline bundle manifest

An offline bundle manifest is a signed `bundle` role document. Its canonical
signed body contains exactly these top-level fields in lexicographic order:

```json
{
  "files": [
    { "id": "developer-id", "kind": "delegation", "length": 456, "sha256": "..." },
    { "id": "package-sha256", "kind": "package", "length": 1234, "sha256": "..." }
  ],
  "role": "bundle",
  "schema": "dali.metadata.v1",
  "target_profile": "f405",
  "version": 1
}
```

`kind` MUST be one of `delegation`, `package`, `root`, `snapshot`,
`targets`, `revocation`, or `timestamp`. Fixed metadata files use their kind as the logical
identity; delegation files use the delegation identifier; package files use
the lowercase SHA-256 filename stem. Each kind/id pair MUST be unique, every
length MUST be non-zero, and every digest MUST be non-zero. The canonical file
order is `root`, `timestamp`, `snapshot`, `targets`, `revocation`,
`delegation`, then `package`; each repeated kind is ordered by its identifier.
A complete ordinary bundle MUST contain all seven kinds, and the manifest MUST
be verified before any file is installed. Recovery metadata is distributed by a
separate recovery procedure and is not required in an ordinary bundle.

## 7. Package acceptance flow

The target-side acceptance flow is fixed:

1. discover the candidate package from the supported storage backend;
2. read bounded repository metadata and the signed trust-store state;
3. validate metadata structure, size, version, role, and expiration policy;
4. verify the root-to-role metadata chain;
5. verify snapshot references and every referenced metadata hash;
6. resolve every executable target record for the boot profile within the
   bounded execution capacity; zero records, duplicate identities, or records
   beyond the capacity MUST be rejected;
7. open `packages/<lowercase-sha256>.amrn` from the content-addressed package
   directory, never by directory order or an untrusted display name;
8. verify package length and complete-file hash;
9. verify the developer delegation and its validity/revocation state;
10. verify the AMRN v5 developer signature over its specified signed range;
11. validate AMRN fields, CRC32, bounds, compatibility, services, and entry;
12. copy only after all checks pass;
13. apply relocation only within the declared code/data contract;
14. configure MPU and privilege state only after relocation is complete; and
15. enter the application only after the active lifecycle state permits it.

Any failure before step 12 MUST leave the previous valid application and trust
store unchanged. The loader MUST report a typed reason without exposing keys
or sensitive metadata.

### 7.1 Implemented verification-chain boundary

The hardware-neutral `dali-metadata` crate now exposes the bounded
`verify_repository_package` entry point. Its sequence is deliberately linear:

```text
Root
  -> Bundle manifest (signature and generation admission against DALI-CMT.BIN)
  -> Timestamp (snapshot length/version/SHA-256)
  -> Snapshot (targets, revocation, delegation references)
  -> Targets (package record)
  -> Delegation (developer identity/key/scope)
  -> Revocation (effective repository generation)
  -> Package record (complete-file length/SHA-256 and AMRN fields)
  -> AMRN v5 (CRC32 and developer Ed25519 signature)
```

Every metadata document is parsed from its canonical signed body and verified
with the root-declared role threshold before its references are consumed.
The signed `bundle.manifest` is streamed through the same Root-declared bundle
role policy before its generation is used. Active boot accepts the committed
generation; an explicitly authorized candidate must be strictly newer, and an
older or otherwise inadmissible generation is rejected before package loading.
`Ed25519Verifier` delegates to the workspace `dali-crypto` facade; there is no
deterministic or test-only verifier in the production path. The chain is
bounded and borrows caller-owned bytes, so it is suitable for host tooling and
later target integration without introducing storage ownership or heap policy
into the metadata contract. The kernel installation path creates a
`PackageInstallationAuthorization` only after this chain succeeds, binds the
package digest and length to the candidate generation, and checks that
generation against the active `DALI-CMT.BIN` counter before writing the
inactive slot. F405 production release acceptance remains separate hardware
evidence.

## 8. Trust-store update flow

Trust-store updates are separate from application packages. An application
MUST NOT be able to update the trust store.

The update flow is:

1. obtain a complete signed update bundle from the supported transport;
2. validate the bundle length before reading it into bounded storage;
3. verify root and role signatures using the kernel-provisioned root keys;
4. reject a bundle below the stored monotonic trust-store version;
5. validate target profile, key scope, validity interval, and revocations;
6. write the new bundle to the inactive durable slot;
7. verify the written bytes and metadata again;
8. atomically mark the new slot active; and
9. retain the previous valid slot for recovery until the next update is
   committed and verified.

Power loss at any point MUST recover either the old valid trust store or the
new fully verified trust store. It MUST NOT activate a partially written slot.

The kernel provides this flow through the board-agnostic persistence
coordinator and repository installation boundary. The existing
`release_trust_anchors` target manifest remains the single-image development
precursor and must not be presented as a multi-developer trust store. F405
production release acceptance of installation and recovery remains pending.

## 8.1 Authority-side delegation artifact generation

The CLI now provides a bounded authority-side constructor for one signed
developer delegation:

```text
dali metadata delegation create \
  --output <delegation-envelope.json> \
  --signing-key <authority-seed-file> \
  --signer-key-id <root-authority-key-id> \
  --developer-id <developer-id> \
  --developer-key-id <developer-key-id> \
  --developer-public-key <developer-public-key> \
  --namespace <exact-namespace> \
  --target <target-profile> \
  --abi <abi-version> \
  --version <delegation-version>
```

The command reads the authority seed locally, signs only the canonical
delegation body, and writes a canonical signed envelope with create-new
semantics. It never prints or embeds the seed. The authority seed MUST remain
offline or hardware-backed; this command is not a trust-store installer and
does not authorize a delegation on a target by itself.

The corresponding inspection command verifies the canonical body and
signature without requiring private material:

```text
dali metadata delegation inspect \
  --input <delegation-envelope.json> \
  --signer-public-key <authority-public-key>
```

The storage-independent trust-store state machine uses the following durable
order: stage the candidate, verify it with a final read-back, persist the
commit marker, then finalize the active-slot swap. Recovery discards a
candidate before the commit marker and completes the swap after the marker.
The state machine does not own filesystem or block-device I/O; those adapters
must persist each boundary using the target's durable-storage contract.

The kernel implementation keeps this boundary board-agnostic:

```text
storage/durable.rs
  BlockDevice + DurableStorageAdapter contracts
storage/durable/journal.rs
  104-byte DALI-CMT.BIN encoder/decoder and CRC32 validation
storage/durable/coordinator.rs
  bounded persistence transitions and adapter calls
platform/f405/sdio*.rs
  F405 SDIO mechanics and mapping to generic StorageError
```

The coordinator does not interpret FAT, SDIO registers, or board addresses.
It writes the inactive logical slot, flushes it, records a prepared journal
state, flushes again, and records the committed state. The journal CRC is an
integrity check only; repository authenticity remains the Ed25519 verification
chain defined above.

## 9. Key lifecycle

### 9.1 Developer key creation

The CLI MUST generate developer private keys using the host OS CSPRNG. It MUST
write restrictive permissions, refuse accidental overwrite, and never print
private material. The private key belongs to the developer and MUST be backed
up in approved secret storage.

### 9.2 Enrollment

Enrollment MUST authenticate the developer through an out-of-band process
before a delegation is issued. The registry MUST record who approved the
delegation, its scope, and its validity interval. Self-published public keys
are not trusted merely because they are available in a repository.

### 9.3 Rotation

Rotation MUST overlap old and new keys long enough to update supported devices.
The new key MUST be added before packages are signed with it. The old key MUST
be removed only after the migration window and acceptance evidence.

### 9.4 Revocation

Revocation metadata MUST identify the key or certificate, reason, effective
version, and issuing authority. A revoked developer key MUST prevent new
package installation while preserving an explicit policy for already-installed
packages.

### 9.5 Compromise and loss

If a developer private key is lost, existing packages remain verifiable, but
new packages cannot be signed with that key. The developer must enroll a new
key. If a key is suspected compromised, it MUST be revoked immediately and a
replacement package/trust update must be issued. The root-key incident
procedure is separate and requires root threshold recovery.

## 10. Repository and CLI responsibilities

The registry is responsible for:

- issuing and revoking developer delegations;
- generating canonical root, targets, snapshot, and timestamp metadata;
- enforcing namespace and target authorization;
- publishing complete, hash-addressed package artifacts; and
- preserving auditable metadata history.

The CLI is responsible for:

- generating local developer keys;
- creating signing requests without exposing private keys;
- building and signing AMRN packages;
- generating and inspecting metadata bundles;
- verifying packages and metadata before installation; and
- producing deterministic, reviewable release artifacts.

The CLI MUST NOT silently generate a new trust root, replace a key, bypass an
expired metadata role, or accept an unsigned production package. Commands that
change trust state MUST require explicit input and display the resulting key
identifier, version, and target scope.

### 10.1 Host bundle workflow

For the complete new-developer key, authorization, build, SD-card, and F405
acceptance procedure, see
[`docs/cli/BINARY_V2_DEVELOPER_WORKFLOW.md`](cli/BINARY_V2_DEVELOPER_WORKFLOW.md).

The host CLI exposes the first repository-bundle workflow:

```text
dali metadata bundle generate \
  --input <repository-root> \
  --output <repository-root>/bundle.manifest \
  --target-profile <profile> \
  --version <monotonic-version> \
  --signing-key <bundle-signing-seed> \
  --signer-key-id <bundle-signer-key-id>

dali metadata bundle inspect --input <repository-root>
dali metadata bundle verify \
  --input <repository-root> \
  --package-id <package-id-hex>
```

`generate` hashes the fixed metadata files, delegation documents, and
content-addressed `.amrn` packages, then writes a canonical signed bundle
manifest. It refuses to overwrite an existing manifest. `inspect` parses the
manifest and verifies every recorded length and SHA-256 reference before
printing the bundle inventory. `verify` performs those checks, verifies the
bundle role signature using the root trust policy, and then invokes the same
host-side Ed25519 chain used by the metadata crate:

```text
Root -> Timestamp -> Snapshot -> Targets -> Delegation -> Revocation
      -> Package Record -> AMRN hash/signature
```

The commands are host-side release and pre-install tooling. They do not prove
durable SD activation or kernel-loader integration; those remain separate
target and hardware acceptance tasks. Private signing seeds are read only from
the path supplied by the operator and are never included in the generated
bundle or command output.

The kernel is responsible for:

- storing the root public-key set;
- verifying bounded trust-store updates;
- enforcing the active trust policy;
- verifying package metadata and AMRN contents before copy/relocation; and
- failing closed on any unsupported role, algorithm, version, or scope.

The kernel is not responsible for developer account management or for
transporting arbitrary registry data without a bounded storage contract.

## 11. Development, release, and recovery modes

Development mode MAY use an explicitly documented test anchor and unsigned
packages. It MUST be impossible to confuse development artifacts with release
artifacts through names, metadata, or logs.

Release mode MUST require:

- AMRN format v5;
- a valid developer delegation;
- valid repository metadata;
- a supported target and ABI;
- an unexpired and non-revoked key;
- complete-file hash and length matches; and
- an accepted signature chain.

Recovery mode MUST accept only a separately authorized recovery bundle. A
normal application package MUST NOT activate recovery mode or modify root
trust. Recovery behavior, physical authorization, and anti-rollback policy
must be documented for each target family.

## 12. Failure and logging policy

The system MUST distinguish these failures:

- malformed metadata;
- unsupported metadata version or role;
- unknown root or delegated key;
- invalid signature;
- hash or length mismatch;
- expired metadata;
- revoked key or certificate;
- rollback or stale version;
- unauthorized package namespace or target;
- AMRN validation failure; and
- durable trust-store commit failure.

Logs MAY include role, key identifier, package identity, version, and typed
failure reason. Logs MUST NOT include private keys, signatures when not needed
for diagnosis, seed material, or complete metadata dumps from untrusted input.

## 13. Validation gates

No implementation milestone is complete without the matching evidence.

### Host and codec tests

Host tests MUST cover:

- canonical encoding stability;
- every role's accepted and rejected signature;
- unknown fields and unsupported versions;
- bounded document and list sizes;
- hash and length mismatch;
- expiry, revocation, and rollback;
- delegation scope and namespace rejection;
- duplicate identities and conflicting metadata;
- key rotation overlap and removal; and
- interrupted trust-store commit state transitions.

These tests prove codec and policy behavior only; they are not hardware
evidence.

### Target tests

The embedded target checks MUST prove:

- no allocation or unbounded parsing in the loader/update path;
- the selected crypto facade builds for the target;
- trust-store verification completes before application copy;
- trust-store writes use the declared storage ownership boundary; and
- every failure returns to a safe, deterministic lifecycle state.

### Hardware acceptance

Each supported board must demonstrate:

1. valid metadata and package acceptance;
2. unknown developer rejection;
3. revoked or expired developer rejection;
4. modified metadata rejection;
5. modified package rejection;
6. rollback rejection;
7. power-loss-safe trust-store replacement, where testable;
8. key rotation with old/new overlap; and
9. recovery after an interrupted or invalid update.

The current F405 release-anchor signature verification is evidence for the
single static-anchor path only. It is not evidence for this multi-developer
contract.

## 14. Implementation order

Implementation MUST follow this order:

1. freeze this document and version its contract identifier;
2. define canonical metadata schemas and bounded size constants;
3. implement a hardware-neutral metadata codec and policy validator;
4. implement root, role, delegation, hash, expiry, and revocation checks;
5. extend the CLI to generate requests and build signed metadata bundles;
6. define the offline bundle layout and atomic storage-update contract;
7. implement target trust-store verification and durable activation;
8. connect package installation to metadata authorization;
9. add host, target, and hardware acceptance evidence; and
10. only then publish a public multi-developer registry workflow.

No application scheduler, board backend, or package loader feature may silently
implement part of this design under a different name. All changes must point
back to this document and the corresponding versioned contract.

The bounded root, timestamp, snapshot, targets, and developer-delegation
models now have canonical host-side encode/parse paths and contract validation
in `crates/dali-metadata`. This is codec and policy evidence only; it does not
prove repository bundle installation, durable trust-store updates, or target
hardware acceptance.

## 15. Frozen initial-profile decisions

The following decisions are part of the initial profile. Implementation may
refine internal code structure, but it MUST NOT change these wire or security
rules without a contract revision.

### 15.1 Encoding and cryptography

- Repository metadata uses UTF-8 canonical JSON: no insignificant whitespace,
  UTF-8 object keys sorted lexicographically, deterministic number encoding,
  and no duplicate object keys.
- Ed25519 is the only accepted signature algorithm in profile version `1`.
- SHA-256 is the repository metadata and complete-package hash algorithm.
- CRC32 remains only the AMRN corruption detector.
- Signatures cover the canonical serialized body, never an ambiguous parsed
  representation.

### 15.2 Root and role custody

- Production root uses three independently held keys and a two-of-three
  threshold.
- Root keys are offline or hardware-backed and never live in CI variables used
  for ordinary package builds.
- Targets, snapshot, and timestamp use separate delegated keys.
- Revocation uses a separate delegated key and explicit signed metadata.
- Developer keys never sign root, snapshot, or timestamp metadata.
- Development may use one clearly labelled non-production root key.

### 15.3 Certificate and namespace rules

A developer delegation contains exactly these security fields. The common
`version` field is the delegation version; there is no separate certificate
version field in profile `v1`:

```text
developer_id
key_id
public_key
allowed_namespaces
allowed_targets
allowed_abis
not_before
not_after
version
```

`developer_id` is an opaque registry identifier. `key_id` is a random fixed
16-byte identifier. Namespaces use lowercase UTF-8 package names separated by
`/`; a delegation MUST name an exact namespace or an explicitly bounded
namespace prefix. Wildcard access to every package is forbidden for developer
delegations.

### 15.4 Bounded metadata limits

The first F405 profile uses named, target-owned limits:

| Document | Maximum encoded size | Maximum records |
| --- | ---: | ---: |
| root | 16 KiB | 16 keys and 16 roles |
| timestamp | 4 KiB | 1 snapshot reference |
| snapshot | 16 KiB | 64 metadata references |
| targets | 64 KiB | 256 package records |
| developer delegation | 4 KiB | 32 package scopes |
| revocation | 4 KiB | 64 revocation records |
| trust-store update bundle | 128 KiB | 64 developer delegations |

These values are named constants in the target policy, not raw literals in
parsers. A future target may declare larger limits through a versioned target
profile; it may not silently accept an unbounded document.

### 15.5 Offline bundle and durable storage

An offline update bundle contains these files at fixed logical names:

```text
metadata/root.json
metadata/timestamp.json
metadata/snapshot.json
metadata/targets.json
metadata/delegations/<delegation-id>.json
metadata/revocations.json
packages/<package-sha256>.amrn
bundle.manifest
```

`bundle.manifest` is signed by the authorized update role and records the
bundle version, target profile, file lengths, and SHA-256 hashes. The physical
filesystem and transport are not trust boundaries; the signed metadata is.

The target stores two trust-store slots, `active` and `candidate`, in a
target-declared persistent area. A candidate is never active until it has
passed full verification and a final read-back check. The active slot is the
fallback after power loss.

The initial FAT adapter uses the following root-level artifact names when a
write-capable block transport is selected:

```text
DALI-ACT.BIN   active trust-store bundle
DALI-CAN.BIN   candidate trust-store bundle
DALI-CMT.BIN   commit marker
```

These names are kernel-owned and are not application packages. The commit
marker contains only the bounded commit record defined by the installer; it
must never be inferred from a filename or directory order.

The kernel's FAT write boundary is intentionally separate from package
verification: `write_root_file` can create or truncate one bounded root-level
artifact and flush its directory entry, but it does not provide a transaction.
The trust-store installer must sequence candidate bytes, read-back verification,
and the commit marker above this primitive. No installer may activate a bundle
through a single file write.

### 15.5.1 Board-agnostic kernel storage contracts

The durable installer and repository loader MUST depend on logical storage
traits, never on SDIO, FAT, STM32, or board-specific types:

```rust
pub trait DurableStorageAdapter {
    type Error;

    fn read_artifact(
        &mut self,
        artifact: DurableArtifact,
        output: &mut [u8],
    ) -> Result<usize, Self::Error>;

    fn write_artifact(
        &mut self,
        artifact: DurableArtifact,
        contents: &[u8],
    ) -> Result<(), Self::Error>;

    fn flush(&mut self) -> Result<(), Self::Error>;
}

pub trait RepositoryStreamStorage {
    type Error;

    fn stream_metadata<F>(
        &mut self,
        document: RepositoryDocument<'_>,
        chunk: &mut [u8],
        consumer: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>;

    fn stream_package<F>(
        &mut self,
        digest: RepositoryPackageDigest,
        chunk: &mut [u8],
        consumer: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>;
}
```

`BlockDevice` is the lower-level fixed-block boundary. A board support package
implements it using its native controller. A filesystem adapter then
implements `DurableStorageAdapter` and `RepositoryStreamStorage` without exposing
the block controller to the installer or loader. The F405 path is therefore:

```text
STM32F405 SDIO -> F405 block adapter -> FAT adapter
             -> DurableStorageAdapter / RepositoryStreamStorage
             -> persistence coordinator / kernel loader
```

The loader receives metadata and AMRN bytes through `RepositoryStreamStorage`;
it MUST NOT open files, interpret FAT paths, or select packages by directory
order. The adapter's returned byte count MUST equal the bytes delivered to the
consumer, and the caller owns the chunk lifetime.

The kernel provides a feature-gated repository loader over these traits.
`load_binary_repository<S>` consumes the streaming contract through bounded
caller-owned buffers. It reads the five fixed roles, selects every executable
Targets record for the configured target profile within the execution capacity,
and verifies each selected delegation, revocation state, content-addressed
AMRN package, and shared Root -> Timestamp -> Snapshot -> Targets -> Delegation
-> Revocation -> Package Record -> AMRN chain. After successful verification,
`load_repository_package()` opens the same lowercase SHA-256 package path and
passes it to the existing slot/relocation execution loader. The F405 target has
a concrete `FatRepositoryStorage<D>` adapter injected at this boundary. The
feature is intentionally excluded from the default MVP build. The
feature-gated F405 path has signed-bundle boot evidence; physical acceptance
of durable installation and recovery remains pending. The old retained-slice
`RepositoryBuffers` API remains host-side
only and still requires 360,448 bytes versus the 32 KiB kernel/runtime region;
the feature-gated F405 boot path MUST use `load_repository_package()` instead.

### 15.5.2 DALI-CMT.BIN commit journal record

`DALI-CMT.BIN` is a kernel-owned binary journal. One record is exactly 104
bytes; the file reserves two records (208 bytes total) so a torn write leaves
at least one recoverable record. Multi-byte integers are little-endian.

| Offset | Size | Field | Rule |
| ---: | ---: | --- | --- |
| `0x00` | 4 | Magic | ASCII `DLCT` |
| `0x04` | 1 | Format version | `1` |
| `0x05` | 1 | State | `1=Prepared`, `2=Committed` |
| `0x06` | 1 | Active slot | `0=SlotA`, `1=SlotB` |
| `0x07` | 1 | Reserved | Must be zero |
| `0x08` | 8 | Journal sequence | Strictly increasing valid record sequence |
| `0x10` | 8 | Bundle version | Monotonic trust-store version |
| `0x18` | 4 | Bundle length | Exact candidate bundle byte length |
| `0x1C` | 4 | Reserved | Must be zero |
| `0x20` | 32 | Bundle SHA-256 | Digest of the complete bundle |
| `0x40` | 4 | CRC32 | CRC32 over bytes `0x00..0x3F` |
| `0x44` | 28 | Reserved/padding | Must be zero |

The journal is a storage-integrity and activation record, not an authenticity
mechanism. The bundle's signed metadata and AMRN signature remain mandatory.
On recovery, the kernel selects the highest-sequence record with valid magic,
version, reserved bytes, CRC32, slot, length, and digest fields. A `Prepared`
record leaves the previous active generation authoritative. A `Committed`
record selects the referenced fully verified slot, after which the coordinator
may write the next journal record and retire the older slot.

### 15.6 Time, freshness, and rollback

- `timestamp` expiry is enforced only when the target has a trustworthy wall
  clock; a missing RTC MUST NOT be treated as current time.
- Every accepted trust-store bundle has a strictly increasing durable version.
- Every package namespace has a strictly increasing accepted package version.
- A target without a hardware monotonic counter MUST label rollback protection
  as limited to its durable trust-store and package-version state.
- A production anti-rollback claim requires a hardware monotonic counter or an
  equivalent tamper-resistant durable counter.

### 15.7 Revocation behavior

Revocation prevents installation, restart, and replacement of newly evaluated
packages signed by the revoked key. A currently running package is not
forcibly interrupted solely because a later revocation arrives; it becomes
ineligible at its next lifecycle transition. Emergency recovery MAY override
this only through a separately authorized recovery bundle.

### 15.8 Registry and mirror behavior

The registry publishes immutable, hash-addressed package artifacts and signed
metadata. Mirrors MAY cache and serve the same bytes but MUST NOT rewrite
metadata. A client MUST accept a mirror response only after verifying the
root-to-target chain, referenced hashes, target scope, and freshness policy.

The first embedded implementation uses offline bundles. Network registry APIs,
authentication, account management, and mirror discovery are host-side
concerns and must not be required by the kernel.

### 15.9 Recovery authorization

Recovery bundles require a dedicated recovery role authorized by root metadata.
An application key, developer delegation, or ordinary targets key MUST NOT
authorize a root/trust-store recovery. Physical recovery procedures and debug
lock behavior remain target-specific, but the signature and role rules are
common.

Until this profile is implemented and hardware-accepted, the existing static
F405 trust-anchor path remains the only supported release authentication
mechanism.
