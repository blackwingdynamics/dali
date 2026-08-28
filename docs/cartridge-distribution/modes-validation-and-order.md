# 11. Development, release, and recovery modes

Development mode MAY use an explicitly documented test anchor and unsigned
cartridges. It MUST be impossible to confuse development artifacts with release
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
normal application cartridge MUST NOT activate recovery mode or modify root
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
- unauthorized cartridge namespace or target;
- AMRN validation failure; and
- durable trust-store commit failure.

Logs MAY include role, key identifier, cartridge identity, version, and typed
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

1. valid metadata and cartridge acceptance;
2. unknown developer rejection;
3. revoked or expired developer rejection;
4. modified metadata rejection;
5. modified cartridge rejection;
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
8. connect cartridge installation to metadata authorization;
9. add host, target, and hardware acceptance evidence; and
10. only then publish a public multi-developer registry workflow.

No application scheduler, board backend, or cartridge loader feature may silently
implement part of this design under a different name. All changes must point
back to this document and the corresponding versioned contract.

The bounded root, timestamp, snapshot, targets, and developer-delegation
models now have canonical host-side encode/parse paths and contract validation
in `crates/dali-metadata`. This is codec and policy evidence only; it does not
prove repository bundle installation, durable trust-store updates, or target
hardware acceptance.
