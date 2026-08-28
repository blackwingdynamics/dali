# 5. Compatibility rules

The loader must validate at least:

- target architecture and MCU target;
- AMRN format version;
- application ABI version;
- load address and memory limits;
- application payload size;
- package integrity.

Compatibility is explicit. A package must not be loaded merely because its name or semantic version appears valid.

Future compatibility metadata may include:

```text
minimum_kernel_version
maximum_kernel_version
required_abi_version
required_services
memory_requirements
```

Target profiles permanently declare package authentication policy for both
development and release builds. The current F405 policy permits unsigned
development packages for local bring-up and requires Ed25519 for release
packages. Release manifests must select AMRN format `5`, declare a
`signing_key_id`, and provide the private seed through the external
`DALI_SIGNING_KEY_HEX` environment variable. Target manifests provision
verification public keys through `authentication.development_trust_anchors`
and `authentication.release_trust_anchors`; private key material must never be
placed in a target manifest or firmware source. The reference F405 development
anchor is the RFC8032 test vector and is enabled only by `abi-test-fixtures`;
the release manifest contains a generated public anchor and has hardware
verification evidence. This static target-profile mechanism is the precursor
to the multi-developer repository trust contract in
`docs/package-distribution/README.md`; it is not that contract's dynamic trust store.

AMRN format version `4` defines package identity and selection metadata. It
remains ABI v3-compatible: format v4 changes the container header and
compatibility checks, not the application calling convention, service gateway,
or MPU contract. The current feature-gated F405 loader supports format v4 for
one selected package; it does not yet provide multi-package execution.

AMRN format version `5` is the signed successor to format v4. It leaves the
v4 bytes unchanged, signs the fixed header and payload, and appends a DSIG
trailer. Format v5 is a host/CLI contract only until target-side verification
is implemented. The crypto crate now rejects unknown key identifiers before
verification; this still must not be described as Secure Boot until the target
trust store is provisioned and the kernel loader enforces it.
