# Release and External Review Readiness

Use this checklist before publishing a release or submitting Dali OS for
startup funding or external technical evaluation.

## Release gate

- [ ] Confirm the version, release tag, AMRN format, ABI, SDK, CLI, and target
      compatibility records.
- [ ] Run the documented host, embedded, documentation, and applicable
      hardware validations.
- [ ] Review the staged files, generated artifacts, security claims, and
      recorded limitations.
- [ ] Confirm that release notes include behavior changes, migration notes,
      unsupported modes, and known limitations.
- [ ] Confirm generated changelog output is produced by release tooling.

## External evaluation gate

- [ ] Start with the root README, documentation index, architecture, roadmap,
      testing, security, CLI, and SDK indexes.
- [ ] Confirm every major requirement maps to an owner, validation layer,
      evidence record, and limitation.
- [ ] Confirm incomplete F405 or other hardware validation is visibly marked
      `Unverified` or otherwise bounded.
- [ ] Confirm no credentials, private keys, local paths, or unreviewed
      experimental claims appear in the submitted documentation.
- [ ] Record the evaluated revision, validation date, and remaining risks.

## Handoff

The release or evaluation handoff must include the exact revision, validation
commands and results, hardware evidence boundary, compatibility notes, and
known limitations. This checklist does not approve a release by itself; the
owning maintainers must make that decision from the recorded evidence.
