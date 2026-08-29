# 7. Cartridge acceptance flow

The target-side acceptance flow is fixed:

1. discover the candidate cartridge from the supported storage backend;
2. read bounded repository metadata and the signed trust-store state;
3. validate metadata structure, size, version, role, and expiration policy;
4. verify the root-to-role metadata chain;
5. verify snapshot references and every referenced metadata hash;
6. resolve every executable target record for the boot profile within the
   bounded execution capacity; zero records, duplicate identities, or records
   beyond the capacity MUST be rejected;
7. open `amrns/<lowercase-sha256>.amrn` from the content-addressed cartridge
   directory, never by directory order or an untrusted display name;
8. verify cartridge length and complete-file hash;
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

## 7.1 Implemented verification-chain boundary

The hardware-neutral `dali-metadata` crate now exposes the bounded
`verify_repository_cartridge` entry point. Its sequence is deliberately linear:

```text
Root
  -> Bundle manifest (signature and generation admission against DALI-CMT.BIN)
  -> Timestamp (snapshot length/version/SHA-256)
  -> Snapshot (targets, revocation, delegation references)
  -> Targets (cartridge record)
  -> Delegation (developer identity/key/scope)
  -> Revocation (effective repository generation)
  -> Cartridge record (complete-file length/SHA-256 and AMRN fields)
  -> AMRN v5 (CRC32 and developer Ed25519 signature)
```

Every metadata document is parsed from its canonical signed body and verified
with the root-declared role threshold before its references are consumed.
The signed `bundle.manifest` is streamed through the same Root-declared bundle
role policy before its generation is used. Active boot accepts the committed
generation; an explicitly authorized candidate must be strictly newer, and an
older or otherwise inadmissible generation is rejected before cartridge loading.
`Ed25519Verifier` delegates to the workspace `dali-crypto` facade; there is no
deterministic or test-only verifier in the production path. The chain is
bounded and borrows caller-owned bytes, so it is suitable for host tooling and
later target integration without introducing storage ownership or heap policy
into the metadata contract. The kernel installation path creates a
`CartridgeInstallationAuthorization` only after this chain succeeds, binds the
cartridge digest and length to the candidate generation, and checks that
generation against the active `DALI-CMT.BIN` counter before writing the
inactive slot. F405 production release acceptance remains separate hardware
evidence.

## 8. Trust-store update flow

Trust-store updates are separate from application cartridges. An application
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
crates/dali-boards/dali-board-stm32f405/src/sdio*.rs
  F405 SDIO mechanics and mapping to generic StorageError
```

The coordinator does not interpret FAT, SDIO registers, or board addresses.
It writes the inactive logical slot, flushes it, records a prepared journal
state, flushes again, and records the committed state. The journal CRC is an
integrity check only; repository authenticity remains the Ed25519 verification
chain defined above.
