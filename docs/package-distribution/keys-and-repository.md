# 9. Key lifecycle

## 9.1 Developer key creation

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
