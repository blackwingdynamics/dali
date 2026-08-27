# 1. Goals

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
