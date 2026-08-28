# 5. F405 streaming profile

The generic metadata contract retains larger bounds for host and future
targets. The F405 profile uses a 512-byte I/O chunk and bounded verifier state:

- no complete metadata envelope is retained after its signature/hash pass;
- targets records are length-prefixed and processed one at a time;
- only the selected target, delegation, references, and revocation decision
  are retained;
- the AMRN cartridge is verified through a streamed header/payload pass;
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
AMRN cartridge streaming remain loader integration work.

Typed role-body streaming parsers now cover Root, Timestamp, Snapshot,
Delegation, and Revocation. Each parser uses a bounded queue and emits typed
fixed-capacity metadata after fragmented-input validation. Targets continues
to use the kernel's selective record parser because its `record_length` field
allows non-selected package records to be skipped without retaining them.
The complete repository chain and package discovery wiring remain in progress.

The `<4 KiB` figure is a measured F405 verifier-state budget, not a property
of binary encoding alone. It must be reported from the linked target image
and include scratch storage, parser state, and callback state.
