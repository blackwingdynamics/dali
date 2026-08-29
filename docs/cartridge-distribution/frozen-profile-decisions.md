# 15. Frozen initial-profile decisions

The following decisions are part of the initial profile. Implementation may
refine internal code structure, but it MUST NOT change these wire or security
rules without a contract revision.

## 15.1 Encoding and cryptography

- Repository metadata uses UTF-8 canonical JSON: no insignificant whitespace,
  UTF-8 object keys sorted lexicographically, deterministic number encoding,
  and no duplicate object keys.
- Ed25519 is the only accepted signature algorithm in profile version `1`.
- SHA-256 is the repository metadata and complete-cartridge hash algorithm.
- CRC32 remains only the AMRN corruption detector.
- Signatures cover the canonical serialized body, never an ambiguous parsed
  representation.

### 15.2 Root and role custody

- Production root uses three independently held keys and a two-of-three
  threshold.
- Root keys are offline or hardware-backed and never live in CI variables used
  for ordinary cartridge builds.
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
16-byte identifier. Namespaces use lowercase UTF-8 cartridge names separated by
`/`; a delegation MUST name an exact namespace or an explicitly bounded
namespace prefix. Wildcard access to every cartridge is forbidden for developer
delegations.

### 15.4 Bounded metadata limits

The first F405 profile uses named, target-owned limits:

| Document | Maximum encoded size | Maximum records |
| --- | ---: | ---: |
| root | 16 KiB | 16 keys and 16 roles |
| timestamp | 4 KiB | 1 snapshot reference |
| snapshot | 16 KiB | 64 metadata references |
| targets | 64 KiB | 256 cartridge records |
| developer delegation | 4 KiB | 32 cartridge scopes |
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
amrns/<cartridge-sha256>.amrn
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

These names are kernel-owned and are not application cartridges. The commit
marker contains only the bounded commit record defined by the installer; it
must never be inferred from a filename or directory order.

The kernel's FAT write boundary is intentionally separate from cartridge
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

    fn stream_cartridge<F>(
        &mut self,
        digest: RepositoryCartridgeDigest,
        chunk: &mut [u8],
        consumer: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>;
}
```

`BlockDevice` is the lower-level fixed-block boundary. A board support cartridge
implements it using its native controller. A filesystem adapter then
implements `DurableStorageAdapter` and `RepositoryStreamStorage` without exposing
the block controller to the installer or loader. The F405 path is therefore:

```text
STM32F405 SDIO -> F405 block adapter -> FAT adapter
             -> DurableStorageAdapter / RepositoryStreamStorage
             -> persistence coordinator / kernel loader
```

The loader receives metadata and AMRN bytes through `RepositoryStreamStorage`;
it MUST NOT open files, interpret FAT paths, or select cartridges by directory
order. The adapter's returned byte count MUST equal the bytes delivered to the
consumer, and the caller owns the chunk lifetime.

The kernel provides a feature-gated repository loader over these traits.
`load_binary_repository<S>` consumes the streaming contract through bounded
caller-owned buffers. It reads the five fixed roles, selects every executable
Targets record for the configured target profile within the execution capacity,
and verifies each selected delegation, revocation state, content-addressed
AMRN cartridge, and shared Root -> Timestamp -> Snapshot -> Targets -> Delegation
-> Revocation -> Cartridge Record -> AMRN chain. After successful verification,
`load_repository_cartridge()` opens the same lowercase SHA-256 cartridge path and
passes it to the existing slot/relocation execution loader. The F405 target has
a concrete `FatRepositoryStorage<D>` adapter injected at this boundary. The
feature is intentionally excluded from the default MVP build. The
feature-gated F405 path has signed-bundle boot evidence; physical acceptance
of durable installation and recovery remains pending. The old retained-slice
`RepositoryBuffers` API remains host-side
only and still requires 360,448 bytes versus the 32 KiB kernel/runtime region;
the feature-gated F405 boot path MUST use `load_repository_cartridge()` instead.

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
- Every cartridge namespace has a strictly increasing accepted cartridge version.
- A target without a hardware monotonic counter MUST label rollback protection
  as limited to its durable trust-store and cartridge-version state.
- A production anti-rollback claim requires a hardware monotonic counter or an
  equivalent tamper-resistant durable counter.

### 15.7 Revocation behavior

Revocation prevents installation, restart, and replacement of newly evaluated
cartridges signed by the revoked key. A currently running cartridge is not
forcibly interrupted solely because a later revocation arrives; it becomes
ineligible at its next lifecycle transition. Emergency recovery MAY override
this only through a separately authorized recovery bundle.

### 15.8 Registry and mirror behavior

The registry publishes immutable, hash-addressed cartridge artifacts and signed
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
