# Validation and generation

From the repository root:

```text
cargo check -p dali-targets
cargo test -p dali-targets -p dali-cli
dali target list
dali target scaffold f405 --output <existing-directory>
```

The scaffold command creates a `.rs.template` and a board review document. It
renders the manifest values as named constants, but it does not register or
compile a backend automatically. Review the generated mapping before adding a
module to the platform facade.

The kernel linker-generation step reads `DALI_TARGET_PROFILE` when it is set.
If exactly one application-supported target exists, it may be omitted; if
multiple supported targets exist, the build fails until the profile is set
explicitly.
