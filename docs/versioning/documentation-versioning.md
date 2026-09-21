# Documentation Versioning and Compatibility

Documentation is versioned with the contract it describes. A document must
identify the current scope, compatibility boundary, and limitations rather
than silently describing a future or feature-gated behavior as current.

## Ownership and updates

The domain README is the navigation owner. Technical source files, target
manifests, generated profiles, tests, and recorded Silicon Trace remain the
authoritative sources for implementation facts. Update the owning document in
the same change as a public behavior, contract, evidence, or security-claim
change.

## Compatibility levels

Review documentation changes against these layers:

- CLI and SDK semantic versions;
- AMRN format and application ABI versions;
- target profile and declared capabilities;
- kernel and hardware acceptance scope.

A documentation-only wording change must not alter the meaning of a versioned
contract. A changed command, field, ABI rule, target capability, or security
claim requires the corresponding compatibility and migration review.

## Status and migration rules

Use the documented status vocabulary: `Supported`, `Unverified`, `Not
applicable`, or `Deferred`. Do not use a successful build or host test to
replace missing hardware evidence. Breaking behavior requires migration notes;
unsupported behavior requires an explicit boundary and recovery guidance.

Generated changelogs are derived from commit history and must not be edited
manually. The release archive range begins at the most recent release that has
an archived changelog, so a tag without a published archive cannot hide the
commits since the previous archived release. Release tags and compatibility
decisions follow the [release tag policy](release-tags-and-breaking-changes.md).
