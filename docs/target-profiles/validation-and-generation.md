# Validation rules

The target registry build rejects:

- missing or empty profile identity fields;
- duplicate profile names;
- duplicate non-zero AMRN target identifiers;
- zero contract identifiers on application-supported profiles;
- storage widths outside the supported four-line representation when storage is declared;
- missing or malformed TOML fields.

The generated registry is not committed. Rebuilding `dali-targets` regenerates
it from the manifests and tracks each manifest with Cargo rerun directives.
