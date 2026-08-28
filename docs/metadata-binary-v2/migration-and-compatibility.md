# 6. Migration and compatibility

1. CLI emits a Binary v2 bundle manifest and resolves referenced metadata
   files with the `.dmb` extension when `--metadata-format binary-v2` is
   selected. The role-file generators are still a separate CLI milestone.
2. JSON v1 remains available for host inspection and migration tooling.
3. The current `bundle.manifest` body has no standalone format field; the
   selected CLI format controls the referenced file suffix and parser. Adding
   an explicit manifest format field requires a versioned contract change.
4. The feature-gated kernel has a bounded selective targets parser and the
   full Binary v2 verification chain is wired into `load_repository()` through
   `load_binary_repository()`. F405 development-profile signed-bundle boot is
   hardware-tested. JSON v1 remains available for host compatibility and is
   not the active repository-loader format when `repository-loader` is enabled.
5. Rollback protection compares the binary bundle version and digest through
   the existing durable coordinator.
6. JSON generation may be removed only after host workflows have migrated and
   the compatibility policy is updated.

No signature is reused across JSON and binary representations. Re-encoding a
role requires signing the canonical binary body again.
