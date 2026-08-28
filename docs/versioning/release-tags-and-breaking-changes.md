# 6. Release tags

Release tags use the component name and semantic version:

```text
kernel-v0.1.0
sdk-v0.1.0
cli-v0.1.0
amrn-format-v1
```

The current monorepo release workflow uses tags in the form `vX.Y.Z` or
`vX.Y.Z-alpha.N`. Component-specific tags may be introduced after the
repository is split.

For a monorepo release, the release notes must clearly state which components changed and which contracts remain compatible.

## 7. Breaking changes

A breaking change requires:

1. an update to the affected specification;
2. an update to the compatibility rules;
3. migration notes;
4. tests for accepted and rejected versions;
5. a roadmap task or release note;
6. an explicit version increment.

Do not silently change the meaning of an existing AMRN field, ABI rule, memory address, or public SDK item.
