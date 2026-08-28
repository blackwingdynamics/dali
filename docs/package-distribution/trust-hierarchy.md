# 5. Trust hierarchy

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

## 5.1 Root role

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
